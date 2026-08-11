//! One test body, run against every [`Store`] implementation.
//!
//! Two implementations of the same trait drift. Writing the behavior once and running it
//! against both is the cheapest way to keep the in-memory store honest — if it stops
//! matching Postgres, the API's fast tests stop meaning anything.
//!
//! This lives in `src/` rather than `tests/` so both the unit test in `memory.rs` and the
//! Postgres integration test can call it.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
// This module is test scaffolding: an assertion failure here must abort loudly.

use landscape_core::{AnalysisStatus, Applied, Failure, NewAnalysis, Report};

use crate::Store;

fn sample_report(subject: &str) -> Report {
    Report {
        subject: subject.to_owned(),
        searched_as: "ordering software for small farms".to_owned(),
        generated_at: chrono::Utc::now(),
        model_id: "test".to_owned(),
        prompt_version: 1,
        subjects: Vec::new(),
        sections: Vec::new(),
        sources: Vec::new(),
        interpreted: None,
        notes: Vec::new(),
    }
}

/// A report with something in it, so "the partial report is gone" is a real assertion rather
/// than one that would pass against an empty one.
fn half_a_report() -> Report {
    Report {
        sections: vec![landscape_core::Section::not_found(
            "pricing",
            "What it costs",
            vec!["https://example.com/pricing".to_owned()],
        )],
        ..sample_report("example.com")
    }
}

fn prompt(text: &str) -> NewAnalysis {
    NewAnalysis::parse(text).unwrap_or_else(|e| panic!("test prompt {text:?} is invalid: {e}"))
}

