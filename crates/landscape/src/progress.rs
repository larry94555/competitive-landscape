//! Writing the report-so-far to the store, in order, while the run carries on.
//!
//! # Why this is not four lines of `tokio::spawn`
//!
//! It was. `analyze_with` hands out the whole report after every window, the worker spawned a
//! task per snapshot, and the reasoning written beside it was: *ordering is not a concern
//! because each write is the whole report so far, not a delta.*
//!
//! **That is true of the payloads and false of the writes.** Two spawned tasks are two
//! concurrent database round-trips over two pooled connections, and nothing makes the first
//! land first. When the second overtakes the first, the store ends up holding the *older*
//! report — so a section that had four claims goes back to three, and `PagePricing::assembled`
//! replacing *"Free is listed with no published price"* with *"Free costs $0"* is undone in
//! front of a reader who already saw the correction. It repairs itself at `complete`, which
//! means the window where it is visible is exactly the ninety seconds somebody is watching.
//!
//! This is the same defect as the `claims.len()` one in `BENCHMARKS.md` Run 16 wearing a
//! different hat: a correction that arrives and then un-arrives. The register in
//! `.claude/skills/coding-mistakes/SKILL.md` calls the class *"a state that only exists when
//! something goes wrong mid-operation"*, and nine of its eleven entries are that.
//!
//! # What it does instead
//!
//! One writer task, fed by a [`tokio::sync::watch`] channel. The task writes, then looks at
//! what the latest snapshot is *now* — so writes are strictly ordered, and a run producing
//! snapshots faster than the store can take them **skips the intermediate ones rather than
//! queueing them**. Skipping is right: every snapshot is a whole report, so the newest is
//! strictly better than a backlog of stale ones, and a queue that grows under load is how a
//! slow database turns into a slow reader.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use landscape_core::{AnalysisId, Applied, Report};
use landscape_db::Store;

/// Accepts snapshots from the pipeline and writes them one at a time.
///
/// Dropping it stops the writer. [`Progress::finish`] is the version that waits, which is what
/// the worker wants before it calls `complete` — otherwise the last progress write can land
/// *after* the final report and leave a finished analysis holding a partial one.
pub(crate) struct Progress {
    latest: tokio::sync::watch::Sender<Option<Report>>,
    writer: tokio::task::JoinHandle<Outcome>,
    /// Set by the writer task the first time a write is refused, read by [`Progress::record`].
    ///
    /// The revocation is discovered asynchronously — `record` cannot await — so the answer it
    /// gives is *as of the last write that landed*, one or two windows behind. That is soon
    /// enough: it saves the rest of the run, which is what the cost was.
    revoked: Arc<AtomicBool>,
}

/// What the writer task found out while nobody was looking.
///
/// A revoked claim is discovered by a *write*, which happens inside a spawned task with no way
/// to return anything. Carrying it back out of [`Progress::finish`] is what lets the worker
/// say so once, rather than the store logging it four times a minute.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Outcome {
    /// Every write that was attempted took effect.
    Ours,
    /// The row moved to a later generation. This worker has been replaced.
    Revoked,
}

