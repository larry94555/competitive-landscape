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

/// What a reader would **do** about a search that did not come back.
///
/// Three, because there are three answers and they are not interchangeable: *do not bother*,
/// *wait a moment*, and *try again*. Every variant of [`SearchError`] collapsed into one
/// sentence before this existed — *"that is usually temporary - try again"* — which is true of
/// exactly one of them and is an instruction to wait for ever for the others.
///
/// **The one that matters is the one this project has already documented as the first thing
/// that goes wrong.** `deploy/searxng/settings.yml` exists for a single reason: SearXNG serves
/// HTML and answers **403** to `format=json` until an instance opts in. So the most likely
/// first experience of a configured engine is every query refused, permanently, with a report
/// telling the reader it was probably temporary.
///
/// # The rule, which is HTTP's rather than a guess about SearXNG
///
/// **Did the engine answer at all?** Anything that came back is a decision — the same decision
/// will come back next time. Only silence, and the two answers that explicitly mean *later*,
/// are worth waiting on. Written this way so a provider nobody has built yet inherits it: no
/// variant of this enum is about SearXNG.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Fault {
    /// The engine answered, and what it said was no. Configuration; waiting changes nothing.
    Refused,
    /// The engine answered, and asked to be asked less often. Waiting is the whole fix.
    TooFast,
    /// The engine did not answer. The only one that is weather.
    Silent,
}

impl Fault {
    /// The one to act on when several queries failed differently.
    ///
    /// **Declaration order is precedence, and it runs most-actionable first.** A run that saw
    /// one refusal and two timeouts has an engine that refuses — the refusal is the fact a
    /// person can do something about, and the timeouts may well be the same engine declining
    /// more slowly. Telling somebody to wait when one query proved waiting is useless is the
    /// defect this type exists to stop, so the tie is broken towards the actionable answer.
    ///
    /// [`None`] when nothing failed, which is not the same as *nothing is wrong*: a caller
    /// with no failures has nothing to report and this says so rather than inventing a calm.
    #[must_use]
    pub fn worst_of(faults: impl IntoIterator<Item = Self>) -> Option<Self> {
        faults.into_iter().min()
    }

    /// What a **reader** is told to do about it, in a sentence that names nothing internal.
    ///
    /// A stranger reading a report cannot fix our search engine and should not be shown its
    /// status code. What they can be told is whether the thing they are looking at will be
    /// different if they ask again — which is the only part of this that is theirs. Operator
    /// detail lives in `landscape search`'s output and in the log line, where somebody who can
    /// act on it is reading. That division is `migrations/0001_init.sql`'s rule about
    /// `failure_reason`, applied to the other half of the same event.
    /// One word for a table or a log line, where a sentence does not fit.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Refused => "refused",
            Self::TooFast => "asked too fast",
            Self::Silent => "no answer",
        }
    }

    #[must_use]
    pub const fn advice(self) -> &'static str {
        match self {
            // `concat!` rather than a `\` continuation: `cargo fmt` joined one of these onto a
            // single line and kept the indentation *inside* the literal, so a reader was shown
            // a sentence with a gap in the middle of it. A test asserts the wording now, and
            // this shape cannot be reflowed into something else.
            Self::Refused => concat!(
                "Our search engine is refusing us, which is ours to fix - ",
                "asking again will not change it."
            ),
            Self::TooFast => "We asked it too quickly - trying again shortly should work.",
            Self::Silent => "That is usually temporary - try again.",
        }
    }
}

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
    /// It answered 200 and kept answering. See [`crate::searx::MAX_RESPONSE_BYTES`].
    #[error("search engine answered with more than {limit} bytes")]
    TooLarge { limit: usize },
    /// The engine is configured and could not be built. Distinct from [`Self::NotConfigured`]
    /// on purpose: *"you set the variable and it did not work"* and *"you did not set the
    /// variable"* send a reader to different places.
    #[error("search engine could not be initialised: {0}")]
    Unusable(String),
}