/// Exercise the whole contract. Panics with a description on the first difference.
pub async fn run(store: &impl Store) {
    // --- enqueue and read back -------------------------------------------------
    let a = store
        .enqueue(&prompt("an app that helps small farms sell to restaurants"))
        .await
        .expect("enqueue");
    assert_eq!(a.status, AnalysisStatus::Queued, "a new analysis is queued");
    assert!(a.report.is_none(), "a new analysis has no report");

    let fetched = store.get(a.id).await.expect("get after enqueue");
    assert_eq!(fetched.id, a.id);
    assert_eq!(fetched.prompt, a.prompt, "the prompt survives storage");

    // --- an unknown id is not found, rather than a silent empty ----------------
    let missing = landscape_core::AnalysisId::new();
    assert!(
        store.get(missing).await.is_err(),
        "an unknown id must be an error, not a default value"
    );

    // --- claiming is FIFO ------------------------------------------------------
    let b = store
        .enqueue(&prompt(
            "a tool that chases unpaid invoices for freelancers",
        ))
        .await
        .expect("enqueue second");

    let first = store
        .claim_next()
        .await
        .expect("claim")
        .expect("one queued");
    assert_eq!(
        first.id, a.id,
        "the oldest queued analysis is claimed first"
    );
    assert_eq!(
        first.status,
        AnalysisStatus::Running,
        "claiming marks it running"
    );
    assert!(
        first.generation > a.generation,
        "claiming has to raise the generation, or a worker cannot be told its claim is gone"
    );

    // --- a claimed analysis is never handed out twice --------------------------
    let second = store
        .claim_next()
        .await
        .expect("claim")
        .expect("one queued");
    assert_eq!(
        second.id, b.id,
        "the second claim gets the next one, not a repeat"
    );

    let third = store.claim_next().await.expect("claim");
    assert!(third.is_none(), "an empty queue yields nothing");

    // --- progress is visible while it is still running -------------------------
    // What makes streaming possible: a reader waiting ninety seconds sees the pricing
    // section when pricing is done, not when everything is.
    let partial = sample_report(&a.prompt);
    assert_eq!(
        store
            .save_progress(a.id, first.generation, &partial)
            .await
            .expect("save progress"),
        Applied::Yes
    );
    let running = store.get(a.id).await.expect("get during run");
    assert_eq!(
        running.status,
        AnalysisStatus::Running,
        "progress does not finish an analysis"
    );
    assert!(
        running.report.is_some(),
        "a running analysis can carry the report so far"
    );

    // --- completing attaches the report ----------------------------------------
    // --- a revoked claim cannot write ------------------------------------------
    // The case `status` cannot express. A worker slower than the staleness threshold and the
    // replacement the sweep handed its row to *both* see `running`, so without a number to
    // compare, whichever finished last won - and the reader got whichever report that was.
    let stale = first.generation.saturating_sub(1);
    assert_eq!(
        store
            .save_progress(a.id, stale, &sample_report("a run nobody is waiting for"))
            .await
            .expect("a revoked write is an outcome, not an error"),
        Applied::ClaimRevoked,
        "a worker holding an older generation wrote progress over the current run"
    );
    assert_eq!(
        store
            .complete(a.id, stale, &sample_report("a run nobody is waiting for"))
            .await
            .expect("a revoked write is an outcome, not an error"),
        Applied::ClaimRevoked,
        "a worker holding an older generation finished a run it no longer owns"
    );
    assert_eq!(
        store
            .fail(
                a.id,
                stale,
                crate::Refused {
                    kind: Failure::Internal,
                    reason: "from a worker long replaced",
                    choices: &[],
                }
            )
            .await
            .expect("a revoked write is an outcome, not an error"),
        Applied::ClaimRevoked,
        "a worker holding an older generation failed a run it no longer owns"
    );
    let untouched = store.get(a.id).await.expect("get after revoked writes");
    assert_eq!(
        untouched.status,
        AnalysisStatus::Running,
        "a revoked write changed the status"
    );
    assert_eq!(
        untouched.report.map(|r| r.subject),
        Some(partial.subject.clone()),
        "a revoked write replaced the current run's report"
    );

    // --- completing attaches the report ----------------------------------------
    let report = sample_report(&a.prompt);
    assert_eq!(
        store
            .complete(a.id, first.generation, &report)
            .await
            .expect("complete"),
        Applied::Yes
    );

    let done = store.get(a.id).await.expect("get after complete");
    assert_eq!(done.status, AnalysisStatus::Complete);
    let stored = done
        .report
        .expect("a completed analysis carries its report");
    assert_eq!(
        stored.subject, report.subject,
        "the report survives storage"
    );
    assert_eq!(
        stored.searched_as, report.searched_as,
        "searched_as survives storage"
    );

    // --- a late write does not resurrect a finished run ------------------------
    // A worker that lost a race writes progress after another finished the same row. The
    // correct outcome is nothing at all: the reader is looking at a complete report.
    assert_eq!(
        store
            .save_progress(
                a.id,
                first.generation,
                &sample_report("something else entirely")
            )
            .await
            .expect("a late write is not an error"),
        Applied::ClaimRevoked,
        "a write to a finished run should report that it did nothing"
    );
    let after = store.get(a.id).await.expect("get after a late write");
    assert_eq!(after.status, AnalysisStatus::Complete, "still complete");
    assert_eq!(
        after.report.map(|r| r.subject),
        Some(report.subject.clone()),
        "the finished report was not overwritten"
    );

    // --- failing is terminal and records nothing user-facing -------------------
    assert_eq!(
        store
            .fail(
                b.id,
                second.generation,
                crate::Refused {
                    kind: Failure::Internal,
                    reason: "network unreachable",
                    choices: &[],
                }
            )
            .await
            .expect("fail"),
        Applied::Yes
    );
    let failed = store.get(b.id).await.expect("get after fail");
    assert_eq!(failed.status, AnalysisStatus::Failed);
    assert_eq!(
        failed.failure,
        Some(Failure::Internal),
        "a failed analysis says which situation it is in, so an interface can write a sentence a reader can act on"
    );
    assert!(
        failed.report.is_none(),
        "a failed analysis must not carry a partial report"
    );
    assert!(
        failed.choices.is_empty(),
        "a refusal with nothing to pick between must not invent a question"
    );
    assert!(
        done.choices.is_empty(),
        "a finished report must not offer a choice — see analysis_from_row"
    );

    // --- a refusal that asks a question carries the question ------------------
    // Storing `Ambiguous` without the candidates leaves an interface able to say a name
    // matched several companies and unable to say which, which is the reader guessing at
    // exactly what the gate refused to guess at.
    let c = store
        .enqueue(&prompt("notion, and whoever competes with it"))
        .await
        .expect("enqueue for the ambiguous case");
    let third = store
        .claim_next()
        .await
        .expect("claim for the ambiguous case")
        .expect("a queued analysis to claim");
    assert_eq!(third.id, c.id, "the queue handed back a different analysis");
    let offered = [
        landscape_core::Choice {
            name: "Notion".to_owned(),
            domain: "notion.so".to_owned(),
            what_it_is: "one workspace for notes, docs and projects".to_owned(),
            prompt: "notion.so".to_owned(),
        },
        landscape_core::Choice {
            name: "Notion Energy".to_owned(),
            domain: "notionenergy.com".to_owned(),
            // Empty on purpose: a front page that said nothing quotable must round-trip as
            // an absent line and not as a missing choice.
            what_it_is: String::new(),
            prompt: "notionenergy.com".to_owned(),
        },
    ];
    assert_eq!(
        store
            .fail(
                third.id,
                third.generation,
                crate::Refused {
                    kind: Failure::Ambiguous,
                    reason: "two companies share that name",
                    choices: &offered,
                }
            )
            .await
            .expect("fail with choices"),
        Applied::Yes
    );
    let asked = store
        .get(c.id)
        .await
        .expect("get after an ambiguous refusal");
    assert_eq!(
        asked.failure,
        Some(Failure::Ambiguous),
        "the situation survives storage"
    );
    assert_eq!(
        asked.choices,
        offered.to_vec(),
        "every field of every choice survives storage, in the order it was offered"
    );

    // --- counting ---------------------------------------------------------------
    assert_eq!(
        store
            .count_with_status(AnalysisStatus::Complete)
            .await
            .expect("count"),
        1
    );
    assert_eq!(
        store
            .count_with_status(AnalysisStatus::Queued)
            .await
            .expect("count"),
        0
    );

    // --- a completed analysis is not re-claimed --------------------------------
    assert!(
        store.claim_next().await.expect("claim").is_none(),
        "terminal analyses must never be claimed again"
    );

    // --- a job whose worker died comes back ------------------------------------
    // A worker killed mid-analysis leaves its row `running` with nothing to finish it.
    // Nothing failed, so no error is recorded; the row is simply stranded and the reader
    // watches a spinner that never resolves.
    let stranded = store
        .enqueue(&prompt("a service for booking guitar lessons online"))
        .await
        .expect("enqueue");
    let claimed = store
        .claim_next()
        .await
        .expect("claim")
        .expect("one queued");
    assert_eq!(claimed.status, AnalysisStatus::Running);

    // Nothing is old enough yet, so nothing moves. A reclaimer that fires early would
    // hand a second worker a job the first is still working on.
    let none_yet = store
        .reclaim_stale(chrono::Duration::hours(1))
        .await
        .expect("reclaim");
    assert_eq!(none_yet, 0, "a job that just started must not be reclaimed");
    assert_eq!(
        store.get(stranded.id).await.expect("get").status,
        AnalysisStatus::Running
    );

    // The dead worker got partway. This is the state a reclaim actually finds: not a blank
    // row, but one carrying a half-written report nobody is going to finish.
    store
        .save_progress(stranded.id, claimed.generation, &half_a_report())
        .await
        .expect("progress");
    assert!(
        store
            .get(stranded.id)
            .await
            .expect("get")
            .report
            .is_some_and(|r| !r.sections.is_empty()),
        "the setup for the next assertion did not take"
    );

    // With a zero threshold everything running is overdue.
    let reclaimed = store
        .reclaim_stale(chrono::Duration::zero())
        .await
        .expect("reclaim");
    assert_eq!(reclaimed, 1, "the stranded analysis should come back");
    let back = store.get(stranded.id).await.expect("get");
    assert_eq!(
        back.status,
        AnalysisStatus::Queued,
        "a reclaimed analysis is queued again, not failed - nothing has gone wrong with it"
    );
    assert!(
        back.generation > claimed.generation,
        "the sweep has to raise the generation, or the worker it just replaced can still write - and it will not find out until it tries"
    );
    assert!(
        back.report.is_none(),
        "a reclaimed analysis still carries the dead worker's partial report. Nobody stands \
         behind those sections any more, and the run that replaces them starts from an empty \
         report - so a reader watching sees answers they were already shown blank themselves \
         out, which reads as a retraction rather than a restart"
    );

    // And it is claimable again, which is the whole point.
    let again = store.claim_next().await.expect("claim").expect("requeued");
    assert_eq!(again.id, stranded.id);

    // Terminal analyses are never touched, however old.
    let terminal_untouched = store
        .reclaim_stale(chrono::Duration::zero())
        .await
        .expect("reclaim");
    assert_eq!(
        terminal_untouched, 1,
        "only the running one; complete and failed stay put"
    );
    assert_eq!(
        store.get(a.id).await.expect("get").status,
        AnalysisStatus::Complete,
        "a completed analysis must never be reclaimed"
    );
}
