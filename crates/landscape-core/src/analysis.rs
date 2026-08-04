//! An analysis: one prompt, its lifecycle, and the report it produces.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{CoreError, Result};

/// Shortest prompt we will accept. Below this there is nothing to resolve a category from.
pub const MIN_PROMPT: usize = 8;
/// Longest prompt we will accept. Guards the queue, not the model.
pub const MAX_PROMPT: usize = 2_000;

/// Identifier for one analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct AnalysisId(pub Uuid);

impl AnalysisId {
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for AnalysisId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for AnalysisId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Where an analysis has got to.
///
/// `Failed` carries no detail here on purpose: what a user is told about a failure is a
/// presentation decision made at the boundary, not a database field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisStatus {
    Queued,
    Running,
    Complete,
    Failed,
}

impl AnalysisStatus {
    /// Whether this is an end state. A terminal analysis is never claimed by a worker.
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Complete | Self::Failed)
    }

    /// The string stored in Postgres. Kept explicit rather than derived so a rename in
    /// Rust cannot silently orphan rows already written.
    #[must_use]
    pub const fn as_db_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Complete => "complete",
            Self::Failed => "failed",
        }
    }

    /// Parse a value read back from Postgres.
    #[must_use]
    pub fn from_db_str(s: &str) -> Option<Self> {
        match s {
            "queued" => Some(Self::Queued),
            "running" => Some(Self::Running),
            "complete" => Some(Self::Complete),
            "failed" => Some(Self::Failed),
            _ => None,
        }
    }
}

/// Why an analysis failed, in the only terms a reader can act on.
///
/// **Not the operator's reason.** `migrations/0001_init.sql` is explicit that
/// `failure_reason` is recorded for operators and never shown verbatim, because what somebody
/// is told about a failure is a presentation decision rather than a database field. This is
/// the other half of that rule: a small, closed set of *situations*, so the interface can
/// write a sentence for each and nothing internal leaks into it.
///
/// The distinction it exists for: one of these is our fault and one is the reader's to fix,
/// and telling somebody *"nothing you did caused it"* when they typed a description we cannot
/// resolve sends them away with no way forward.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Failure {
    /// The prompt named no company we could identify. **The reader can fix this.**
    NoSubject,
    /// Anything else. The reader can only try again.
    Internal,
}

impl Failure {
    /// The string stored in Postgres, explicit for the same reason [`AnalysisStatus`]'s is.
    #[must_use]
    pub const fn as_db_str(self) -> &'static str {
        match self {
            Self::NoSubject => "no_subject",
            Self::Internal => "internal",
        }
    }

    /// Parse a value read back from Postgres.
    ///
    /// An unrecognised kind reads as [`Failure::Internal`] rather than nothing: a row written
    /// by a newer version must not make an older one claim the analysis succeeded.
    #[must_use]
    pub fn from_db_str(s: &str) -> Self {
        match s {
            "no_subject" => Self::NoSubject,
            _ => Self::Internal,
        }
    }
}

/// A validated request to analyse something.
///
/// The only way to build one is [`NewAnalysis::parse`], so an unvalidated prompt cannot
/// reach the queue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewAnalysis {
    prompt: String,
}

impl NewAnalysis {
    /// Validate raw user input.
    ///
    /// Trims surrounding whitespace first: a prompt that is only spaces is empty, not
    /// short, and the message should say so honestly.
    pub fn parse(raw: &str) -> Result<Self> {
        let prompt = raw.trim();
        let len = prompt.chars().count();
        if len < MIN_PROMPT {
            return Err(CoreError::PromptTooShort {
                min: MIN_PROMPT,
                got: len,
            });
        }
        if len > MAX_PROMPT {
            return Err(CoreError::PromptTooLong {
                max: MAX_PROMPT,
                got: len,
            });
        }
        Ok(Self {
            prompt: prompt.to_owned(),
        })
    }

    #[must_use]
    pub fn prompt(&self) -> &str {
        &self.prompt
    }
}

/// An analysis as stored and returned.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Analysis {
    pub id: AnalysisId,
    pub prompt: String,
    pub status: AnalysisStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Present once the run finishes. Absent while queued or running, and on failure.
    pub report: Option<crate::report::Report>,
    /// Which *situation* a failed analysis is in, so the interface can say something useful.
    /// `None` unless the status is `Failed`.
    pub failure: Option<Failure>,
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
// Panicking IS how a test reports failure. The lints stay denied everywhere else.
mod tests {
    use super::*;

    #[test]
    fn a_failure_kind_survives_a_round_trip_through_the_database() {
        for f in [Failure::NoSubject, Failure::Internal] {
            assert_eq!(Failure::from_db_str(f.as_db_str()), f);
        }
    }

    #[test]
    fn an_unknown_failure_kind_reads_as_internal() {
        // A row written by a newer version must not make an older one claim success.
        assert_eq!(Failure::from_db_str("something_new"), Failure::Internal);
    }

    #[test]
    fn a_reasonable_prompt_is_accepted() {
        let n = NewAnalysis::parse("an app that helps small farms sell to restaurants")
            .expect("valid prompt");
        assert!(n.prompt().starts_with("an app"));
    }

    #[test]
    fn surrounding_whitespace_is_trimmed_before_measuring() {
        let n = NewAnalysis::parse("   a tool for chasing invoices   ").expect("valid prompt");
        assert_eq!(n.prompt(), "a tool for chasing invoices");
    }

    #[test]
    fn whitespace_only_is_too_short_not_accepted_as_padding() {
        let err = NewAnalysis::parse("            ").expect_err("should reject");
        assert!(matches!(err, CoreError::PromptTooShort { got: 0, .. }));
    }

    #[test]
    fn a_short_prompt_is_rejected_with_both_numbers() {
        let err = NewAnalysis::parse("a crm").expect_err("should reject");
        match err {
            CoreError::PromptTooShort { min, got } => {
                assert_eq!(min, MIN_PROMPT);
                assert_eq!(got, 5);
            }
            other => panic!("wrong error: {other:?}"),
        }
    }

    #[test]
    fn an_over_long_prompt_is_rejected() {
        let long = "a".repeat(MAX_PROMPT + 1);
        assert!(matches!(
            NewAnalysis::parse(&long),
            Err(CoreError::PromptTooLong { .. })
        ));
    }

    #[test]
    fn length_is_measured_in_characters_not_bytes() {
        // Eight accented characters are 16 bytes but 8 characters. Measuring bytes would
        // accept this and reject its ASCII equivalent, which is the wrong way round.
        let eight_chars = "ééééééééé";
        assert!(
            eight_chars.len() > MIN_PROMPT,
            "precondition: more bytes than chars"
        );
        assert!(NewAnalysis::parse(eight_chars).is_ok());
    }

    #[test]
    fn status_survives_a_database_round_trip() {
        for s in [
            AnalysisStatus::Queued,
            AnalysisStatus::Running,
            AnalysisStatus::Complete,
            AnalysisStatus::Failed,
        ] {
            assert_eq!(AnalysisStatus::from_db_str(s.as_db_str()), Some(s));
        }
        assert_eq!(AnalysisStatus::from_db_str("nonsense"), None);
    }

    #[test]
    fn only_finished_states_are_terminal() {
        assert!(!AnalysisStatus::Queued.is_terminal());
        assert!(!AnalysisStatus::Running.is_terminal());
        assert!(AnalysisStatus::Complete.is_terminal());
        assert!(AnalysisStatus::Failed.is_terminal());
    }
}
