//! Source dispositions — the five-way classification from `docs/FACT_CHECKING.md` §3.2.1.
//!
//! The governing rule from that document, encoded here so it cannot drift: **a disposition
//! records what we were able to confirm, not a judgement about the publisher.** Every
//! description below is written with us as the subject.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// What we were able to establish about a page we read.
///
/// Ordering is deliberate: `Primary` is the most authoritative and `NotRead` the least
/// informative, so `Ord` sorts a source list into a sensible presentation order.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Disposition {
    /// The subject's own pages, or a regulator's. Sets the authoritative values.
    Primary,
    /// An established reference or data service — not the company, but maintained and dated.
    Supplementary,
    /// An independent page where we confirmed authorship, dating, publisher and sourcing.
    Attributed,
    /// A page we could read that shows nothing troubling, but which we could not fully
    /// attribute. Included by default and labelled — see FACT_CHECKING.md §3.2.1.
    Unverified,
    /// It states something the subject's own current page contradicts. Both are shown,
    /// neither is adjudicated.
    NotReconciled,
    /// We were not able or permitted to read it. Listed with a link so the reader can.
    NotRead,
}

impl Disposition {
    /// The short label used in reports (`P`, `S`, `A`, `U`, `N`, `R`).
    #[must_use]
    pub const fn code(self) -> char {
        match self {
            Self::Primary => 'P',
            Self::Supplementary => 'S',
            Self::Attributed => 'A',
            Self::Unverified => 'U',
            Self::NotReconciled => 'N',
            Self::NotRead => 'R',
        }
    }

    /// Whether a value from this source may set a figure in a comparison table.
    ///
    /// Only the subject's own statement does. Everything else is reported alongside the
    /// table with its provenance, never inside it.
    #[must_use]
    pub const fn may_set_a_table_value(self) -> bool {
        matches!(self, Self::Primary)
    }

    /// How we describe this disposition to a reader.
    ///
    /// The subject of every sentence is us. We never say a page is unreliable or wrong —
    /// see FACT_CHECKING.md §3.2.5 for why the phrasing is constrained rather than free.
    #[must_use]
    pub const fn reader_description(self) -> &'static str {
        match self {
            Self::Primary => "the company's own page",
            Self::Supplementary => "a reference service, named and linked",
            Self::Attributed => "an independent page whose author and date we confirmed",
            Self::Unverified => "a page we could read but could not fully attribute",
            Self::NotReconciled => "states something the company's own page contradicts",
            Self::NotRead => "we were not able to read it — you can",
        }
    }
}

/// A page we looked at, and how far we got with it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Source {
    /// Reference label as it appears in the report: `S1`, `S2`, …
    pub label: String,
    pub url: String,
    pub title: String,
    pub disposition: Disposition,
    /// When we read it. Every claim carries this so a reader can judge staleness.
    pub fetched_at: chrono::DateTime<chrono::Utc>,
    /// Groups sources that are not independent of each other — a syndicated post and its
    /// original share a group, so three copies of one claim are never counted as three.
    pub independence_group: String,
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
// Panicking IS how a test reports failure. The lints stay denied everywhere else.
mod tests {
    use super::*;

    #[test]
    fn only_primary_sources_set_table_values() {
        assert!(Disposition::Primary.may_set_a_table_value());
        for d in [
            Disposition::Supplementary,
            Disposition::Attributed,
            Disposition::Unverified,
            Disposition::NotReconciled,
            Disposition::NotRead,
        ] {
            assert!(
                !d.may_set_a_table_value(),
                "{d:?} must not be allowed to set a table value"
            );
        }
    }

    #[test]
    fn codes_are_distinct() {
        let all = [
            Disposition::Primary,
            Disposition::Supplementary,
            Disposition::Attributed,
            Disposition::Unverified,
            Disposition::NotReconciled,
            Disposition::NotRead,
        ];
        let mut codes: Vec<char> = all.iter().map(|d| d.code()).collect();
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes.len(), all.len(), "two dispositions share a code");
    }

    #[test]
    fn no_reader_description_judges_the_publisher() {
        // FACT_CHECKING.md §3.2.5: we say what we confirmed, never that a site is bad.
        const FORBIDDEN: [&str; 6] = [
            "unreliable",
            "untrustworthy",
            "wrong",
            "bad",
            "poor",
            "fake",
        ];
        for d in [
            Disposition::Primary,
            Disposition::Supplementary,
            Disposition::Attributed,
            Disposition::Unverified,
            Disposition::NotReconciled,
            Disposition::NotRead,
        ] {
            let text = d.reader_description().to_lowercase();
            for word in FORBIDDEN {
                assert!(!text.contains(word), "{d:?} description uses \"{word}\"");
            }
        }
    }

    #[test]
    fn dispositions_round_trip_as_snake_case() {
        let json = serde_json::to_string(&Disposition::NotReconciled).expect("serialise");
        assert_eq!(json, "\"not_reconciled\"");
        let back: Disposition = serde_json::from_str(&json).expect("deserialise");
        assert_eq!(back, Disposition::NotReconciled);
    }
}
