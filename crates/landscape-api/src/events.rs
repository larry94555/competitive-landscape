//! Streaming a report while it is being written.
//!
//! `PRODUCT_SPEC.md` §2.1A: the first pass should reach a reader in twenty to forty seconds,
//! not at the end of ninety. A report is assembled a section at a time, so the wait is only
//! unavoidable if nobody sends anything until it is over.
//!
//! # Why this polls the database
//!
//! The worker and the API are two processes that share one thing: the row. A message broker
//! between them would carry events a few hundred milliseconds sooner and would be **a second
//! piece of infrastructure to run, supervise and lose events through** — on a box whose whole
//! design is three resident models and no spare memory
//! ([ADR 0005](../../../docs/decisions/0005-observability-on-a-24gb-box.md)).
//!
//! The queue already lives in the database for the same reason. So this reads the row on a
//! short interval and sends what changed. It is not the fastest possible design; it is the one
//! with the fewest things that can fail, and the reader cannot tell the difference.
//!
//! # What it sends
//!
//! ```text
//! event: section     one section that now has claims, as JSON
//! event: status      queued -> running -> complete | failed
//! event: progress    what the run is doing, and how far through it is
//! event: done        nothing else is coming
//! ```
//!
//! **`progress` is sent for a running analysis that has written nothing yet**, which no other
//! event here is. The rest describe a report; this one describes a *run*, and the longest
//! stretch with no report at all - working out which companies a description means, then
//! finding their pages - is exactly the stretch a reader most needs to be told is not a hang.
//! So it is synthesized from the status when the row carries no report, and read off the
//! report once there is one.
//!
//! **A section is sent when it first has something in it, and again whenever it changes.**
//! Watching a real run in a browser is what settled that: sending it once put *"What it does:
//! 1 item"* on screen and left it there for two minutes while eight more capabilities were
//! read. A section that arrives and then freezes reads as a section that is finished.
//!
//! *Changes*, not *grows*. A section can be corrected without getting longer:
//! `PagePricing::assembled` replaces a plan when a later window supplies the price the first
//! one lacked, so `Free is listed with no published price` becomes `Free costs $0` at the same
//! length. Comparing lengths would leave the retracted claim on screen until the run ended —
//! which is the one thing this product cannot do.

use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::{Path, State};
use axum::response::sse::{Event, KeepAlive, Sse};
use futures_util::stream::Stream;
use landscape_core::{AnalysisId, AnalysisStatus, Section, SectionStatus};

use crate::error::ApiError;
use crate::routes::AppState;

/// How often the row is read.
///
/// Half a second is well inside what a person reads as "live" and is 180 queries over a
/// 90-second analysis — against a database that is already handling one row per analysis.
const POLL: Duration = Duration::from_millis(500);

/// A stream that never ends is a resource leak wearing a feature's clothes.
///
/// Ten minutes is far longer than any analysis should take and far shorter than a browser tab
/// left open overnight. On expiry the stream ends with `done`, and the client can re-open it.
const MAX_STREAM: Duration = Duration::from_secs(600);

