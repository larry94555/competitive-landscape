//! What a reader watching sees when the run behind them does not go to plan.
//!
//! Every stream test until now checked a *helper* — does this section serialise, is a
//! correction a different payload. The loop itself, which is where all four of `BENCHMARKS.md`
//! Run 16's defects lived, was never driven by anything.
//!
//! The register in `.claude/skills/coding-mistakes/SKILL.md` is blunt about why that matters:
//! nine of its eleven entries are **states that only exist when something goes wrong
//! mid-operation**, and three of them were introduced by the fix for the previous one. A suite
//! built from complete runs cannot see any of them, because a complete run never enters them.
//!
//! So these drive the real handler, over the real router, against a store the test mutates
//! underneath it — a worker dying, a run being reclaimed, a report being rebuilt from scratch.

#![allow(clippy::expect_used, clippy::panic)]
// Panicking IS how a test reports failure. The lints stay denied everywhere else.

use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::http::Request;
use futures_util::StreamExt;
use landscape_api::{router, AppState};
use landscape_core::{
    AnalysisId, AnalysisStatus, Claim, Confidence, NewAnalysis, Report, Section, SectionStatus,
};
use landscape_db::{MemoryStore, Store};
use tower::ServiceExt;

/// Longer than any of these need, short enough that a hang fails rather than hangs CI.
const PATIENCE: Duration = Duration::from_secs(20);

fn at() -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::parse_from_rfc3339("2026-08-05T09:00:00Z")
        .map(|d| d.with_timezone(&chrono::Utc))
        .unwrap_or_default()
}

/// A report whose pricing section says one thing, so two runs can be told apart.
fn report_saying(text: &str) -> Report {
    Report {
        subject: "basecamp.com".to_owned(),
        searched_as: "https://basecamp.com".to_owned(),
        generated_at: at(),
        model_id: "test".to_owned(),
        prompt_version: 1,
        sections: vec![Section {
            key: "pricing".to_owned(),
            title: "What it costs".to_owned(),
            status: SectionStatus::Populated,
            claims: vec![Claim {
                text: text.to_owned(),
                source_label: "S1".to_owned(),
                evidence_quote: "$15/user, billed monthly".to_owned(),
                confidence: Confidence::High,
                as_of: at(),
            }],
            checked: Vec::new(),
            notes: Vec::new(),
        }],
        sources: Vec::new(),
    }
}

/// Reads server-sent events off the real handler, one at a time.
struct Reader {
    body: futures_util::stream::BoxStream<'static, Result<axum::body::Bytes, axum::Error>>,
    buffered: String,
}

impl Reader {
    async fn open(store: &Arc<dyn Store>, id: AnalysisId) -> Self {
        let app = router(AppState {
            store: Arc::clone(store),
        });
        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/analyses/{}/events", id.0))
                    .body(Body::empty())
                    .expect("a request"),
            )
            .await
            .expect("the stream opens");
        assert_eq!(response.status(), 200, "the stream should open");
        Self {
            body: response.into_body().into_data_stream().boxed(),
            buffered: String::new(),
        }
    }

    /// The next `event: …` block, or `None` once the handler has hung up.
    async fn next(&mut self) -> Option<String> {
        loop {
            if let Some(end) = self.buffered.find("\n\n") {
                let event: String = self.buffered.drain(..end + 2).collect();
                let event = event.trim().to_owned();
                // Keep-alive comments are noise here; the stream sends them so proxies do not
                // close an analysis that spends thirty seconds fetching before it can speak.
                if event.is_empty() || event.starts_with(':') {
                    continue;
                }
                return Some(event);
            }
            let chunk = self.body.next().await?.expect("the stream does not error");
            self.buffered.push_str(&String::from_utf8_lossy(&chunk));
        }
    }

    /// Every event until the stream ends. Fails rather than hangs.
    async fn drain(mut self) -> Vec<String> {
        let mut all = Vec::new();
        let collected = tokio::time::timeout(PATIENCE, async {
            while let Some(event) = self.next().await {
                let last = event.contains("event: done");
                all.push(event);
                if last {
                    break;
                }
            }
            all
        })
        .await;
        collected.unwrap_or_else(|_| panic!("the stream never ended"))
    }
}

async fn running_analysis(store: &Arc<dyn Store>) -> AnalysisId {
    let queued = store
        .enqueue(&NewAnalysis::parse("a competitor report for basecamp.com").expect("valid"))
        .await
        .expect("enqueue");
    store
        .claim_next()
        .await
        .expect("claim")
        .expect("one queued");
    queued.id
}

