//! Storage for Landscape.
//!
//! The [`Store`] trait is the seam that keeps the rest of the application testable with
//! nothing installed. [`MemoryStore`] is a complete implementation used by the API's own
//! tests; [`PgStore`] is the real one. Anything that needs storage takes a `&dyn Store`,
//! so no test needs a database to exercise a request path.
//!
//! That is a deliberate cost: two implementations must agree. [`conformance`] is one test
//! body run against both, so they cannot drift apart quietly.

pub mod conformance;
mod memory;
mod pg;

pub use memory::MemoryStore;
pub use pg::PgStore;

use async_trait::async_trait;
use landscape_core::{Analysis, AnalysisId, AnalysisStatus, Applied, Failure, NewAnalysis, Report};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("no analysis with id {0}")]
    NotFound(AnalysisId),

    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("stored data could not be read back: {0}")]
    Corrupt(String),
}

pub type Result<T> = std::result::Result<T, StoreError>;

/// Everything the application needs from storage.
///
/// Deliberately narrow. A wide trait is expensive to implement twice, and the second
/// implementation is what makes local testing free.
#[async_trait]
pub trait Store: Send + Sync + std::fmt::Debug {
    /// Queue a new analysis and return it in its initial state.
    async fn enqueue(&self, new: &NewAnalysis) -> Result<Analysis>;

    /// Fetch one analysis by id.
    async fn get(&self, id: AnalysisId) -> Result<Analysis>;

    /// Claim the oldest queued analysis and mark it running, atomically.
    ///
    /// **Raises the row's generation**, and the returned [`Analysis::generation`] is what the
    /// worker must quote on every write it makes. That is what makes a claim revocable: the
    /// sweep raises it too, so a worker that has been replaced is holding a number the row has
    /// moved past.
    ///
    /// Returns `None` when the queue is empty. Two workers calling this concurrently must
    /// never receive the same analysis — the Postgres implementation relies on
    /// `FOR UPDATE SKIP LOCKED` for that, which is the reason the queue lives in the
    /// database rather than in a separate broker.
    async fn claim_next(&self) -> Result<Option<Analysis>>;

    /// Attach the report so far, leaving the analysis running.
    ///
    /// **This is what makes streaming possible without a second piece of infrastructure.**
    /// A report is assembled a section at a time, and a reader waiting ninety seconds should
    /// see the pricing section as soon as pricing is done rather than everything at the end
    /// — `PRODUCT_SPEC.md` §2.1A. The worker writes progress here; the API reads it and
    /// streams the difference.
    ///
    /// The queue already lives in the database for the same reason a broker does not: a
    /// second thing to run is a second thing to fail, and at this scale the row is enough.
    ///
    /// Refused when `generation` is not the row's current one — a worker whose claim has been
    /// revoked must not write over the run that replaced it.
    async fn save_progress(
        &self,
        id: AnalysisId,
        generation: u32,
        report: &Report,
    ) -> Result<Applied>;

    /// Attach a finished report and mark the analysis complete.
    ///
    /// Refused when `generation` is not the row's current one. **This is the case `status`
    /// cannot express**: a worker slower than the staleness threshold and the replacement the
    /// sweep handed its row to both see `running`, and without a number to compare, whichever
    /// finished last won.
    async fn complete(&self, id: AnalysisId, generation: u32, report: &Report) -> Result<Applied>;

    /// Mark an analysis failed.
    ///
    /// Two arguments because they have two audiences. `reason` is for operators and is never
    /// shown verbatim — `migrations/0001_init.sql` says so. `kind` is the *situation*, from a
    /// closed set, so the interface can write a sentence a reader can act on without
    /// anything internal leaking into it.
    async fn fail(
        &self,
        id: AnalysisId,
        generation: u32,
        kind: Failure,
        reason: &str,
    ) -> Result<Applied>;

    /// Return analyses that have been `running` longer than `max_age` to the queue.
    ///
    /// A worker that is killed mid-analysis leaves its row `running` with nothing to
    /// finish it. Without this the row is stranded permanently: the user watches a
    /// spinner that will never resolve, and no error is ever recorded because nothing
    /// failed — the process simply stopped existing.
    ///
    /// Returns how many were reclaimed. Deliberately time-based rather than
    /// heartbeat-based: a heartbeat is a second thing that can fail, and at this scale
    /// "it has been running implausibly long" is the same signal for a tenth of the
    /// machinery.
    async fn reclaim_stale(&self, max_age: chrono::Duration) -> Result<u64>;

    /// Count analyses in a given state. Used by the health endpoint and by tests.
    async fn count_with_status(&self, status: AnalysisStatus) -> Result<i64>;
}
