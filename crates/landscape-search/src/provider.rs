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
/// **Did the engine answer with a decision about the request?** A decision comes back the same
/// next time. What does not is silence, and the answers that are *about the exchange rather
/// than about the request* — `408` gave up waiting for us, `429` asked to be asked less often,
/// a `5xx` broke in the middle. Written this way so a provider nobody has built yet inherits
/// it: no variant of this enum is about SearXNG.
///
/// **The first version said "did the engine answer at all", and review found the counterexample
/// in one line.** `408 Request Timeout` is an answer, and it is the one status whose entire
/// meaning is *that did not work, try it again* — so the coarse version filed a timeout under
/// *do not bother*. "Answered" was never the property that mattered; *decided* was.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Fault {
    /// The engine answered, and what it said was no. Configuration; waiting changes nothing.
    Refused,
    /// The engine answered, and asked to be asked less often. Waiting is the whole fix.
    TooFast,
    /// The engine did not answer. The only one that is weather.
    Silent,
}

/// What the engine actually did, in the terms whoever is fixing it will be looking for.
///
/// **Three layers, because there are three audiences and they need different resolutions.**
/// [`SearchError`] is the rich one: it carries a body, a client's own message, and cannot be
/// stored — [`crate::candidates::Queried`] is cloned and compared. [`Fault`] is the coarse one:
/// three values, for a reader who can only decide whether to ask again. This is the middle one,
/// and it exists because review found the two ends were not enough.
///
/// **A `401` was being diagnosed as a `403`.** Every refusal printed the same remedy — *check
/// that the instance names `json` in `search.formats`* — which is right for the one status that
/// motivated the change and wrong for an instance asking for credentials, a URL pointing at
/// nothing, or a body we could not parse. An operator was sent to edit a file that was not the
/// problem. The coarse answer is right for a reader and is not a diagnosis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Condition {
    /// It answered with this status.
    Answered(u16),
    /// It answered `200` with something that is not the shape we parse.
    Unreadable,
    /// It answered `200` and kept answering. See [`crate::searx::MAX_RESPONSE_BYTES`].
    TooLarge,
    /// The engine is configured and a client could not be built from it.
    Unusable,
    /// Nothing came back.
    NoAnswer,
    /// No engine is configured at all.
    NotConfigured,
}

impl Condition {
    /// What happened, from the error the call returned.
    #[must_use]
    pub const fn of(error: &SearchError) -> Self {
        match *error {
            SearchError::Status { status } => Self::Answered(status),
            SearchError::Unreadable(_) => Self::Unreadable,
            SearchError::TooLarge { .. } => Self::TooLarge,
            SearchError::Unusable(_) => Self::Unusable,
            SearchError::Unreachable(_) => Self::NoAnswer,
            SearchError::NotConfigured => Self::NotConfigured,
        }
    }

    /// What a reader would do about it — the coarse answer, derived rather than stored.
    #[must_use]
    pub const fn fault(self) -> Fault {
        match self {
            // The statuses that are about the exchange rather than about the request. `408` is
            // the one that made the earlier "did it answer at all" rule wrong: its whole
            // meaning is *that did not work, try it again*.
            Self::Answered(429) => Fault::TooFast,
            Self::Answered(408 | 425) => Fault::Silent,
            Self::Answered(status) if status >= 500 => Fault::Silent,
            // A decision about the request. The same one comes back next time.
            Self::Answered(_)
            | Self::Unreadable
            | Self::TooLarge
            | Self::Unusable
            | Self::NotConfigured => Fault::Refused,
            Self::NoAnswer => Fault::Silent,
        }
    }

    /// A few words for a per-query line, where a sentence does not fit.
    ///
    /// **On the condition rather than on [`Fault`], for the same reason as everything else
    /// here.** The coarse version printed *"no answer"* beside a query the engine had answered
    /// `408` to, which is a small lie in the one place an operator is reading closely.
    #[must_use]
    pub fn word(self) -> String {
        match self {
            Self::Answered(status) => format!("answered {status}"),
            Self::Unreadable => "unreadable body".to_owned(),
            Self::TooLarge => "oversized body".to_owned(),
            Self::Unusable => "unusable engine".to_owned(),
            Self::NoAnswer => "no answer".to_owned(),
            Self::NotConfigured => "no engine".to_owned(),
        }
    }

