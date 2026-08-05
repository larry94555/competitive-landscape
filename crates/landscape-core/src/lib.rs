//! Domain types for Landscape.
//!
//! This crate is deliberately pure: no database, no network, no clock reads except
//! where a caller passes one in. Everything here can be exercised by a unit test with
//! no services running, which is what keeps `cargo test` useful on a laptop with
//! nothing installed.
//!
//! The report schema lives here because it has to be generated from exactly one place:
//! `schemars` turns these types into the JSON Schema the frontend consumes and the
//! decoding grammar the model is constrained by. Two hand-maintained copies of a schema
//! diverge; one generated copy cannot.

pub mod analysis;
pub mod coverage;
pub mod extract;
pub mod report;
pub mod source;
pub mod subject;

pub use analysis::{Analysis, AnalysisId, AnalysisStatus, Applied, Failure, NewAnalysis};
pub use coverage::{Attempt, Coverage};
pub use extract::{
    BillingPeriod, Change, FeatureExtraction, IdentityExtraction, PageChanges, PageFeatures,
    PageIdentity, PagePricing, PricingExtraction, Stated,
};
pub use report::{Claim, Confidence, Report, Section, SectionStatus};
pub use source::{Disposition, Source};
pub use subject::{resolve, Candidate, Resolution};

use thiserror::Error;

/// Anything that can go wrong in the domain layer.
///
/// Deliberately small. Transport, database and model errors belong to the crates that
/// own those concerns and are mapped at the boundary.
#[derive(Debug, Error)]
pub enum CoreError {
    #[error("a prompt must contain at least {min} characters, got {got}")]
    PromptTooShort { min: usize, got: usize },

    #[error("a prompt may contain at most {max} characters, got {got}")]
    PromptTooLong { max: usize, got: usize },
}

/// Result alias for the domain layer.
pub type Result<T> = std::result::Result<T, CoreError>;
