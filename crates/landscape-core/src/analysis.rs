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
/// The distinction it exists for: some of these are ours and some are the reader's to fix, and
/// telling somebody *"nothing you did caused it"* when they typed a description we cannot
/// resolve sends them away with no way forward.
///
/// **It was two values, and the analysis had five answers.** Every refusal
/// `landscape_analyze::subject::decide` produces arrived as `NoSubject` and rendered as one
/// sentence — *"we could not work out which company you meant; try naming its website"*. For a
/// search that timed out that is a wrong instruction, and for a name several products share it
/// throws away the question a reader could have answered in a word. The analysis had spent four
/// changes keeping those silences apart internally while the boundary collapsed them again.
///
/// **Closed on purpose, and small on purpose.** A situation earns a variant when the reader
/// would *do something different*; anything finer belongs in `failure_reason`, which is for
/// operators and is never shown verbatim.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Failure {
    /// Nothing in the prompt named a company, and nothing here can go looking.
    ///
    /// **The reader fixes this by naming a domain**, or an operator by configuring a search
    /// engine. It is the only one of these that is about our configuration.
    NoSubject,
    /// Several real companies match what was typed, and we will not choose between them.
    ///
    /// **A question, not a failure** — `PRODUCT_SPEC.md` §3's *"Which Notion do you mean?"*.
    /// The reader answers it in one word, which is why it cannot share a sentence with the
    /// three below: every one of those tells them to do something that would not help.
    Ambiguous,
    /// We searched, and found no company we could stand behind.
    ///
    /// About the world rather than about us. Rewording helps; naming a domain helps; waiting
    /// does not.
    NothingFound,
    /// The searching did not finish, so nothing was concluded.
    ///
    /// **The retryable one.** Telling somebody to name a domain because an engine timed out
    /// sends them to fix something that was never wrong.
    SearchIncomplete,
    /// The searching reached an engine, and the engine refused.
    ///
    /// **Every count is identical to [`Self::SearchIncomplete`] and the advice is the
    /// opposite.** An engine that answers with a refusal will answer the same way in a minute
    /// and in a week — a misconfigured SearXNG answers `403` to every query until somebody
    /// edits a file — so *"try again"* is an instruction to wait for something that cannot
    /// happen. A reader can still do the one thing that skips the engine entirely, which is
    /// name a domain, and that is what they are told.
    ///
    /// It is a separate kind rather than a longer sentence because the interface renders from
    /// the kind: `PRODUCT_SPEC.md` §3's rule that a situation earns a value when a reader
    /// would **do something different**, and here they would.
    SearchRefused,
    /// Anything else. The reader can only try again.
    Internal,
}

impl Failure {
    /// The string stored in Postgres, explicit for the same reason [`AnalysisStatus`]'s is.
    #[must_use]
    pub const fn as_db_str(self) -> &'static str {
        match self {
            Self::NoSubject => "no_subject",
            Self::Ambiguous => "ambiguous",
            Self::NothingFound => "nothing_found",
            Self::SearchIncomplete => "search_incomplete",
            Self::SearchRefused => "search_refused",
            Self::Internal => "internal",
        }
    }

    /// Parse a value read back from Postgres.
    ///
    /// An unrecognized kind reads as [`Failure::Internal`] rather than nothing: a row written
    /// by a newer version must not make an older one claim the analysis succeeded.
    #[must_use]
    pub fn from_db_str(s: &str) -> Self {
        match s {
            "no_subject" => Self::NoSubject,
            "ambiguous" => Self::Ambiguous,
            "nothing_found" => Self::NothingFound,
            "search_incomplete" => Self::SearchIncomplete,
            "search_refused" => Self::SearchRefused,
            _ => Self::Internal,
        }
    }
}

/// One company a reader can pick, and the words that pick it.
///
/// **A question is only worth asking if answering it is cheap.** `PRODUCT_SPEC.md` §3 puts the
/// cost at *one chip click*, and the reason this carries a whole [`Self::prompt`] rather than a
/// domain is that the click has to be the entire answer: a reader who has to retype their idea
/// with a company name bolted on has been asked to do the work themselves.
///
/// **Named from the company's own front page**, like everything else a reader chooses between —
/// see `landscape_search::candidates::describe`. Choosing between three summaries written by a
/// search engine is choosing between an engine's opinions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Choice {
    /// What the company calls itself.
    pub name: String,
    /// Its canonical domain, shown so a reader can tell two same-named products apart.
    pub domain: String,
    /// The one line under its own heading. Empty when its page said nothing quotable.
    pub what_it_is: String,
    /// What to run instead. Sent verbatim as a new analysis.
    pub prompt: String,
}

/// A validated request to analyze something.
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
    /// The companies a reader can pick between, when the run stopped because it would not guess.
    ///
    /// Empty unless [`Self::failure`] is [`Failure::Ambiguous`]. **This is the question itself**,
    /// not decoration on a refusal: without it the interface can say *"that name matches more
    /// than one company"* and cannot say which, so the reader is left to guess what we would not.
    #[serde(default)]
    pub choices: Vec<Choice>,
    /// How many times this run has been started.
    ///
    /// **A claim is a number, not a state.** `status` cannot tell two workers apart — a slow
    /// one and the replacement handed its row by the staleness sweep both see `running` — so
    /// every write carries the generation it was claimed under, and one quoting an older number
    /// is refused. See [`Applied`].
    ///
    /// It also goes out on the stream, which is the only way a client that reconnects can know
    /// the sections it is holding belong to a run that no longer exists: a fresh connection has
    /// no memory of the one it replaced.
    ///
    /// Not called `attempt` because [`Attempt`](crate::Attempt) is already a fetch of one URL,
    /// and not `claim` because [`Claim`](crate::Claim) is a sentence a reader is shown.
    ///
    /// `0` means nothing has claimed it yet.
    pub generation: u32,
}

/// Whether a write was applied, or refused because the claim behind it had been revoked.
///
/// Returned rather than logged inside the store: the worker is the only caller, and it is the
/// one that needs to know its work is being discarded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Applied {
    Yes,
    /// The row has moved on to a later attempt. Nothing was written.
    ClaimRevoked,
}

impl Applied {
    /// Whether the write took effect.
    #[must_use]
    pub const fn took_effect(self) -> bool {
        matches!(self, Self::Yes)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
// Panicking IS how a test reports failure. The lints stay denied everywhere else.
mod tests {

    #[test]
    fn every_situation_survives_the_database_and_back() {
        // **A row written by a newer version must not make an older one claim success**, which
        // is why `from_db_str` falls back to `Internal` - and why that fallback must never
        // swallow a value this version writes.
        for kind in [
            Failure::NoSubject,
            Failure::Ambiguous,
            Failure::NothingFound,
            Failure::SearchIncomplete,
            Failure::Internal,
        ] {
            assert_eq!(Failure::from_db_str(kind.as_db_str()), kind);
        }
        assert_eq!(Failure::from_db_str("something_later"), Failure::Internal);
    }

    #[test]
    fn no_two_situations_share_a_stored_value() {
        let stored: Vec<&str> = [
            Failure::NoSubject,
            Failure::Ambiguous,
            Failure::NothingFound,
            Failure::SearchIncomplete,
            Failure::Internal,
        ]
        .iter()
        .map(|k| k.as_db_str())
        .collect();
        let mut unique = stored.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), stored.len(), "{stored:?}");
    }

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