/// `GET /api/analyses/{id}/events` — server-sent events for one analysis.
pub(crate) async fn stream(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, ApiError> {
    // Same reading as `get_analysis`: an unparseable id is "not found" rather than "bad
    // request", because a mistyped reference and a deleted one are the same situation from
    // the reader's side.
    let id = id
        .parse::<uuid::Uuid>()
        .map(AnalysisId)
        .map_err(|_| ApiError::NotFound)?;

    // Resolved before the stream opens, so an unknown id is a 404 rather than a connection
    // that opens and immediately says nothing.
    state.store.get(id).await?;

    let store = Arc::clone(&state.store);
    let stream = async_stream::stream! {
        let started = std::time::Instant::now();
        // key -> exactly what the reader was last sent, so any change is resent and an
        // unchanged section is not. The payload rather than a count: a corrected claim is
        // the same length as the wrong one it replaces.
        let mut sent_sections: HashMap<String, String> = HashMap::new();
        let mut sent_status: Option<AnalysisStatus> = None;
        let mut sent_generation: Option<u32> = None;
        let mut sent_subjects: Option<String> = None;
        let mut sent_progress: Option<String> = None;

        loop {
            let Ok(analysis) = store.get(id).await else {
                // The row went away mid-stream. Nothing further is coming, and saying so is
                // better than holding a connection open over it.
                yield Ok(done());
                return;
            };

            if sent_status != Some(analysis.status) {
                sent_status = Some(analysis.status);
                yield Ok(status_event(analysis.status));
            }

            // **Which run these sections belong to.** A reader watching one analysis can be
            // watching two runs of it: a worker dies, the sweep hands the row on, and the
            // replacement starts from nothing. The sections already delivered belong to a run
            // that no longer exists, and the reader has to be told.
            //
            // Sending the number and letting the client compare is what makes this work
            // **across a reconnect**, which is where two earlier versions of this failed. A
            // server-side condition — "the report went away", "this connection has sent
            // something" — is about the connection, and a fresh connection remembers neither.
            // The reader's own state is the durable thing, so the comparison belongs there.
            if sent_generation != Some(analysis.generation) {
                // Only *after* the first one: on a new connection this says nothing about
                // whether the reader is holding anything, and the client decides that.
                if sent_generation.is_some() {
                    // Otherwise a replacement reaching the same answer is suppressed as a
                    // duplicate, and a reader whose screen was just cleared sits in front of
                    // an empty section for the whole second run.
                    sent_sections.clear();
                }
                sent_generation = Some(analysis.generation);
                yield Ok(generation_event(analysis.generation));
            }

            // **Before the report, and from the report once there is one.** A running analysis
            // with no report has still started: it is discovering, and saying so is the whole
            // difference between a page that looks busy and a page that looks broken.
            if analysis.status == AnalysisStatus::Running {
                let reached = analysis
                    .report
                    .as_ref()
                    .and_then(|r| r.progress)
                    // No report yet means no plan and no company count - the honest reading is
                    // "started, and nothing countable yet", which is what `starting(0)` is.
                    .unwrap_or(landscape_core::Progress::starting(0));
                let payload = progress_payload(&reached);
                if sent_progress.as_ref() != Some(&payload) {
                    sent_progress = Some(payload.clone());
                    yield Ok(Event::default().event("progress").data(payload));
                }
            }

            if let Some(report) = &analysis.report {
                // **Which companies this run set out to cover**, sent before the sections that
                // depend on it. A claim says "Pro costs $15" and never names the company, so a
                // reader watching a two-company run needs to know it *is* a two-company run
                // before the first claim lands - and the report that would tell them is not
                // fetched until the run is over. Deriving it from the claims on screen is the
                // defect this feature was opened to remove, arriving one surface later.
                if !report.subjects.is_empty() {
                    let payload = subjects_payload(&report.subjects);
                    if sent_subjects.as_ref() != Some(&payload) {
                        sent_subjects = Some(payload.clone());
                        yield Ok(Event::default().event("subjects").data(payload));
                    }
                }

                for section in &report.sections {
                    if section.status == SectionStatus::NotFoundInPublicSources
                        && section.claims.is_empty()
                    {
                        // Nothing in it yet. Its coverage note is part of the finished
                        // report, and sending it now would be sending a "we found nothing"
                        // for a question still being read.
                        continue;
                    }
                    let payload = payload_of(section);
                    if sent_sections.get(&section.key) != Some(&payload) {
                        sent_sections.insert(section.key.clone(), payload.clone());
                        yield Ok(Event::default().event("section").data(payload));
                    }
                }
            }

            if matches!(
                analysis.status,
                AnalysisStatus::Complete | AnalysisStatus::Failed
            ) {
                yield Ok(done());
                return;
            }
            if started.elapsed() > MAX_STREAM {
                yield Ok(done());
                return;
            }
            tokio::time::sleep(POLL).await;
        }
    };

    // The keep-alive is not decoration: proxies close idle connections, and an analysis can
    // spend thirty seconds fetching before it has anything to say.
    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}

fn status_event(status: AnalysisStatus) -> Event {
    Event::default().event("status").data(status.as_db_str())
}

/// What a reader is sent for one section, and what is compared against next time.
///
/// A section that will not serialize is a bug in the report type rather than something a
/// reader can act on, so the stream sends its key and carries on.
fn payload_of(section: &Section) -> String {
    serde_json::to_string(section).unwrap_or_else(|_| section.key.clone())
}

/// The companies being compared, as a JSON array.
///
/// A list that will not serialize is a bug in the report type rather than something a reader can
/// act on; an empty array is read by the client as "one company", which is the safe reading —
/// the label is left off rather than put on the wrong thing.
fn subjects_payload(subjects: &[String]) -> String {
    serde_json::to_string(subjects).unwrap_or_else(|_| "[]".to_owned())
}

/// What a run is doing, and how far through it is.
///
/// **The percentage is computed here rather than in the browser**, so the one place that decides
/// what a fraction means is `landscape_core::progress` and not two implementations of the same
/// arithmetic that can disagree. `percent` is `null` when there is nothing countable yet, and a
/// client that receives `null` is being told there is no number - not told to invent one.
fn progress_payload(reached: &landscape_core::Progress) -> String {
    serde_json::json!({
        "phase": reached.phase,
        "saying": reached.phase.wording(),
        "percent": reached.percent(),
        // **Where the browser's estimate must stop.** It interpolates elapsed time across
        // discovery, and this is the percentage the first counted tick will land on - computed
        // by `landscape_core::progress`, which owns the arithmetic, rather than by a constant
        // copied into TypeScript that would drift away from it. `null` once counting has begun.
        "estimating_to": reached.estimating_to(),
        "companies": reached.companies,
        "pages": reached.pages,
    })
    .to_string()
}

fn done() -> Event {
    Event::default().event("done").data("")
}

/// Which run the sections that follow belong to.
///
/// Not `done`, which would mean the opposite — a reader told a run finished does not
/// reconnect. A change in this number says: keep watching, and forget what you have.
fn generation_event(generation: u32) -> Event {
    Event::default()
        .event("generation")
        .data(generation.to_string())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use landscape_core::{Claim, Confidence, Report};

    fn at() -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::parse_from_rfc3339("2026-08-04T09:00:00Z")
            .map(|d| d.with_timezone(&chrono::Utc))
            .unwrap_or_default()
    }

    #[test]
    fn the_wire_carries_where_the_estimate_must_stop() {
        // One copy of the constant, in the language that owns the arithmetic. The browser is
        // told the ceiling rather than computing it, so the estimate and the count meet.
        let discovering = landscape_core::Progress::starting(2);
        let payload = progress_payload(&discovering);
        let read: serde_json::Value = serde_json::from_str(&payload).expect("valid JSON");
        assert_eq!(
            read["estimating_to"],
            serde_json::json!(discovering.estimating_to())
        );
        assert!(
            !read["estimating_to"].is_null(),
            "a discovering run has a ceiling"
        );

        let reading = landscape_core::Progress {
            phase: landscape_core::Phase::Reading,
            companies: landscape_core::Counted::new(0, 2),
            pages: Some(landscape_core::Counted::new(1, 4)),
        };
        let counting: serde_json::Value =
            serde_json::from_str(&progress_payload(&reading)).expect("valid JSON");
        assert!(
            counting["estimating_to"].is_null(),
            "nothing is left to estimate once something is counted"
        );
    }

    #[test]
    fn a_run_with_nothing_countable_yet_sends_no_number_rather_than_zero() {
        // The window this event exists for: running, no report, nothing knows how big the job
        // is. `null` says so. A `0` here would be a claim that no work has been done, and the
        // reader would watch it sit there through the longest silent stretch of the run.
        let payload = progress_payload(&landscape_core::Progress::starting(0));
        let read: serde_json::Value = serde_json::from_str(&payload).expect("valid JSON");
        assert_eq!(read["percent"], serde_json::Value::Null);
        assert_eq!(read["saying"], "Finding the pages worth reading");
        assert_eq!(read["pages"], serde_json::Value::Null);
    }

    #[test]
    fn the_percentage_on_the_wire_is_the_one_the_core_computes() {
        // **One implementation of the arithmetic.** Two would be a fact derived in two places,
        // which is the shape this register has an entry for.
        let reached = landscape_core::Progress {
            phase: landscape_core::Phase::Reading,
            companies: landscape_core::Counted::new(1, 2),
            pages: Some(landscape_core::Counted::new(2, 4)),
        };
        let payload = progress_payload(&reached);
        let read: serde_json::Value = serde_json::from_str(&payload).expect("valid JSON");
        assert_eq!(read["percent"], serde_json::json!(reached.percent()));
        // One of two companies, plus the second's finished discovery and half of its pages.
        assert_eq!(read["percent"], 79);
        assert_eq!(read["pages"]["done"], 2);
        assert_eq!(read["pages"]["of"], 4);
    }

    fn section(key: &str, claims: Vec<Claim>) -> Section {
        Section {
            key: key.to_owned(),
            title: "Pricing & packaging".to_owned(),
            status: if claims.is_empty() {
                SectionStatus::NotFoundInPublicSources
            } else {
                SectionStatus::Populated
            },
            claims,
            checked: vec!["/pricing (404)".to_owned()],
            notes: Vec::new(),
        }
    }

    fn claim() -> Claim {
        Claim {
            subject: String::new(),
            text: "Pro costs $15".to_owned(),
            source_label: "S1".to_owned(),
            evidence_quote: "$15/user".to_owned(),
            confidence: Confidence::High,
            as_of: at(),
        }
    }

    #[test]
    fn a_populated_section_serializes_into_its_payload() {
        let payload = payload_of(&section("pricing", vec![claim()]));
        assert!(payload.contains("Pro costs $15"), "{payload}");
    }

    #[test]
    fn a_corrected_claim_is_a_different_payload_at_the_same_length() {
        // The case a claim count cannot see. `PagePricing::assembled` replaces a plan when a
        // later window supplies the price the first one lacked, so one claim becomes a
        // different one claim — and the retracted version would otherwise sit on the
        // reader's screen until the run ended.
        let vague = Claim {
            text: "Free is listed with no published price".to_owned(),
            ..claim()
        };
        let corrected = Claim {
            text: "Free costs $0".to_owned(),
            ..claim()
        };
        let before = payload_of(&section("pricing", vec![vague]));
        let after = payload_of(&section("pricing", vec![corrected]));
        assert_ne!(before, after, "a correction must be visible to the stream");
    }

    #[test]
    fn an_unchanged_section_has_an_unchanged_payload() {
        // The other half: polling twice a second must not resend what nobody has changed.
        let once = payload_of(&section("pricing", vec![claim()]));
        let twice = payload_of(&section("pricing", vec![claim()]));
        assert_eq!(once, twice);
    }

    #[test]
    fn an_empty_section_is_recognizable_as_not_ready() {
        // The condition the stream filters on. A section with no claims and a "not found"
        // status is a question still being read, and sending it would tell a reader we had
        // finished looking.
        let empty = section("changes", Vec::new());
        assert!(empty.claims.is_empty());
        assert_eq!(empty.status, SectionStatus::NotFoundInPublicSources);
    }

    #[test]
    fn a_status_event_carries_the_same_word_the_database_uses() {
        // One vocabulary. A reader debugging a stuck analysis compares the stream against
        // the row, and two spellings of "running" would waste an hour.
        let event = status_event(AnalysisStatus::Running);
        assert!(format!("{event:?}").contains("running"));
    }

    #[test]
    fn a_report_with_every_section_empty_sends_none_of_them() {
        let report = Report {
            subject: "https://e.com".to_owned(),
            searched_as: "https://e.com".to_owned(),
            generated_at: at(),
            model_id: "test".to_owned(),
            prompt_version: 1,
            subjects: Vec::new(),
            sections: vec![
                section("pricing", Vec::new()),
                section("changes", Vec::new()),
            ],
            sources: Vec::new(),
            interpreted: None,
            notes: Vec::new(),
            progress: None,
            asked: None,
            searches: None,
        };
        let ready = report
            .sections
            .iter()
            .filter(|s| !s.claims.is_empty())
            .count();
        assert_eq!(ready, 0);
    }
}
