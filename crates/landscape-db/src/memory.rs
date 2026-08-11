//! An in-memory [`Store`](crate::Store).
//!
//! Not a mock: it is a complete implementation with the same observable behavior as
//! Postgres, including FIFO claim ordering and single-claim guarantees. That is what lets
//! the API's tests run with nothing installed while still testing the real request path.

use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use landscape_core::{Analysis, AnalysisId, AnalysisStatus, Applied, NewAnalysis, Report};

use crate::{Refused, Result, Store, StoreError};

#[derive(Debug, Default)]
pub struct MemoryStore {
    // One lock over the whole map. Contention does not matter here: this exists for tests
    // and for `--store memory`, never under real load.
    inner: Mutex<HashMap<AnalysisId, Analysis>>,
}

impl MemoryStore {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Take the lock, treating poisoning as recoverable.
    ///
    /// A panic in another test holding this lock should not cascade into unrelated
    /// failures, and the map's invariants do not depend on the panicking section having
    /// completed.
    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<AnalysisId, Analysis>> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }
}

#[async_trait]
impl Store for MemoryStore {
    async fn enqueue(&self, new: &NewAnalysis) -> Result<Analysis> {
        let analysis = Analysis {
            id: AnalysisId::new(),
            prompt: new.prompt().to_owned(),
            status: AnalysisStatus::Queued,
            created_at: chrono::Utc::now(),
            report: None,
            failure: None,
            choices: Vec::new(),
            generation: 0,
        };
        self.lock().insert(analysis.id, analysis.clone());
        Ok(analysis)
    }

    async fn get(&self, id: AnalysisId) -> Result<Analysis> {
        self.lock()
            .get(&id)
            .cloned()
            .ok_or(StoreError::NotFound(id))
    }

    async fn claim_next(&self) -> Result<Option<Analysis>> {
        let mut map = self.lock();
        // Oldest first, matching the Postgres `ORDER BY created_at` — a worker that
        // claimed the newest would starve the queue under load.
        let next = map
            .values()
            .filter(|a| a.status == AnalysisStatus::Queued)
            .min_by_key(|a| (a.created_at, a.id.0))
            .map(|a| a.id);

        match next {
            None => Ok(None),
            Some(id) => {
                let entry = map.get_mut(&id).ok_or(StoreError::NotFound(id))?;
                entry.status = AnalysisStatus::Running;
                // The number the worker will quote on every write. Raised here and in
                // `reclaim_stale`, so it counts how many times this run has been started.
                entry.generation = entry.generation.saturating_add(1);
                Ok(Some(entry.clone()))
            }
        }
    }

    async fn save_progress(
        &self,
        id: AnalysisId,
        generation: u32,
        report: &Report,
    ) -> Result<Applied> {
        let mut map = self.lock();
        let analysis = map.get_mut(&id).ok_or(StoreError::NotFound(id))?;
        // Deliberately does not touch `status`. Progress on a run that has already finished
        // is a late write from a worker that lost a race, and it must not resurrect the row.
        if analysis.generation != generation || analysis.status != AnalysisStatus::Running {
            return Ok(Applied::ClaimRevoked);
        }
        analysis.report = Some(report.clone());
        Ok(Applied::Yes)
    }

    async fn complete(&self, id: AnalysisId, generation: u32, report: &Report) -> Result<Applied> {
        let mut map = self.lock();
        let entry = map.get_mut(&id).ok_or(StoreError::NotFound(id))?;
        if entry.generation != generation {
            return Ok(Applied::ClaimRevoked);
        }
        entry.status = AnalysisStatus::Complete;
        entry.report = Some(report.clone());
        Ok(Applied::Yes)
    }

    async fn fail(&self, id: AnalysisId, generation: u32, refused: Refused<'_>) -> Result<Applied> {
        let mut map = self.lock();
        let entry = map.get_mut(&id).ok_or(StoreError::NotFound(id))?;
        if entry.generation != generation {
            return Ok(Applied::ClaimRevoked);
        }
        entry.status = AnalysisStatus::Failed;
        entry.failure = Some(refused.kind);
        entry.choices = refused.choices.to_vec();
        tracing::warn!(
            %id,
            reason = refused.reason,
            kind = refused.kind.as_db_str(),
            choices = refused.choices.len(),
            "analysis failed"
        );
        Ok(Applied::Yes)
    }

    async fn reclaim_stale(&self, max_age: chrono::Duration) -> Result<u64> {
        let cutoff = chrono::Utc::now() - max_age;
        let mut map = self.lock();
        let mut n = 0_u64;
        for entry in map.values_mut() {
            // `started_at` is not tracked here, so `created_at` stands in. It is always
            // earlier, which makes this store slightly more eager to reclaim than Postgres
            // — erring toward retrying a job rather than stranding one.
            if entry.status == AnalysisStatus::Running && entry.created_at < cutoff {
                entry.status = AnalysisStatus::Queued;
                // The partial report goes with the worker that wrote it. Keeping it leaves a
                // queued row holding sections nobody stands behind, and the replacement run
                // starts from an empty report - so a reader watching sees answers they were
                // already shown blank themselves out, which reads as a retraction.
                entry.report = None;
                // Raised here as well as on claim, so the worker that was running it is
                // holding a revoked number the moment the sweep passes - not only once a
                // replacement has picked the row up.
                entry.generation = entry.generation.saturating_add(1);
                n += 1;
            }
        }
        Ok(n)
    }

    async fn count_with_status(&self, status: AnalysisStatus) -> Result<i64> {
        let n = self.lock().values().filter(|a| a.status == status).count();
        Ok(i64::try_from(n).unwrap_or(i64::MAX))
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
// Panicking IS how a test reports failure. The lints stay denied everywhere else.
mod tests {
    use super::*;

    #[tokio::test]
    async fn memory_store_satisfies_the_contract() {
        let store = MemoryStore::new();
        crate::conformance::run(&store).await;
    }
}