#[tokio::test]
async fn a_reader_watching_a_reclaimed_run_is_never_told_it_finished() {
    // A worker dies mid-run. Twenty minutes later the sweep returns the row to the queue and
    // another worker starts over. From the reader's side this is one analysis that takes a
    // long time — and the one thing that must not happen is the stream saying `done`, because
    // a reader who has been told it finished does not reconnect.
    let store: Arc<dyn Store> = Arc::new(MemoryStore::new());
    let id = running_analysis(&store).await;
    store
        .save_progress(id, &report_saying("Pro costs $15"))
        .await
        .expect("progress");

    let reader = Reader::open(&store, id).await;

    let driving = {
        let store = Arc::clone(&store);
        tokio::spawn(async move {
            // Let the reader see the first run's answer before anything happens to it.
            tokio::time::sleep(Duration::from_millis(700)).await;

            // The worker died. The sweep finds it.
            let n = store
                .reclaim_stale(chrono::Duration::zero())
                .await
                .expect("reclaim");
            assert_eq!(n, 1, "the stranded run should come back");
            tokio::time::sleep(Duration::from_millis(700)).await;

            // A second worker takes it and reaches a different answer — the first run's page
            // may have been a redirect, a 404, or simply read in a different order.
            store.claim_next().await.expect("claim").expect("requeued");
            store
                .save_progress(id, &report_saying("Pro costs $19"))
                .await
                .expect("progress");
            tokio::time::sleep(Duration::from_millis(700)).await;
            store
                .complete(id, &report_saying("Pro costs $19"))
                .await
                .expect("complete");
        })
    };

    let events = reader.drain().await;
    driving.await.expect("the driver finished");

    let dones = events.iter().filter(|e| e.contains("event: done")).count();
    assert_eq!(
        dones, 1,
        "the stream said `done` more than once, or ended early: {events:#?}"
    );
    assert!(
        events.last().is_some_and(|e| e.contains("event: done")),
        "`done` must be the last thing sent: {events:#?}"
    );

    // The backwards transition is real and the reader is told about it. `running` going back
    // to `queued` is not a state a happy run ever reaches, and a client that assumed status
    // only moves forward would be wrong here rather than at the end.
    let statuses: Vec<&String> = events
        .iter()
        .filter(|e| e.contains("event: status"))
        .collect();
    assert!(
        statuses.iter().any(|e| e.contains("queued")),
        "a reclaimed run goes back to queued and the reader should see it: {statuses:#?}"
    );

    // And the answer the reader is left with is the second run's, not the dead worker's.
    let last_section = events
        .iter()
        .rev()
        .find(|e| e.contains("event: section"))
        .expect("at least one section");
    assert!(
        last_section.contains("Pro costs $19"),
        "the reader was left holding the dead worker's answer: {last_section}"
    );

    // The eventual value is not the interesting part — review pointed out that asserting it
    // misses the *interval*. Between the reclaim and the replacement finding pricing again,
    // this connection has already sent `$15` and nothing has taken it back. So the retraction
    // has to be on the wire, and it has to come before the replacement's answer.
    let reset_at = events
        .iter()
        .position(|e| e.contains("event: reset"))
        .expect("a reclaimed run must retract what it already sent");
    let nineteen_at = events
        .iter()
        .position(|e| e.contains("Pro costs $19"))
        .expect("the second run's answer");
    assert!(
        reset_at < nineteen_at,
        "the retraction has to arrive before the replacement, or the gap is exactly the          window where a reader is looking at a claim nobody stands behind: {events:#?}"
    );
    let fifteen_after: Vec<&String> = events[reset_at..]
        .iter()
        .filter(|e| e.contains("Pro costs $15"))
        .collect();
    assert!(
        fifteen_after.is_empty(),
        "the dead worker's answer was sent again after being retracted: {fifteen_after:#?}"
    );
}