impl SearchError {
    /// Which of the three answers this one deserves.
    ///
    /// **The split is whether the engine spoke.** A status, an unreadable body, a body past
    /// the size limit and a client that would not build are all things it did — asking again
    /// gets the same one, so a reader waiting is a reader waiting for nothing. Only
    /// [`Self::Unreachable`], a server-side 5xx and an explicit 429 can be different in a
    /// minute.
    ///
    /// [`Self::NotConfigured`] is a refusal by this reading, and it is the least interesting
    /// case: the paths that can say *"no engine"* decide that before any query goes out, and
    /// say it in their own words.
    #[must_use]
    pub const fn fault(&self) -> Fault {
        match *self {
            // It answered. 429 is the one status that means *later* in so many words, and a
            // 5xx is the engine breaking rather than declining.
            Self::Status { status: 429 } => Fault::TooFast,
            Self::Status { status } if status >= 500 => Fault::Silent,
            Self::Status { .. }
            | Self::Unreadable(_)
            | Self::TooLarge { .. }
            | Self::Unusable(_)
            | Self::NotConfigured => Fault::Refused,
            // It did not.
            Self::Unreachable(_) => Fault::Silent,
        }
    }
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

    #[test]
    fn an_engine_that_answered_is_never_something_to_wait_for() {
        // **The rule, stated as a test rather than as a comment.** Anything the engine said is
        // a decision, and the same decision comes back next time - so every one of these is a
        // refusal, whatever the wording of the error. A 403 is the documented first-run state
        // of an unconfigured SearXNG, and it used to be reported as weather.
        for answered in [
            SearchError::Status { status: 403 },
            SearchError::Status { status: 401 },
            SearchError::Status { status: 404 },
            SearchError::Status { status: 400 },
            SearchError::Unreadable("html, not json".to_owned()),
            SearchError::TooLarge { limit: 1 },
            SearchError::Unusable("bad base url".to_owned()),
            SearchError::NotConfigured,
        ] {
            assert_eq!(answered.fault(), Fault::Refused, "{answered}");
        }
    }

    #[test]
    fn the_two_answers_that_mean_later_are_the_only_ones_worth_waiting_on() {
        assert_eq!(
            SearchError::Status { status: 429 }.fault(),
            Fault::TooFast,
            "429 says later in so many words"
        );
        for broke in [500u16, 502, 503] {
            assert_eq!(
                SearchError::Status { status: broke }.fault(),
                Fault::Silent,
                "a {broke} is the engine breaking rather than declining"
            );
        }
        assert_eq!(
            SearchError::Unreachable("no route to host".to_owned()).fault(),
            Fault::Silent
        );
    }

    #[test]
    fn a_refusal_among_timeouts_is_an_engine_that_refuses() {
        // The actionable fact wins. Telling somebody to wait when one query has already
        // proved that waiting is useless is exactly what this type exists to stop.
        assert_eq!(
            Fault::worst_of([Fault::Silent, Fault::Refused, Fault::Silent]),
            Some(Fault::Refused)
        );
        assert_eq!(
            Fault::worst_of([Fault::Silent, Fault::TooFast]),
            Some(Fault::TooFast)
        );
        assert_eq!(Fault::worst_of([Fault::Silent]), Some(Fault::Silent));
    }

    #[test]
    fn nothing_failed_is_not_a_fault() {
        // **`None` rather than a calm default.** A caller with no failures has nothing to
        // report, and a default here would put a sentence about our engine on a report that
        // has no reason to mention one.
        assert_eq!(Fault::worst_of([]), None);
    }

    #[test]
    fn what_a_reader_is_told_names_nothing_they_cannot_act_on() {
        // The report is read by strangers. `SEARX_URL`, a status code and a file path are the
        // operator's half and live in `landscape search`'s output - the same division
        // `migrations/0001_init.sql` makes for `failure_reason`.
        for fault in [Fault::Refused, Fault::TooFast, Fault::Silent] {
            let advice = fault.advice();
            for internal in ["SEARX_URL", "403", "429", "settings.yml", "SearXNG", "http"] {
                assert!(
                    !advice.contains(internal),
                    "{fault:?} tells a reader about {internal}: {advice}"
                );
            }
        }
        // And the one thing they can act on is whether asking again is worth anything.
        assert!(Fault::Silent.advice().contains("try again"));
        assert!(Fault::Refused.advice().contains("will not change"));
    }
}
