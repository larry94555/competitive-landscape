//! SearXNG — the first provider.
//!
//! `ARCHITECTURE.md` §5.1 chose it for a reason that is not about quality: **zero marginal
//! cost and no vendor lock**. It is a metasearch front end we host, so asking it a thousand
//! questions costs a thousand questions' worth of nothing, and the day it stops working the
//! trait above it means the replacement is a file rather than a refactor. A paid Brave
//! Search key is the documented fallback and is deliberately not built yet — a fallback
//! written before the primary has ever run is a guess about how the primary fails.
//!
//! # Why this does not go through [`landscape_fetch`]
//!
//! That crate is for **reading strangers' websites**: it honours `robots.txt`, waits a
//! second per host, and refuses private address ranges. Every one of those is wrong here.
//! SearXNG ships a `robots.txt` that disallows everything — correctly, it does not want to
//! be crawled — so the polite fetcher would refuse our own service. It runs on
//! `127.0.0.1:8888` on a laptop and inside the compose network on the box, which the SSRF
//! guard exists to forbid. **This is infrastructure we operate, not a page we found**, and
//! the two need different clients. The distinction is worth keeping sharp: the moment a
//! URL comes *out* of here it is a stranger's again, and [`crate::admit`] treats it as one.
//!
//! # Enabling the JSON format
//!
//! SearXNG serves HTML by default and returns **403** for `format=json` until the instance
//! opts in:
//!
//! ```yaml
//! # searxng/settings.yml
//! search:
//!   formats:
//!     - html
//!     - json
//! ```
//!
//! [`SearchError::Status`] carries the number rather than flattening it, because a 403 here
//! means *that block was never uncommented* and a 429 means *slow down* — two different
//! jobs for whoever is reading the message.

use std::time::Duration;

use crate::provider::{Hit, SearchError, SourceProvider, HITS_PER_QUERY};
use crate::queries::Query;

/// The environment variable naming the instance.
pub const URL_VAR: &str = "SEARX_URL";

/// How long one query may take.
///
/// Short on purpose. A search is a round trip taken *before* any page has been read, inside
/// a 90–180 second budget that discovery has already spent fifteen seconds of. Waiting
/// thirty seconds for a metasearch engine to aggregate a slow upstream costs a reader more
/// than the section is worth.
pub const TIMEOUT: Duration = Duration::from_secs(8);

/// A SearXNG instance.
#[derive(Debug, Clone)]
pub struct Searx {
    base: String,
    http: reqwest::Client,
}

impl Searx {
    /// Point at an instance.
    ///
    /// The trailing slash is normalised here rather than being a rule people have to
    /// remember when they set the variable.
    #[must_use]
    pub fn new(base: &str) -> Self {
        Self {
            base: base.trim().trim_end_matches('/').to_owned(),
            http: reqwest::Client::builder()
                .timeout(TIMEOUT)
                .build()
                // `Client::new()` is what `build()` falls back to; the builder only fails on
                // a TLS backend that could not be initialised, and the default client has
                // the same backend. This keeps the crate free of `unwrap`.
                .unwrap_or_default(),
        }
    }

    /// The instance named by `SEARX_URL`, if there is one.
    ///
    /// Returns [`None`] rather than a default. A hard-coded fallback to a public instance
    /// would send every subject a stranger types into the box to a third party we do not
    /// run, which is the one thing the local-inference posture in `ROADMAP.md` §1 promises
    /// does not happen.
    #[must_use]
    pub fn from_env() -> Option<Self> {
        Self::configured(std::env::var(URL_VAR).ok().as_deref())
    }

    /// The decision [`Self::from_env`] makes, with the environment passed in.
    ///
    /// Split out so the rule can be tested without a test mutating the process's
    /// environment — a global that every other test in the binary shares, and one whose
    /// safety would rest on the runner happening to give each test its own process.
    #[must_use]
    pub fn configured(raw: Option<&str>) -> Option<Self> {
        let trimmed = raw?.trim();
        if trimmed.is_empty() {
            return None;
        }
        Some(Self::new(trimmed))
    }

    /// Where a query is sent. Separate so the test can read it without a server.
    #[must_use]
    pub fn endpoint(&self) -> String {
        format!("{}/search", self.base)
    }
}

/// The subset of SearXNG's JSON we read.
///
/// Deliberately partial: `serde` ignores what is not named, so an upstream that adds fields
/// does not break this, and one that *renames* `url` fails loudly at parse time rather than
/// returning an empty result set that reads as "nothing found".
#[derive(Debug, serde::Deserialize)]
struct Body {
    #[serde(default)]
    results: Vec<Row>,
}

#[derive(Debug, serde::Deserialize)]
struct Row {
    url: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    content: String,
}

/// Turn a response body into hits.
///
/// Split out from the request so the parse is testable against a frozen body with no server
/// anywhere — the same reason the golden set holds frozen pages.
///
/// # Errors
/// [`SearchError::Unreadable`] if the body is not the documented shape.
pub fn hits_from_json(body: &str) -> Result<Vec<Hit>, SearchError> {
    let parsed: Body =
        serde_json::from_str(body).map_err(|e| SearchError::Unreadable(e.to_string()))?;
    Ok(parsed
        .results
        .into_iter()
        // A row with no URL is not a page. SearXNG emits `infobox` and `suggestion` entries
        // in the same document, and an empty string here would become a candidate nothing
        // could fetch.
        .filter(|r| !r.url.trim().is_empty())
        .take(HITS_PER_QUERY)
        .map(|r| Hit {
            url: r.url.trim().to_owned(),
            title: r.title,
            snippet: r.content,
        })
        .collect())
}

