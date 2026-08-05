//! Writing the report-so-far to the store, in order, while the run carries on.
//!
//! # Why this is not four lines of `tokio::spawn`
//!
//! It was. `analyse_with` hands out the whole report after every window, the worker spawned a
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

use std::sync::Arc;

use landscape_core::{AnalysisId, Report};
use landscape_db::Store;

/// Accepts snapshots from the pipeline and writes them one at a time.
///
/// Dropping it stops the writer. [`Progress::finish`] is the version that waits, which is what
/// the worker wants before it calls `complete` — otherwise the last progress write can land
/// *after* the final report and leave a finished analysis holding a partial one.
pub(crate) struct Progress {
    latest: tokio::sync::watch::Sender<Option<Report>>,
    writer: tokio::task::JoinHandle<()>,
}

impl Progress {
    /// Start writing progress for one analysis.
    pub(crate) fn new(store: Arc<dyn Store>, id: AnalysisId) -> Self {
        let (latest, mut rx) = tokio::sync::watch::channel(None::<Report>);
        let writer = tokio::spawn(async move {
            // `changed()` returns an error once every sender is gone, which is the signal to
            // stop: the run is over and `complete` owns the report from here.
            while rx.changed().await.is_ok() {
                let snapshot = rx.borrow_and_update().clone();
                let Some(report) = snapshot else { continue };
                if let Err(e) = store.save_progress(id, &report).await {
                    // A lost intermediate write costs a reader a few seconds of staleness.
                    // Abandoning the run over it would cost them the report.
                    tracing::warn!(%id, error = %e, "could not save progress");
                }
            }
        });
        Self { latest, writer }
    }

    /// Record the report so far. Never blocks, and never fails the run.
    ///
    /// Called from a synchronous callback inside the pipeline, which is why it cannot await.
    pub(crate) fn record(&self, report: &Report) {
        // `send_replace` rather than `send`: with no receiver left — the writer task having
        // ended — `send` reports an error nobody can act on, and the run must continue anyway.
        self.latest.send_replace(Some(report.clone()));
    }

    /// Stop accepting snapshots and wait for the one in flight to land.
    ///
    /// **The waiting is the point.** `complete` writes the finished report, and a progress
    /// write still in flight would land after it. Both stores refuse a `save_progress` on a
    /// row that is no longer `running`, so the damage is bounded — but relying on that is
    /// relying on a guard for ordinary operation, and the guard exists for lost races.
    pub(crate) async fn finish(self) {
        drop(self.latest);
        if let Err(e) = self.writer.await {
            tracing::error!(error = %e, "the progress writer stopped badly");
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
// Panicking IS how a test reports failure. The lints stay denied everywhere else.
mod tests {
    use std::sync::Mutex;
    use std::time::Duration;

    use landscape_core::{Analysis, AnalysisStatus, Failure, NewAnalysis, Section};
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
        async fn complete(&self, id: AnalysisId, report: &Report) -> Result<()> {
            self.inner.complete(id, report).await
        }
        async fn fail(&self, id: AnalysisId, kind: Failure, reason: &str) -> Result<()> {
            self.inner.fail(id, kind, reason).await
        }
        async fn reclaim_stale(&self, max_age: chrono::Duration) -> Result<u64> {
            self.inner.reclaim_stale(max_age).await
        }
        async fn count_with_status(&self, status: AnalysisStatus) -> Result<i64> {
            self.inner.count_with_status(status).await
        }

        async fn save_progress(&self, id: AnalysisId, report: &Report) -> Result<()> {
            let first = {
                let mut seen = self.seen.lock().expect("not poisoned");
                *seen += 1;
                *seen == 1
            };
            if first {
                tokio::time::sleep(Duration::from_millis(120)).await;
            }
            self.inner.save_progress(id, report).await?;
            self.writes.lock().expect("not poisoned").push(
                report
                    .sections
                    .first()
                    .map_or_else(|| "(no sections)".to_owned(), |s: &Section| s.title.clone()),
            );
            Ok(())
        }
    }

    /// A report whose only section is named, so the order writes landed in is readable.
    fn report_titled(title: &str) -> Report {
        Report {
            subject: "basecamp.com".to_owned(),
            searched_as: "https://basecamp.com".to_owned(),
            generated_at: chrono::Utc::now(),
            model_id: "test".to_owned(),
            prompt_version: 1,
            sections: vec![Section::not_found("pricing", title, Vec::new())],
            sources: Vec::new(),
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
            .expect("something to claim");
        queued.id
    }

    #[tokio::test]
    async fn a_slow_write_never_overwrites_a_newer_report() {
        // The defect, stated as a test. Two snapshots, the first one slow. Written with a
        // task per snapshot, "first" lands last and the store keeps the older report — which
        // a reader sees as a section losing a claim it had already been shown.
        let slow = Arc::new(SlowFirstWrite::new());
        let store: Arc<dyn Store> = Arc::clone(&slow) as Arc<dyn Store>;
        let id = running_analysis(&store).await;

        let progress = Progress::new(Arc::clone(&store), id);
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
        let id = running_analysis(&store).await;

        let progress = Progress::new(Arc::clone(&store), id);
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
        let id = running_analysis(&store).await;

        let progress = Progress::new(Arc::clone(&store), id);
        progress.record(&report_titled("only"));
        progress.finish().await;

        assert_eq!(
            slow.landed(),
            vec!["only".to_owned()],
            "finish returned before the write it was waiting for"
        );
    }

    #[tokio::test]
    async fn recording_after_the_run_is_over_does_not_panic() {
        // `record` is called from a synchronous callback deep inside the pipeline, where
        // there is nothing sensible to do with an error. It must be impossible to fail.
        let store: Arc<dyn Store> = Arc::new(MemoryStore::new());
        let id = running_analysis(&store).await;
        let progress = Progress::new(Arc::clone(&store), id);
        progress.writer.abort();
        progress.record(&report_titled("after"));
    }
}
