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
use landscape_core::{Analysis, AnalysisId, AnalysisStatus, NewAnalysis, Report};
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
    /// Returns `None` when the queue is empty. Two workers calling this concurrently must
    /// never receive the same analysis — the Postgres implementation relies on
    /// `FOR UPDATE SKIP LOCKED` for that, which is the reason the queue lives in the
    /// database rather than in a separate broker.
    async fn claim_next(&self) -> Result<Option<Analysis>>;

    /// Attach a finished report and mark the analysis complete.
    async fn complete(&self, id: AnalysisId, report: &Report) -> Result<()>;

    /// Mark an analysis failed. The reason is recorded for operators, not for readers.
    async fn fail(&self, id: AnalysisId, reason: &str) -> Result<()>;

    /// Count analyses in a given state. Used by the health endpoint and by tests.
    async fn count_with_status(&self, status: AnalysisStatus) -> Result<i64>;
}