#[async_trait::async_trait]
impl SourceProvider for Searx {
    fn name(&self) -> &str {
        "searxng"
    }

    async fn search(&self, query: &Query) -> Result<Vec<Hit>, SearchError> {
        let response = self
            .http
            .get(self.endpoint())
            // `query` percent-encodes, so a template containing quotes and colons arrives
            // as written rather than as a malformed URL.
            .query(&[
                ("q", query.text.as_str()),
                ("format", "json"),
                ("language", "en"),
            ])
            .send()
            .await
            .map_err(|e| SearchError::Unreachable(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            return Err(SearchError::Status {
                status: status.as_u16(),
            });
        }

        let body = response
            .text()
            .await
            .map_err(|e| SearchError::Unreadable(e.to_string()))?;
        let hits = hits_from_json(&body)?;
        tracing::debug!(
            engine = self.name(),
            question = query.answers.name(),
            hits = hits.len(),
            "searched"
        );
        Ok(hits)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    /// A real SearXNG response, trimmed to the fields this parses and one it does not.
    const FROZEN: &str = r#"{
      "query": "\"Linear\" changelog OR \"release notes\"",
      "number_of_results": 3,
      "results": [
        {
          "url": "https://linear.app/changelog",
          "title": "Changelog – Linear",
          "content": "Product updates and release notes from the Linear team.",
          "engine": "duckduckgo",
          "score": 4.5
        },
        {
          "url": "https://github.com/linear/linear/releases",
          "title": "Releases · linear/linear",
          "content": "Releases of the Linear SDK.",
          "engine": "google"
        }
      ],
      "suggestions": ["linear app release notes"]
    }"#;

    #[test]
    fn a_frozen_response_parses_into_the_pages_it_names() {
        let hits = hits_from_json(FROZEN).unwrap();
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].url, "https://linear.app/changelog");
        assert_eq!(hits[1].url, "https://github.com/linear/linear/releases");
        // The fields we do not model — `engine`, `score`, `suggestions` — are ignored rather
        // than being a parse failure, so an upstream that adds one does not break this.
        assert_eq!(hits[0].title, "Changelog – Linear");
    }

    #[test]
    fn a_row_with_no_url_is_not_a_page() {
        // SearXNG puts infoboxes and corrections in the same document. An empty URL would
        // become a candidate that nothing can fetch and that shows up as a broken citation.
        let body = r#"{"results":[{"url":"","title":"Linear"},{"url":"  ","title":"x"},
                       {"url":"https://linear.app/","title":"Linear"}]}"#;
        let hits = hits_from_json(body).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].url, "https://linear.app/");
    }

    #[test]
    fn an_engine_returning_a_hundred_results_still_costs_one_query() {
        // The cap is enforced where the results arrive, not where they are used, so a
        // misconfigured instance cannot decide how much work this process does.
        let rows: Vec<String> = (0..100)
            .map(|i| format!(r#"{{"url":"https://e.test/{i}","title":"{i}"}}"#))
            .collect();
        let body = format!(r#"{{"results":[{}]}}"#, rows.join(","));
        assert_eq!(hits_from_json(&body).unwrap().len(), HITS_PER_QUERY);
    }

    #[test]
    fn a_response_with_no_results_key_is_empty_rather_than_an_error() {
        // An instance that found nothing answers 200 with a document that has no `results`.
        // That is "nothing found", which the report already knows how to say.
        assert!(hits_from_json(r#"{"query":"x"}"#).unwrap().is_empty());
    }

    #[test]
    fn html_where_json_was_asked_for_is_a_named_failure() {
        // What an instance that has not enabled the JSON format actually returns once it
        // stops 403ing: a page. Parsing it as success would report zero hits for ever.
        let err = hits_from_json("<!DOCTYPE html><html><body>results</body></html>").unwrap_err();
        assert!(
            matches!(err, SearchError::Unreadable(_)),
            "{err:?} should say the body could not be read"
        );
    }

    #[test]
    fn a_renamed_url_field_fails_loudly_rather_than_finding_nothing() {
        // The failure this shape is chosen to avoid. If `url` became `link` upstream and
        // rows were skipped silently, every search would return nothing and look like a web
        // with no pages on it.
        let body = r#"{"results":[{"link":"https://linear.app/","title":"Linear"}]}"#;
        assert!(hits_from_json(body).is_err());
    }

    #[test]
    fn a_trailing_slash_on_the_configured_url_is_not_a_rule_to_remember() {
        assert_eq!(
            Searx::new("http://127.0.0.1:8888/").endpoint(),
            Searx::new("http://127.0.0.1:8888").endpoint()
        );
        assert_eq!(
            Searx::new("http://127.0.0.1:8888").endpoint(),
            "http://127.0.0.1:8888/search"
        );
    }

    #[test]
    fn no_instance_is_configured_rather_than_defaulting_to_somebody_elses() {
        // A default public instance would send every subject a stranger types to a third
        // party, which is the one thing the privacy posture promises does not happen. So an
        // unset variable must yield nothing rather than something.
        assert!(Searx::configured(None).is_none());
        assert!(
            Searx::configured(Some("   ")).is_none(),
            "a blank value is not a configured instance"
        );
        assert_eq!(
            Searx::configured(Some("http://127.0.0.1:8888"))
                .map(|s| s.endpoint())
                .as_deref(),
            Some("http://127.0.0.1:8888/search")
        );
    }
}