    /// What an **operator** is told: the actual condition, and the thing to check for it.
    ///
    /// The other half of [`Fault::advice`], and deliberately a different half. A reader cannot
    /// fix a search engine and is shown no status code; whoever is holding the terminal can fix
    /// it and is shown the file. `migrations/0001_init.sql` draws that line for `failure_reason`
    /// and this is the same line drawn once more.
    ///
    /// **One remedy per condition, because a blanket one is a wrong one four times out of
    /// five.** Only the two conditions that genuinely are the JSON opt-in mention it.
    #[must_use]
    pub fn what_to_check(self) -> String {
        let var = crate::searx::URL_VAR;
        match self {
            Self::Answered(403) => concat!(
                "The engine answered 403, which is what a SearXNG that has not opted into ",
                "JSON answers to every query. Check that its settings name `json` in ",
                "`search.formats`; `deploy/searxng/settings.yml` is that opt-in."
            )
            .to_owned(),
            Self::Answered(401) => format!(
                "The engine answered 401: it wants credentials. {var} points at an instance \
                 that is not open to us, which is a different problem from the JSON format."
            ),
            Self::Answered(404) => format!(
                "The engine answered 404: there is nothing to search at that address. {var} \
                 should be the instance's root, not a search URL."
            ),
            Self::Answered(429) => {
                "The engine asked us to slow down. Waiting is the fix; if it keeps happening, \
                 something in front of the instance is rate-limiting this application."
                    .to_owned()
            }
            Self::Answered(408 | 425) => {
                "The engine gave up waiting for the request rather than refusing it. That is \
                 the network between here and there, and it may well work next time."
                    .to_owned()
            }
            Self::Answered(status) if status >= 500 => format!(
                "The engine broke while answering ({status}). Its own log is where the reason \
                 is; nothing here is misconfigured by that alone."
            ),
            Self::Answered(status) => format!(
                "The engine answered {status}. That is a refusal this application has no \
                 specific advice for - the engine's own log will say more than {var} can."
            ),
            // The other genuine JSON case: an instance serving HTML at 200.
            Self::Unreadable => concat!(
                "The engine answered 200 with a body this cannot parse, which is usually ",
                "HTML rather than JSON. Check that its settings name `json` in ",
                "`search.formats`; `deploy/searxng/settings.yml` is that opt-in."
            )
            .to_owned(),
            Self::TooLarge => format!(
                "The engine answered with more than {} bytes and was cut off. That is a \
                 misbehaving instance rather than a setting.",
                crate::searx::MAX_RESPONSE_BYTES
            ),
            Self::Unusable => format!(
                "{var} is set and no client could be built from it. It is the value itself \
                 that is wrong - a scheme and a host, with no path."
            ),
            Self::NoAnswer => format!(
                "The engine did not answer at all. Check that it is running and that {var} \
                 names the address it is actually listening on."
            ),
            Self::NotConfigured => {
                format!("{var} is not set, so nothing was asked.")
            }
        }
    }
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
    ///
    /// Derived through [`Condition`] rather than matched here, so the coarse answer and the
    /// diagnosis cannot drift into disagreeing about the same status.
    #[must_use]
    pub const fn fault(&self) -> Fault {
        Condition::of(self).fault()
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
    fn an_engine_that_decided_is_never_something_to_wait_for() {
        // **The rule, stated as a test rather than as a comment.** Each of these is a decision
        // about the request, and the same decision comes back next time. A 403 is the
        // documented first-run state of an unconfigured SearXNG and used to read as weather.
        for decided in [
            SearchError::Status { status: 403 },
            SearchError::Status { status: 401 },
            SearchError::Status { status: 404 },
            SearchError::Status { status: 400 },
            SearchError::Status { status: 451 },
            SearchError::Unreadable("html, not json".to_owned()),
            SearchError::TooLarge { limit: 1 },
            SearchError::Unusable("bad base url".to_owned()),
            SearchError::NotConfigured,
        ] {
            assert_eq!(decided.fault(), Fault::Refused, "{decided}");
        }
    }

    #[test]
    fn the_answers_that_are_about_the_exchange_are_worth_waiting_on() {
        // **408 is why the rule is not "did it answer at all".** Review found it: a status
        // whose entire meaning is *that did not work, try it again* was being filed under
        // *do not bother*, so a timed-out request was stored as a refusal, the page said
        // trying again would not help, and the terminal pointed at JSON configuration.
        assert_eq!(
            SearchError::Status { status: 408 }.fault(),
            Fault::Silent,
            "408 Request Timeout is the one answer that means exactly try again"
        );
        assert_eq!(
            SearchError::Status { status: 425 }.fault(),
            Fault::Silent,
            "425 Too Early asks for the same request again"
        );
        assert_eq!(
            SearchError::Status { status: 429 }.fault(),
            Fault::TooFast,
            "429 says later in so many words"
        );
        for broke in [500u16, 502, 503, 504] {
            assert_eq!(
                SearchError::Status { status: broke }.fault(),
                Fault::Silent,
                "a {broke} is the engine breaking rather than deciding"
            );
        }
        assert_eq!(
            SearchError::Unreachable("no route to host".to_owned()).fault(),
            Fault::Silent
        );
    }

    #[test]
    fn a_401_is_not_diagnosed_as_a_403() {
        // **Every refusal used to print one remedy**, and it named the JSON opt-in - so an
        // instance asking for credentials, a URL pointing at nothing, and an oversized body
        // all sent an operator to edit `search.formats`. The coarse answer is right for a
        // reader and is not a diagnosis.
        let json_opt_in = "search.formats";

        let unauthorised = Condition::Answered(401).what_to_check();
        assert!(unauthorised.contains("401"), "{unauthorised}");
        assert!(unauthorised.contains("credentials"), "{unauthorised}");
        assert!(
            !unauthorised.contains(json_opt_in),
            "a 401 sent to the JSON opt-in: {unauthorised}"
        );

        let missing = Condition::Answered(404).what_to_check();
        assert!(missing.contains("404"), "{missing}");
        assert!(!missing.contains(json_opt_in), "{missing}");

        let oversized = Condition::TooLarge.what_to_check();
        assert!(!oversized.contains(json_opt_in), "{oversized}");

        let unusable = Condition::Unusable.what_to_check();
        assert!(!unusable.contains(json_opt_in), "{unusable}");

        // The two that really are the JSON opt-in, and only those two.
        for genuine in [Condition::Answered(403), Condition::Unreadable] {
            assert!(
                genuine.what_to_check().contains(json_opt_in),
                "{genuine:?} is the JSON case and must say so"
            );
        }
    }

    #[test]
    fn the_per_query_line_says_what_the_engine_did() {
        // The line an operator reads first, one per failed query. The coarse version printed
        // "no answer" beside a query the engine had answered `408` to.
        assert_eq!(Condition::Answered(408).word(), "answered 408");
        assert_eq!(Condition::Answered(403).word(), "answered 403");
        assert_eq!(Condition::NoAnswer.word(), "no answer");
        assert_eq!(Condition::Unreadable.word(), "unreadable body");
        for answered in [408u16, 403, 500] {
            assert_ne!(
                Condition::Answered(answered).word(),
                Condition::NoAnswer.word(),
                "a {answered} is not silence"
            );
        }
    }

    #[test]
    fn a_status_nobody_wrote_advice_for_still_says_what_happened() {
        // No arm may quietly become a remedy for a status it was not written for.
        let odd = Condition::Answered(418).what_to_check();
        assert!(odd.contains("418"), "{odd}");
        assert!(!odd.contains("search.formats"), "{odd}");
    }

    #[test]
    fn the_coarse_answer_is_derived_from_the_condition_and_not_beside_it() {
        // Two fields would be two facts, and one of them would go stale. `Failed` stores the
        // condition; `fault()` reads it.
        for error in [
            SearchError::Status { status: 403 },
            SearchError::Status { status: 408 },
            SearchError::Unreachable("down".to_owned()),
        ] {
            assert_eq!(Condition::of(&error).fault(), error.fault());
        }
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