#[tokio::test]
async fn after_a_retraction_the_same_answer_is_sent_again_rather_than_suppressed() {
    // The stream skips a section whose payload it has already sent. After a reset the reader
    // has thrown everything away, so "already sent" is no longer true — and a replacement run
    // that reaches the *same* answer would be suppressed by that memory and never appear.
    //
    // This is the failure mode of every de-duplicating cache: correct until the thing it is
    // deduplicating against is cleared somewhere else.
    let store: Arc<dyn Store> = Arc::new(MemoryStore::new());
    let id = running_analysis(&store).await;
    store
        .save_progress(id, &report_saying("Pro costs $15"))
        .await
        .expect("progress");

    let reader = Reader::open(&store, id).await;
    let driving = {
        let store = Arc::clone(&store);
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(700)).await;
            store
                .reclaim_stale(chrono::Duration::zero())
                .await
                .expect("reclaim");
            tokio::time::sleep(Duration::from_millis(700)).await;
            store.claim_next().await.expect("claim").expect("requeued");
            // The same answer, from a different worker.
            store
                .save_progress(id, &report_saying("Pro costs $15"))
                .await
                .expect("progress");
            tokio::time::sleep(Duration::from_millis(700)).await;
            store
                .complete(id, &report_saying("Pro costs $15"))
                .await
                .expect("complete");
        })
    };

    let events = reader.drain().await;
    driving.await.expect("the driver finished");

    let reset_at = events
        .iter()
        .position(|e| e.contains("event: reset"))
        .expect("a reclaimed run retracts what it sent");
    assert!(
        events[reset_at..].iter().any(|e| e.contains("Pro costs $15")),
        "after retracting it, the stream never sent the answer again - so a reader whose          screen was cleared would sit in front of an empty section for the whole second          run: {events:#?}"
    );
}

#[tokio::test]
async fn a_reclaimed_run_does_not_leave_its_half_report_behind() {
    // The store-level half of the same thing, asserted here as well as in the conformance
    // contract because this is the property the stream above depends on: if the partial
    // report survived a reclaim, a queued analysis would be serving sections from a worker
    // that no longer exists.
    let store: Arc<dyn Store> = Arc::new(MemoryStore::new());
    let id = running_analysis(&store).await;
    store
        .save_progress(id, &report_saying("Pro costs $15"))
        .await
        .expect("progress");
    assert!(store.get(id).await.expect("get").report.is_some());

    store
        .reclaim_stale(chrono::Duration::zero())
        .await
        .expect("reclaim");

    let back = store.get(id).await.expect("get");
    assert_eq!(back.status, AnalysisStatus::Queued);
    assert!(
        back.report.is_none(),
        "nobody stands behind a dead worker's sections; they must not outlive its claim"
    );
}

#[tokio::test]
async fn a_stream_that_opens_after_the_reclaim_still_retracts() {
    // Review found the hole in the first version of the retraction. It fired only when *this
    // connection* had sent something — and the reader's sections deliberately survive a
    // reconnect, while `sent_sections` starts empty on every new stream.
    //
    // So: the connection that saw `$15` is already gone. This one opens on a row that was
    // reclaimed while nobody was watching, and it has to retract anyway, because the reader
    // it is talking to is still holding what the previous connection delivered.
    let store: Arc<dyn Store> = Arc::new(MemoryStore::new());
    let id = running_analysis(&store).await;
    store
        .save_progress(id, &report_saying("Pro costs $15"))
        .await
        .expect("progress");
    store
        .reclaim_stale(chrono::Duration::zero())
        .await
        .expect("reclaim");

    let reader = Reader::open(&store, id).await;
    let driving = {
        let store = Arc::clone(&store);
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(900)).await;
            store.claim_next().await.expect("claim").expect("requeued");
            store
                .complete(id, &report_saying("Pro costs $19"))
                .await
                .expect("complete");
        })
    };

    let events = reader.drain().await;
    driving.await.expect("the driver finished");

    assert!(
        events.iter().any(|e| e.contains("event: reset")),
        "a stream opening on a reclaimed row sent no retraction, so a reader who reconnected          into it keeps the dead worker's answer: {events:#?}"
    );
}

#[tokio::test]
async fn a_run_with_nothing_written_yet_retracts_once_and_not_on_every_poll() {
    // The cost of stating the rule about the row rather than the connection: an ordinary new
    // analysis has no report either, so a stream opens by retracting nothing. That is correct
    // and costs a reader nothing — but the loop polls twice a second, and a retraction per
    // poll would be a section-clearing event twice a second for as long as the first page
    // takes to read.
    let store: Arc<dyn Store> = Arc::new(MemoryStore::new());
    let id = running_analysis(&store).await;

    let reader = Reader::open(&store, id).await;
    let driving = {
        let store = Arc::clone(&store);
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(1600)).await;
            store
                .complete(id, &report_saying("Pro costs $15"))
                .await
                .expect("complete");
        })
    };

    let events = reader.drain().await;
    driving.await.expect("the driver finished");

    let resets = events.iter().filter(|e| e.contains("event: reset")).count();
    assert_eq!(
        resets, 1,
        "a report-less row should be retracted once per episode, not once per poll: {events:#?}"
    );
}

