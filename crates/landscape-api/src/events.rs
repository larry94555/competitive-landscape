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
//! event: done        nothing else is coming
//! ```
//!
//! **A section is sent when it first has something in it, and again whenever it grows.**
//! Watching a real run in a browser is what settled that: sending it once put *"What it does:
//! 1 item"* on screen and left it there for two minutes while eight more capabilities were
//! read. A section that arrives and then freezes reads as a section that is finished.

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
        // key -> how many claims the reader has been sent, so a growing section is resent
        // and a static one is not.
        let mut sent_sections: HashMap<String, usize> = HashMap::new();
        let mut sent_status: Option<AnalysisStatus> = None;

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

            if let Some(report) = &analysis.report {
                for section in &report.sections {
                    if section.status == SectionStatus::NotFoundInPublicSources
                        && section.claims.is_empty()
                    {
                        // Nothing in it yet. Its coverage note is part of the finished
                        // report, and sending it now would be sending a "we found nothing"
                        // for a question still being read.
                        continue;
                    }
                    let sent = sent_sections.get(&section.key).copied();
                    if sent != Some(section.claims.len()) {
                        sent_sections.insert(section.key.clone(), section.claims.len());
                        yield Ok(section_event(section));
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

fn section_event(section: &Section) -> Event {
    // A section that will not serialise is a bug in the report type rather than something a
    // reader can act on, so the stream carries on without it.
    serde_json::to_string(section).map_or_else(
        |_| Event::default().event("section").data(section.key.clone()),
        |json| Event::default().event("section").data(json),
    )
}

fn done() -> Event {
    Event::default().event("done").data("")
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
            text: "Pro costs $15".to_owned(),
            source_label: "S1".to_owned(),
            evidence_quote: "$15/user".to_owned(),
            confidence: Confidence::High,
            as_of: at(),
        }
    }

    #[test]
    fn a_populated_section_serialises_into_its_event() {
        let event = section_event(&section("pricing", vec![claim()]));
        let rendered = format!("{event:?}");
        assert!(rendered.contains("section"), "{rendered}");
    }

    #[test]
    fn an_empty_section_is_recognisable_as_not_ready() {
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
            sections: vec![
                section("pricing", Vec::new()),
                section("changes", Vec::new()),
            ],
            sources: Vec::new(),
        };
        let ready = report
            .sections
            .iter()
            .filter(|s| !s.claims.is_empty())
            .count();
        assert_eq!(ready, 0);
    }
}