impl Progress {
    /// Start writing progress for one analysis.
    pub(crate) fn new(store: Arc<dyn Store>, id: AnalysisId, generation: u32) -> Self {
        let (latest, mut rx) = tokio::sync::watch::channel(None::<Report>);
        let revoked = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&revoked);
        let writer = tokio::spawn(async move {
            let mut outcome = Outcome::Ours;
            // `changed()` returns an error once every sender is gone, which is the signal to
            // stop: the run is over and `complete` owns the report from here.
            while rx.changed().await.is_ok() {
                let snapshot = rx.borrow_and_update().clone();
                let Some(report) = snapshot else { continue };
                match store.save_progress(id, generation, &report).await {
                    // A revoked claim is not an error and not worth a line per write. The
                    // sweep decided this row belongs to somebody else; the worker is told
                    // once, at the end.
                    Ok(Applied::ClaimRevoked) => {
                        outcome = Outcome::Revoked;
                        flag.store(true, Ordering::Relaxed);
                    }
                    Ok(Applied::Yes) => {}
                    Err(e) => {
                        // A lost intermediate write costs a reader a few seconds of
                        // staleness. Abandoning the run over it would cost them the report.
                        tracing::warn!(%id, error = %e, "could not save progress");
                    }
                }
            }
            outcome
        });
        Self {
            latest,
            writer,
            revoked,
        }
    }

    /// Record the report so far, and say whether the run is still wanted.
    ///
    /// Called from a synchronous callback inside the pipeline, which is why it cannot await —
    /// and why the answer is the one the last landed write produced rather than this one's.
    pub(crate) fn record(&self, report: &Report) -> landscape_analyze::Wanted {
        // `send_replace` rather than `send`: with no receiver left — the writer task having
        // ended — `send` reports an error nobody can act on, and the run must continue anyway.
        self.latest.send_replace(Some(report.clone()));
        if self.revoked.load(Ordering::Relaxed) {
            landscape_analyze::Wanted::No
        } else {
            landscape_analyze::Wanted::Yes
        }
    }

    /// Stop accepting snapshots and wait for the one in flight to land.
    ///
    /// **The waiting is the point.** `complete` writes the finished report, and a progress
    /// write still in flight would land after it. Both stores refuse a `save_progress` on a
    /// row that is no longer `running`, so the damage is bounded — but relying on that is
    /// relying on a guard for ordinary operation, and the guard exists for lost races.
    pub(crate) async fn finish(self) -> Outcome {
        drop(self.latest);
        match self.writer.await {
            Ok(outcome) => outcome,
            Err(e) => {
                tracing::error!(error = %e, "the progress writer stopped badly");
                // The writes may or may not have landed. Claiming the run is still ours is
                // the safe reading: `complete` checks the generation for itself.
                Outcome::Ours
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
// Panicking IS how a test reports failure. The lints stay denied everywhere else.
mod tests {
    use std::sync::Mutex;
    use std::time::Duration;

    use landscape_core::{Analysis, AnalysisStatus, NewAnalysis, Section};
    use landscape_db::{MemoryStore, Result};

    use super::*;

    /// A store whose first write is slow, so a second can overtake it.
    ///
    /// This is the whole test: with a task spawned per snapshot the older report lands last,
    /// and no amount of reading the code makes that visible.
    #[derive(Debug)]
    struct SlowFirstWrite {
        inner: MemoryStore,
        writes: Mutex<Vec<String>>,
        seen: Mutex<usize>,
    }

    impl SlowFirstWrite {
        fn new() -> Self {
            Self {
                inner: MemoryStore::new(),
                writes: Mutex::new(Vec::new()),
                seen: Mutex::new(0),
            }
        }

        /// The order the writes actually *landed* in, named by their first section's title.
        fn landed(&self) -> Vec<String> {
            self.writes.lock().expect("not poisoned").clone()
        }
    }

    #[async_trait::async_trait]
    impl Store for SlowFirstWrite {
        async fn enqueue(&self, new: &NewAnalysis) -> Result<Analysis> {
            self.inner.enqueue(new).await
        }
        async fn get(&self, id: AnalysisId) -> Result<Analysis> {
            self.inner.get(id).await
        }
        async fn claim_next(&self) -> Result<Option<Analysis>> {
            self.inner.claim_next().await
        }
        async fn complete(
            &self,
            id: AnalysisId,
            generation: u32,
            report: &Report,
        ) -> Result<Applied> {
            self.inner.complete(id, generation, report).await
        }
        async fn fail(
            &self,
            id: AnalysisId,
            generation: u32,
            refused: landscape_db::Refused<'_>,
        ) -> Result<Applied> {
            self.inner.fail(id, generation, refused).await
        }
        async fn reclaim_stale(&self, max_age: chrono::Duration) -> Result<u64> {
            self.inner.reclaim_stale(max_age).await
        }
        async fn count_with_status(&self, status: AnalysisStatus) -> Result<i64> {
            self.inner.count_with_status(status).await
        }

        async fn save_progress(
            &self,
            id: AnalysisId,
            generation: u32,
            report: &Report,
        ) -> Result<Applied> {
            let first = {
                let mut seen = self.seen.lock().expect("not poisoned");
                *seen += 1;
                *seen == 1
            };
            if first {
                tokio::time::sleep(Duration::from_millis(120)).await;
            }
            let applied = self.inner.save_progress(id, generation, report).await?;
            if applied == Applied::Yes {
                self.writes.lock().expect("not poisoned").push(
                    report
                        .sections
                        .first()
                        .map_or_else(|| "(no sections)".to_owned(), |s: &Section| s.title.clone()),
                );
            }
            Ok(applied)
        }
    }

    /// A report whose only section is named, so the order writes landed in is readable.
    fn report_titled(title: &str) -> Report {
        Report {
            chosen: None,
            progress: None,
            asked: None,
            searches: None,
            subject: "basecamp.com".to_owned(),
            searched_as: "https://basecamp.com".to_owned(),
            generated_at: chrono::Utc::now(),
            model_id: "test".to_owned(),
            prompt_version: 1,
            subjects: Vec::new(),
            sections: vec![Section::not_found("pricing", title, Vec::new())],
            sources: Vec::new(),
            interpreted: None,
            notes: Vec::new(),
        }
    }

    async fn running_analysis(store: &Arc<dyn Store>) -> (AnalysisId, u32) {
        let queued = store
            .enqueue(&NewAnalysis::parse("a competitor report for basecamp.com").expect("valid"))
            .await
            .expect("enqueue");
        let claimed = store
            .claim_next()
            .await
            .expect("claim")
            .expect("something to claim");
        (queued.id, claimed.generation)
    }

    #[tokio::test]
    async fn a_slow_write_never_overwrites_a_newer_report() {
        // The defect, stated as a test. Two snapshots, the first one slow. Written with a
        // task per snapshot, "first" lands last and the store keeps the older report — which
        // a reader sees as a section losing a claim it had already been shown.
        let slow = Arc::new(SlowFirstWrite::new());
        let store: Arc<dyn Store> = Arc::clone(&slow) as Arc<dyn Store>;
        let (id, generation) = running_analysis(&store).await;

        let progress = Progress::new(Arc::clone(&store), id, generation);
        progress.record(&report_titled("first"));
        // Long enough that a spawned second write would have finished while the first slept.
        tokio::time::sleep(Duration::from_millis(20)).await;
        progress.record(&report_titled("second"));
        progress.finish().await;

        let stored = store.get(id).await.expect("get").report.expect("a report");
        assert_eq!(
            stored.sections[0].title,
            "second",
            "the store kept an older report than the newest one sent: writes landed {:?}",
            slow.landed()
        );
        let landed = slow.landed();
        assert_eq!(
            landed.last().map(String::as_str),
            Some("second"),
            "the last write to land must be the newest snapshot, got {landed:?}"
        );
    }

    #[tokio::test]
    async fn snapshots_produced_faster_than_the_store_are_coalesced_not_queued() {
        // Skipping intermediates is deliberate, and worth asserting so that "fix" it back into
        // a queue is a failing test rather than a plausible-looking refactor. Each snapshot is
        // a whole report, so the newest is strictly better than a backlog of stale ones.
        let slow = Arc::new(SlowFirstWrite::new());
        let store: Arc<dyn Store> = Arc::clone(&slow) as Arc<dyn Store>;
        let (id, generation) = running_analysis(&store).await;

        let progress = Progress::new(Arc::clone(&store), id, generation);
        for title in ["one", "two", "three", "four"] {
            progress.record(&report_titled(title));
        }
        progress.finish().await;

        let landed = slow.landed();
        assert!(
            landed.len() < 4,
            "every snapshot was written; they should coalesce, got {landed:?}"
        );
        assert_eq!(
            landed.last().map(String::as_str),
            Some("four"),
            "coalescing may drop intermediate snapshots but never the newest, got {landed:?}"
        );
    }

    #[tokio::test]
    async fn finishing_waits_for_the_write_in_flight() {
        // Without the wait, `complete` and the last progress write race, and the finished
        // report is whichever lands second. Both stores refuse a late `save_progress`, so this
        // is belt and braces — but the guard exists for lost races, not for ordinary operation.
        let slow = Arc::new(SlowFirstWrite::new());
        let store: Arc<dyn Store> = Arc::clone(&slow) as Arc<dyn Store>;
        let (id, generation) = running_analysis(&store).await;

        let progress = Progress::new(Arc::clone(&store), id, generation);
        progress.record(&report_titled("only"));
        progress.finish().await;

        assert_eq!(
            slow.landed(),
            vec!["only".to_owned()],
            "finish returned before the write it was waiting for"
        );
    }

    #[tokio::test]
    async fn a_worker_whose_claim_was_revoked_is_told_so() {
        // The sweep decided this run was dead and handed it to somebody else, while this
        // worker was still working on it. Every write it makes from here is refused — which
        // is the point — but a refusal nobody surfaces is a machine spending ninety seconds
        // of prefill on a report that will be thrown away, with nothing in the log saying so.
        let store: Arc<dyn Store> = Arc::new(MemoryStore::new());
        let (id, generation) = running_analysis(&store).await;

        let progress = Progress::new(Arc::clone(&store), id, generation);
        progress.record(&report_titled("before"));
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Twenty minutes pass, as far as the sweep is concerned.
        store
            .reclaim_stale(chrono::Duration::zero())
            .await
            .expect("reclaim");
        store.claim_next().await.expect("claim").expect("requeued");

        progress.record(&report_titled("after"));
        assert_eq!(
            progress.finish().await,
            Outcome::Revoked,
            "the worker was never told its run had been given to somebody else"
        );

        let stored = store.get(id).await.expect("get").report;
        assert!(
            stored.is_none(),
            "a replaced worker wrote over the run that replaced it"
        );
    }

    #[tokio::test]
    async fn recording_says_to_stop_once_the_claim_is_gone() {
        // The saving is everything the run would have done next. `record` is the only thing
        // the pipeline calls often enough to carry the answer, and it is already the thing
        // that discovers it — a refused write *is* how a worker learns it was replaced.
        let store: Arc<dyn Store> = Arc::new(MemoryStore::new());
        let (id, generation) = running_analysis(&store).await;
        let progress = Progress::new(Arc::clone(&store), id, generation);

        assert_eq!(
            progress.record(&report_titled("first")),
            landscape_analyze::Wanted::Yes,
            "a run nobody has touched should carry on"
        );
        tokio::time::sleep(Duration::from_millis(50)).await;

        store
            .reclaim_stale(chrono::Duration::zero())
            .await
            .expect("reclaim");
        store.claim_next().await.expect("claim").expect("requeued");

        // The first record after the reclaim is the write that gets refused; the answer is
        // as of the last write that *landed*, so it takes one more to come back.
        progress.record(&report_titled("during"));
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(
            progress.record(&report_titled("after")),
            landscape_analyze::Wanted::No,
            "a replaced worker was told to carry on, and would have read every remaining page"
        );
    }

    #[tokio::test]
    async fn a_run_nobody_interrupted_reports_that_it_kept_its_claim() {
        // The other half, so `Revoked` cannot be what the type always says.
        let store: Arc<dyn Store> = Arc::new(MemoryStore::new());
        let (id, generation) = running_analysis(&store).await;
        let progress = Progress::new(Arc::clone(&store), id, generation);
        progress.record(&report_titled("only"));
        assert_eq!(progress.finish().await, Outcome::Ours);
    }

    #[tokio::test]
    async fn recording_after_the_run_is_over_does_not_panic() {
        // `record` is called from a synchronous callback deep inside the pipeline, where
        // there is nothing sensible to do with an error. It must be impossible to fail.
        let store: Arc<dyn Store> = Arc::new(MemoryStore::new());
        let (id, generation) = running_analysis(&store).await;
        let progress = Progress::new(Arc::clone(&store), id, generation);
        progress.writer.abort();
        progress.record(&report_titled("after"));
    }
}
