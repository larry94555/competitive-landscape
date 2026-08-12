//! What the reader gave, and how much of the searching finished.
//!
//! **Two facts the page needs and the finished report did not carry.**
//! [`PRODUCT_IDEA_RESULTS.md`] §4.5 is the accounting: `subjects_in` runs inside the worker and
//! only the prompt survives, and the failed queries were logged and dropped. So a page trying to
//! say *"I found 12 companies"* honestly had two ways to get it wrong and no way to get it
//! right.
//!
//! # Why not derive it in the browser
//!
//! Re-running `subjects_in` in TypeScript would put one business rule in two languages, which is
//! the shape this repository has a register entry about. And no amount of client-side parsing
//! recovers a query that failed inside a worker an hour ago: that fact exists for one moment,
//! in one process, and either it is written down then or it is gone.
//!
//! [`PRODUCT_IDEA_RESULTS.md`]: ../../../docs/PRODUCT_IDEA_RESULTS.md

use serde::{Deserialize, Serialize};

/// What class of thing the reader gave, and what follows from it.
///
/// Named `Given` rather than `Asked` because `landscape_analyze::Asked` is the bag of arguments
/// a run is started with, and two types with one name a crate apart is a re-reading tax on
/// everybody who comes after.
///
/// **"Found" is a claim about provenance.** `landscape_analyze::subject::Subjects` already
/// separates these three inside the worker, and the difference decides both what the page may
/// say and what order the companies are in: a list somebody wrote is an instruction, and a list
/// discovered from a description is a ranking.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum Given {
    /// A description. Every company in the report was discovered.
    Described,
    /// One company, whose rivals were searched for. The named one was not discovered.
    Seeded {
        /// As the reader wrote it.
        named: String,
    },
    /// Companies named outright. **Nothing was discovered at all**, and the order is theirs.
    Named {
        /// How many they named.
        count: usize,
    },
}

impl Given {
    /// Whether the order of the companies is the reader's instruction rather than our ranking.
    ///
    /// `Subjects::Exactly` is documented as *"exactly these, in the order written"*. Somebody
    /// comparing `basecamp.com vs linear.app` has put the one they care about first, and
    /// re-scoring that list would overrule them in the one place they were most explicit.
    #[must_use]
    pub const fn order_is_theirs(&self) -> bool {
        matches!(self, Self::Named { .. })
    }
}

/// How much of the searching behind a set actually finished.
///
/// **A count over an unfinished search is a definite number about an indefinite thing.** The
/// thirteenth company may not exist, or may be behind the query that timed out, and a bare
/// *"12 companies"* gives a reader no way to tell which. `landscape_search::candidates::Queried`
/// has held both halves all along; this is them surviving as far as the reader.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Searches {
    /// Asked and answered.
    pub answered: usize,
    /// Asked and did not come back.
    pub failed: usize,
}

impl Searches {
    #[must_use]
    pub const fn new(answered: usize, failed: usize) -> Self {
        Self { answered, failed }
    }

    /// Every search that was sent, whatever became of it.
    #[must_use]
    pub const fn sent(&self) -> usize {
        self.answered + self.failed
    }

    /// Whether a count taken from these searches may be stated as a definite number.
    ///
    /// **Nothing sent is not the same as everything answered.** A run that asked no questions
    /// has not established an absence, so it is not complete either - `NoRivals::NoEngine` is
    /// exactly that case, and it is why this is not `failed == 0`.
    #[must_use]
    pub const fn finished(&self) -> bool {
        self.failed == 0 && self.answered > 0
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::panic)]
    // Panicking IS how a test reports failure. Both lints stay denied everywhere else.

    use super::*;

    #[test]
    fn only_a_named_set_carries_the_readers_own_order() {
        assert!(Given::Named { count: 3 }.order_is_theirs());
        assert!(!Given::Described.order_is_theirs());
        assert!(
            !Given::Seeded {
                named: "basecamp.com".to_owned()
            }
            .order_is_theirs(),
            "a seed's rivals are ranked by us, so the order is ours to choose"
        );
    }

    #[test]
    fn a_search_that_asked_nothing_has_not_finished() {
        // **The case `failed == 0` gets wrong.** No engine configured means no query was sent
        // and nothing was established; calling that complete would let the page state a
        // definite zero about a search that never happened.
        assert!(!Searches::new(0, 0).finished());
        assert!(Searches::new(8, 0).finished());
        assert!(!Searches::new(6, 2).finished());
    }

    #[test]
    fn sent_is_both_halves() {
        assert_eq!(Searches::new(6, 2).sent(), 8);
        assert_eq!(Searches::new(0, 0).sent(), 0);
    }

    #[test]
    fn the_class_survives_a_round_trip() {
        // It travels as JSON to a browser that decides three different sentences from it, so
        // the tag has to be there and has to be stable.
        for given in [
            Given::Described,
            Given::Seeded {
                named: "basecamp.com".to_owned(),
            },
            Given::Named { count: 3 },
        ] {
            let json = serde_json::to_string(&given).expect("serializes");
            let back: Given = serde_json::from_str(&json).expect("round trips");
            assert_eq!(back, given, "{json}");
            assert!(json.contains("\"kind\""), "{json}");
        }
    }
}