#[tokio::test]
async fn a_second_reclaim_on_the_same_connection_retracts_again() {
    // The retraction is sent once per report-less episode rather than once per poll, which
    // means something has to *rearm* it. Nothing tested that, and the flag not clearing is
    // both the easiest way to write it and completely silent — the first reclaim would work,
    // and a run unlucky enough to lose two workers would leave the second dead worker's
    // answers on screen with no sign anything was wrong.
    let store: Arc<dyn Store> = Arc::new(MemoryStore::new());
    let id = running_analysis(&store).await;
    store
        .save_progress(id, &report_saying("Pro costs $15"))
        .await
        .expect("progress");

    let reader = Reader::open(&store, id).await;
    let driving = {
        let store = Arc::clone(&store);
        tokio::spawn(async move {
            for answer in ["Pro costs $19", "Pro costs $23"] {
                tokio::time::sleep(Duration::from_millis(700)).await;
                store
                    .reclaim_stale(chrono::Duration::zero())
                    .await
                    .expect("reclaim");
                tokio::time::sleep(Duration::from_millis(700)).await;
                store.claim_next().await.expect("claim").expect("requeued");
                store
                    .save_progress(id, &report_saying(answer))
                    .await
                    .expect("progress");
            }
            tokio::time::sleep(Duration::from_millis(700)).await;
            store
                .complete(id, &report_saying("Pro costs $23"))
                .await
                .expect("complete");
        })
    };

    let events = reader.drain().await;
    driving.await.expect("the driver finished");

    let resets = events.iter().filter(|e| e.contains("event: reset")).count();
    assert_eq!(
        resets, 2,
        "two workers died and only one retraction was sent, so the reader kept the second          one's answers: {events:#?}"
    );
    assert!(
        events.iter().any(|e| e.contains("Pro costs $23")),
        "the last run's answer never arrived: {events:#?}"
    );
}

#[tokio::test]
async fn a_run_that_fails_ends_the_stream_with_a_reason_rather_than_a_silence() {
    // The other terminal state. A failure that closes the connection without a `done` looks
    // exactly like a dropped network to a client, so it reconnects — and then reconnects
    // again, for as long as the reader leaves the tab open.
    let store: Arc<dyn Store> = Arc::new(MemoryStore::new());
    let id = running_analysis(&store).await;
    let reader = Reader::open(&store, id).await;

    let driving = {
        let store = Arc::clone(&store);
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(700)).await;
            store
                .fail(
                    id,
                    landscape_core::Failure::NoSubject,
                    "no subject in the prompt",
                )
                .await
                .expect("fail");
        })
    };

    let events = reader.drain().await;
    driving.await.expect("the driver finished");

    assert!(
        events.iter().any(|e| e.contains("failed")),
        "the reader should be told the status, not just that it stopped: {events:#?}"
    );
    assert!(
        events.last().is_some_and(|e| e.contains("event: done")),
        "a failed run still ends the stream properly: {events:#?}"
    );
}

#[tokio::test]
async fn a_section_that_arrives_empty_is_not_sent_until_it_has_something() {
    // The condition the loop filters on, driven through the loop rather than asserted on the
    // helper. A section with no claims and a "not found" status is a question still being
    // read, and sending it would tell a reader we had finished looking at it.
    let store: Arc<dyn Store> = Arc::new(MemoryStore::new());
    let id = running_analysis(&store).await;

    let mut empty = report_saying("Pro costs $15");
    empty.sections[0].claims.clear();
    empty.sections[0].status = SectionStatus::NotFoundInPublicSources;
    store.save_progress(id, &empty).await.expect("progress");

    let reader = Reader::open(&store, id).await;
    let driving = {
        let store = Arc::clone(&store);
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(900)).await;
            store
                .complete(id, &report_saying("Pro costs $15"))
                .await
                .expect("complete");
        })
    };

    let events = reader.drain().await;
    driving.await.expect("the driver finished");

    let sections: Vec<&String> = events
        .iter()
        .filter(|e| e.contains("event: section"))
        .collect();
    assert!(
        sections.iter().all(|e| e.contains("Pro costs $15")),
        "an empty section was sent while its question was still being read: {sections:#?}"
    );
}
