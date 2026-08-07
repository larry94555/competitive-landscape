//! The seam: what a search engine is, from this codebase's side of it.
//!
//! `ARCHITECTURE.md` §5.1 puts every off-site source *"behind a `SourceProvider` trait so
//! providers can be added or disabled without touching the orchestrator"*. The trait is
//! here rather than arriving with the second provider, because the shape of an interface
//! written around one implementation is the shape of that implementation.
//!
//! # What the trait deliberately cannot express
//!
//! **A rank.** [`Hit`] has no score, no position, no relevance. An engine's ordering is
//! evidence about an engine, and a field for it is an invitation to let one page outrank
//! another on nothing. What decides a page's standing is [`crate::admit::disposition_for`],
//! from the host, in one place.

use crate::queries::Query;

/// One result, as the engine described it.
///
/// Every field here is **the engine's account of a page**, not the page. Nothing on this
/// type has been fetched, and the difference is the reason `title` and `snippet` stop at
/// [`crate::admit::Found`]: they exist so a person running the diagnostic can see what came
/// back, and they are not evidence of anything.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    pub url: String,
    /// The engine's title. Frequently the page's `<title>`; sometimes the engine's own
    /// summary; occasionally an aggregator's headline about the page.
    pub title: String,
    /// The engine's snippet. **Not a quote.** It is assembled from the page, from a cached
    /// older copy of the page, or from a description somebody else wrote about it.
    pub snippet: String,
}

/// The most hits a provider may return for one query. Re-exported so an implementation can
/// truncate at the source rather than handing back a hundred to be thrown away.
pub use crate::queries::HITS_PER_QUERY;

/// What went wrong asking.
///
/// A search failure is **not** an analysis failure. Every variant here means *this section
/// keeps the coverage note it already had*, which is the honest-gap treatment the report
/// already has for a 404 — a report that says what was checked is better than one that
/// stops.
#[derive(Debug, thiserror::Error)]
pub enum SearchError {
    /// No engine is configured. The common case on a laptop, and not an error state worth
    /// alarming about — it is the reason `landscape search` prints the queries anyway.
    #[error("no search engine is configured (set SEARX_URL)")]
    NotConfigured,
    /// The request did not complete.
    #[error("search request failed: {0}")]
    Unreachable(String),
    /// It completed and said something other than 200.
    #[error("search engine answered {status}")]
    Status { status: u16 },
    /// It answered 200 with something that is not the shape we parse.
    #[error("search engine answered with an unreadable body: {0}")]
    Unreadable(String),
}

/// A place that turns a [`Query`] into URLs.
#[async_trait::async_trait]
pub trait SourceProvider: Send + Sync {
    /// What to call this in a log line and in a coverage note.
    fn name(&self) -> &str;

    /// Ask, and return at most [`HITS_PER_QUERY`] results.
    ///
    /// # Errors
    /// [`SearchError`] — and every variant means *carry on without this section's extra
    /// pages*, never *fail the analysis*.
    async fn search(&self, query: &Query) -> Result<Vec<Hit>, SearchError>;
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::queries;
    use landscape_discover::probes::Answers;

    /// A provider that answers from a list, so the seam can be exercised without a network.
    struct Canned(Vec<Hit>);

    #[async_trait::async_trait]
    impl SourceProvider for Canned {
        fn name(&self) -> &str {
            "canned"
        }
        async fn search(&self, _query: &Query) -> Result<Vec<Hit>, SearchError> {
            Ok(self.0.clone())
        }
    }

    #[tokio::test]
    async fn a_provider_can_be_swapped_without_touching_a_caller() {
        // The point of the trait. If this test ever needs a network, the seam has leaked.
        let provider = Canned(vec![Hit {
            url: "https://linear.app/changelog".to_owned(),
            title: "Changelog".to_owned(),
            snippet: "Latest releases".to_owned(),
        }]);
        let queries = queries::for_questions("Linear", &[Answers::Changes]);
        let hits = provider.search(&queries[0]).await.unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(provider.name(), "canned");
    }

    #[test]
    fn a_missing_engine_is_a_named_state_rather_than_a_generic_failure() {
        // A laptop with no SEARX_URL is the ordinary case, and it should read as "nothing is
        // configured" rather than as "search is broken".
        let message = SearchError::NotConfigured.to_string();
        assert!(message.contains("SEARX_URL"), "{message}");
    }
}
