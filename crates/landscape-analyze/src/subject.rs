//! Turning what somebody typed into a site to read.
//!
//! `FACT_CHECKING.md` §3.1 resolves a subject in three steps: find candidates, score them,
//! and refuse to guess between the top two if they are close. [`landscape_core::resolve`] is
//! that decision, written and tested — and it takes **candidates**, which come from search.
//!
//! **Search is not built.** §3.3 puts it last on purpose (*"search fills gaps; it does not
//! lead"*) and it needs a self-hosted SearXNG this project does not have. So this module does
//! the one case that needs no search at all: a prompt that already names the site.
//!
//! ```text
//! https://basecamp.com          -> https://basecamp.com
//! basecamp.com                  -> https://basecamp.com
//! "compare basecamp.com"        -> https://basecamp.com
//! "a tool for small farms"      -> nothing, and the run says so
//! ```
//!
//! The last line is the important one. A prompt this cannot resolve **fails the analysis with
//! a reason**, rather than picking a plausible domain — an analysis of the wrong company is
//! the most expensive wrong answer this product can produce, because everything in it is
//! correctly cited and about somebody else.

/// A hostname's last label, when it looks like a public suffix worth trusting.
///
/// Deliberately short. It is not a public-suffix list and does not try to be: it is the
/// difference between recognising `basecamp.com` in a sentence and treating every word with a
/// dot in it as a website.
const KNOWN_SUFFIXES: [&str; 24] = [
    "com", "org", "net", "io", "co", "dev", "app", "ai", "so", "sh", "xyz", "cloud", "tech",
    "software", "tools", "systems", "uk", "de", "fr", "nl", "se", "eu", "ca", "us",
];

/// The origin to analyse, if the prompt names one.
///
/// Returns a scheme and host — `https://example.com` — because everything downstream fetches
/// it and guessing the scheme for somebody is the kind of silent assumption this codebase
/// keeps finding bugs in. `http://` is preserved where it is written; a bare domain becomes
/// `https://`, which is what a browser does.
#[must_use]
pub fn origin_in(prompt: &str) -> Option<String> {
    origins_in(prompt).into_iter().next()
}

/// How many companies one report will cover.
///
/// **A latency cap, not a design limit.** Each company is its own discovery, fetches and model
/// calls, so a report about four takes four times as long — and a reader waiting ten minutes
/// has stopped waiting. Three is what fits inside the ninety-to-a-hundred-and-eighty seconds
/// `PRODUCT_SPEC.md` §2.1A asks for on the free tier; raising it is a decision about the wait,
/// which is why the number is here and named rather than inline.
pub const MAX_SUBJECTS: usize = 3;

/// Every site named in the prompt, in the order written, without repeats.
///
/// `basecamp.com vs linear.app` used to analyse Basecamp alone: [`origin_in`] took the first
/// domain and the rest were dropped in silence. Naming two companies and being given one is the
/// kind of wrong answer that looks like a right answer, because nothing on the page says the
/// second was ignored.
///
/// **Uncapped on purpose.** [`MAX_SUBJECTS`] is a decision about how long a reader will wait,
/// and applying it here would drop the fourth company as silently as the old code dropped the
/// second. The caller takes as many as it will analyse and *says* what it left out.
#[must_use]
pub fn origins_in(prompt: &str) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    for word in prompt.split_whitespace() {
        let Some(origin) = origin_of_word(word) else {
            continue;
        };
        // `basecamp.com and www.basecamp.com` is one company written twice. The comparison is
        // between *companies*, so a repeat is a duplicate rather than a second subject.
        if !found.iter().any(|seen| same_site(seen, &origin)) {
            found.push(origin);
        }
    }
    found
}

/// Whether two origins are the same site, ignoring a leading `www.`.
fn same_site(a: &str, b: &str) -> bool {
    fn bare(origin: &str) -> &str {
        origin
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .trim_start_matches("www.")
    }
    bare(a) == bare(b)
}

/// One word, if it is a URL or a domain.
fn origin_of_word(word: &str) -> Option<String> {
    let trimmed = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '/' && c != ':');
    let (scheme, rest) = match trimmed.split_once("://") {
        Some(("http", rest)) => ("http", rest),
        Some(("https", rest)) => ("https", rest),
        // Any other scheme is not something to fetch, and a bare word has no scheme yet.
        Some(_) => return None,
        None => ("https", trimmed),
    };

    let host = rest.split(['/', '?', '#']).next()?.trim_end_matches('.');
    if host.is_empty() || host.contains('@') || host.contains(' ') {
        return None;
    }
    let labels: Vec<&str> = host.split('.').collect();
    if labels.len() < 2 || labels.iter().any(|l| l.is_empty()) {
        return None;
    }
    let suffix = labels.last()?.to_lowercase();
    if !KNOWN_SUFFIXES.contains(&suffix.as_str()) {
        return None;
    }
    if !labels
        .iter()
        .all(|l| l.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'))
    {
        return None;
    }
    Some(format!("{scheme}://{}", host.to_lowercase()))
}

/// What to tell somebody whose prompt named no site.
///
/// A failure reason a reader can act on. It says what is missing rather than what went wrong,
/// because nothing went wrong: this is a capability the pipeline does not have yet.
pub const NO_SUBJECT: &str = "this prompt does not name a website, and finding one from a \
    description needs a search engine, which is not configured here. Try a domain — for \
    example: basecamp.com";

/// What a reader is told when we searched and found nobody.
///
/// **Different from [`NO_SUBJECT`], and the difference is the whole honest-negative
/// discipline.** *"We have no way to look"* and *"we looked and found nothing"* are different
/// facts about the world, and a reader can act on the second by rewording rather than by
/// installing something.
pub const NOTHING_RESOLVED: &str = "we searched for companies matching that description and \
    found none we could identify well enough to report on. Try naming a domain, or describing \
    the product in the words a vendor would use — for example: basecamp.com";

/// What to do with the gate's verdict.
///
/// **The decision, in a function a test can call.** It lived in the worker as a `match` inside a
/// binary, where the only way to exercise it was to run one — so the arm that tells an outage
/// apart from an empty market had no test, and the mutation that deleted it passed. Every branch
/// here is reachable from a unit test, which matters because this is the code that decides
/// whether a report is written about the wrong company or not written at all.
#[derive(Debug, Clone, PartialEq)]
pub enum Decided {
    /// One company, clearly. The run proceeds against it.
    Analyse(Box<landscape_core::subject::Candidate>),
    /// Not one company, and this is what a reader is told about why.
    Refuse(String),
}

/// Turn a verdict and the evidence behind it into what happens next.
#[must_use]
pub fn decide(
    verdict: landscape_core::subject::Resolution,
    queried: &landscape_search::candidates::Queried,
) -> Decided {
    use landscape_core::subject::Resolution;
    match verdict {
        Resolution::Resolved { entity } => Decided::Analyse(Box::new(entity)),
        // Several real candidates, whatever else failed. A reader can answer this, and telling
        // them to try again instead would throw away an answer we already have.
        Resolution::Ambiguous { candidates, .. } => Decided::Refuse(ambiguous(&candidates)),
        // **"Nothing found" is a conclusion about a market, and it needs the searching to have
        // happened.** With any query unanswered we have not established that nobody is out
        // there — only that we did not finish looking, which is a different sentence and a
        // retryable one. Review found these collapsed into each other.
        Resolution::NothingFound { .. } if !queried.failed.is_empty() => {
            Decided::Refuse(search_incomplete(queried.failed.len(), queried.sent()))
        }
        Resolution::NothingFound { .. } => Decided::Refuse(NOTHING_RESOLVED.to_owned()),
    }
}

/// What a reader is told when the searching itself did not finish.
///
/// **Not the same as finding nobody, and review found the two collapsed.** With every query
/// failing, the previous version said *"we searched and found none"* — a conclusion about a
/// market, drawn from a conclusion about nothing. This one is about us, it is retryable, and it
/// is the only refusal here that a reader fixes by waiting.
#[must_use]
pub fn search_incomplete(failed: usize, sent: usize) -> String {
    format!(
        "we could not complete {failed} of the {sent} searches needed to work out which company \
         you mean, so we have not concluded anything about who is out there. This is usually \
         temporary - try again, or name a domain to skip the search entirely."
    )
}

/// What a reader is told when we found several and will not choose for them.
///
/// `PRODUCT_SPEC.md` §3: *one chip click prevents an entire wrong report*. There are no chips
/// yet, so the companies are named in the sentence and the reader picks one by typing it —
/// which is the same choice, made more slowly, and far better than a confident wrong report.
#[must_use]
pub fn ambiguous(candidates: &[landscape_core::subject::Candidate]) -> String {
    let named: Vec<String> = candidates
        .iter()
        .map(|c| format!("{} ({})", c.name, c.canonical_domain))
        .collect();
    format!(
        "that description matches more than one company and we will not guess between them: \
         {}. Name the one you mean - a domain works.",
        named.join(", ")
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod deciding {
    //! What happens to each of the gate's verdicts, and why they are not the same refusal.

    use super::*;
    use landscape_core::subject::{Candidate, Resolution};
    use landscape_search::candidates::Queried;

    fn candidate(name: &str, domain: &str) -> Candidate {
        Candidate {
            name: name.to_owned(),
            canonical_domain: domain.to_owned(),
            what_it_is: "a company".to_owned(),
            confidence: 0.9,
        }
    }

    fn all_answered() -> Queried {
        Queried {
            completed: vec!["q1".to_owned(), "q2".to_owned(), "q3".to_owned()],
            failed: Vec::new(),
        }
    }

    #[test]
    fn one_company_is_analysed() {
        let decided = decide(
            Resolution::Resolved {
                entity: candidate("Fathom", "usefathom.com"),
            },
            &all_answered(),
        );
        match decided {
            Decided::Analyse(entity) => assert_eq!(entity.canonical_domain, "usefathom.com"),
            Decided::Refuse(why) => panic!("a clear answer was refused: {why}"),
        }
    }

    #[test]
    fn several_companies_are_named_so_a_reader_can_pick() {
        let decided = decide(
            Resolution::Ambiguous {
                question: "which one?".to_owned(),
                candidates: vec![
                    candidate("Alpha", "alpha.example"),
                    candidate("Beta", "beta.example"),
                ],
            },
            &all_answered(),
        );
        let Decided::Refuse(why) = decided else {
            panic!("two companies were chosen between")
        };
        assert!(why.contains("Alpha (alpha.example)"), "{why}");
        assert!(why.contains("Beta (beta.example)"), "{why}");
    }

    #[test]
    fn several_companies_are_still_asked_about_when_a_search_failed() {
        // A real answer we already have beats telling somebody to try again for it.
        let decided = decide(
            Resolution::Ambiguous {
                question: "which one?".to_owned(),
                candidates: vec![
                    candidate("Alpha", "alpha.example"),
                    candidate("Beta", "beta.example"),
                ],
            },
            &Queried {
                completed: vec!["q1".to_owned()],
                failed: vec!["q2".to_owned(), "q3".to_owned()],
            },
        );
        let Decided::Refuse(why) = decided else {
            panic!("two companies were chosen between")
        };
        assert!(why.contains("Alpha"), "{why}");
        assert!(
            !why.contains("try again"),
            "a real answer was thrown away: {why}"
        );
    }

    #[test]
    fn a_market_we_searched_and_found_empty_says_so() {
        let decided = decide(
            Resolution::NothingFound {
                checked: vec!["q1".to_owned()],
            },
            &all_answered(),
        );
        let Decided::Refuse(why) = decided else {
            panic!("nothing found was not a refusal")
        };
        assert_eq!(why, NOTHING_RESOLVED);
        assert!(!why.contains("try again"), "{why}");
    }

    #[test]
    fn a_search_that_did_not_finish_is_never_reported_as_an_empty_market() {
        // **Review found these collapsed.** With every query failing, the reader was told *"we
        // searched and found none"* - a conclusion about a market drawn from a conclusion about
        // nothing. It is retryable and it is about us.
        for failed in [
            vec!["q1".to_owned(), "q2".to_owned(), "q3".to_owned()],
            vec!["q3".to_owned()],
        ] {
            let queried = Queried {
                completed: vec!["q1".to_owned(); 3 - failed.len()],
                failed: failed.clone(),
            };
            let decided = decide(
                Resolution::NothingFound {
                    checked: queried.completed.clone(),
                },
                &queried,
            );
            let Decided::Refuse(why) = decided else {
                panic!("nothing found was not a refusal")
            };
            assert_ne!(why, NOTHING_RESOLVED, "{failed:?}");
            assert!(why.contains("try again"), "{why}");
            assert!(
                why.contains(&format!("{} of the 3", failed.len())),
                "the count a reader needs is missing: {why}"
            );
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod the_examples {
    //! The curated ideas, run through the parser that will really read them.
    //!
    //! **This is the test the catalogue exists for.** A list of domains in a file is a promise;
    //! putting each prompt through `origins_in` - the function the worker calls, not a
    //! reimplementation of it - is the part that can fail. A wording change that breaks the
    //! parser's reading of a domain would otherwise be found by whoever clicked the chip.

    use super::{origins_in, MAX_SUBJECTS};

    #[test]
    fn every_example_prompt_resolves_to_exactly_the_companies_it_names() {
        for example in landscape_core::examples() {
            let expected: Vec<String> = example
                .companies
                .iter()
                .map(|c| format!("https://{c}"))
                .collect();
            assert_eq!(
                origins_in(&example.prompt()),
                expected,
                "{} does not parse to its own companies. The prompt is {:?}",
                example.id,
                example.prompt()
            );
        }
    }

    #[test]
    fn no_example_is_capped_when_it_runs() {
        // The cap drops companies and says so in a note above the sections. That note is right
        // for a prompt somebody typed and wrong on a curated example, where it would mean the
        // demo shipped an idea it cannot run.
        for example in landscape_core::examples() {
            assert!(
                example.companies.len() <= MAX_SUBJECTS,
                "{} names {} companies and the cap is {MAX_SUBJECTS}",
                example.id,
                example.companies.len()
            );
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    #[test]
    fn every_site_named_is_a_subject() {
        // The defect this exists for. `origin_in` took the first and dropped the rest, so a
        // prompt naming two companies produced a report about one — with nothing on the page
        // saying the other had been ignored.
        assert_eq!(
            origins_in("compare basecamp.com vs linear.app for a small team"),
            vec!["https://basecamp.com", "https://linear.app"]
        );
    }

    #[test]
    fn the_same_site_twice_is_one_subject() {
        // A comparison is between companies, so `www.` and a repeat are not a second one.
        assert_eq!(
            origins_in("basecamp.com and www.basecamp.com and basecamp.com"),
            vec!["https://basecamp.com"]
        );
    }

    #[test]
    fn the_order_written_is_the_order_reported() {
        // A reader who writes B before A and reads A before B has to work out whether we
        // reordered them or misread them.
        assert_eq!(
            origins_in("linear.app vs basecamp.com"),
            vec!["https://linear.app", "https://basecamp.com"]
        );
    }

    #[test]
    fn every_site_is_returned_and_the_cap_is_somebody_elses_job() {
        // Capping here would drop the fourth company as silently as the old code dropped the
        // second — the same defect at a higher count. The parser returns what the prompt says
        // and the caller decides what it can afford, so it can say what it left out.
        let many = "a.com b.com c.com d.com e.com";
        assert_eq!(origins_in(many).len(), 5);
        assert!(origins_in(many).len() > MAX_SUBJECTS);
    }

    #[test]
    fn a_prompt_naming_nothing_has_no_subjects() {
        assert!(origins_in("an app that helps small farms sell to restaurants").is_empty());
    }

    #[test]
    fn the_first_site_named_is_still_the_single_subject() {
        // `origin_in` is now the first of `origins_in`, and callers that want one still get
        // the same answer they always did.
        assert_eq!(
            origin_in("compare basecamp.com vs linear.app"),
            Some("https://basecamp.com".to_owned())
        );
    }

    use super::*;

    #[test]
    fn a_url_is_taken_as_written() {
        assert_eq!(
            origin_in("https://basecamp.com/pricing").as_deref(),
            Some("https://basecamp.com")
        );
        assert_eq!(
            origin_in("http://example.org").as_deref(),
            Some("http://example.org")
        );
    }

    #[test]
    fn a_bare_domain_becomes_https() {
        // What a browser does with the same input.
        assert_eq!(
            origin_in("basecamp.com").as_deref(),
            Some("https://basecamp.com")
        );
    }

    #[test]
    fn a_domain_inside_a_sentence_is_found() {
        assert_eq!(
            origin_in("please compare basecamp.com against the others").as_deref(),
            Some("https://basecamp.com")
        );
        assert_eq!(
            origin_in("what does linear.app cost?").as_deref(),
            Some("https://linear.app")
        );
    }

    #[test]
    fn a_description_resolves_to_nothing() {
        // The case that matters. Guessing a domain here would produce a report that is
        // correctly cited and about the wrong company, which is the most expensive wrong
        // answer available.
        assert_eq!(
            origin_in("an app that helps small farms sell to restaurants"),
            None
        );
        assert_eq!(origin_in("a tool that chases unpaid invoices"), None);
    }

    #[test]
    fn an_ordinary_sentence_with_a_full_stop_is_not_a_domain() {
        assert_eq!(origin_in("we ship weekly.and we are proud of it"), None);
        assert_eq!(origin_in("version 1.2 of the thing"), None);
    }

    #[test]
    fn an_email_address_is_not_a_site_to_read() {
        assert_eq!(origin_in("write to hello@basecamp.com"), None);
    }

    #[test]
    fn a_scheme_we_do_not_fetch_is_refused() {
        assert_eq!(origin_in("ftp://files.example.com"), None);
        assert_eq!(origin_in("mailto:hello@example.com"), None);
    }

    #[test]
    fn the_first_site_named_is_the_subject() {
        // One subject per analysis. A prompt naming two is a comparison, which is a
        // different product surface and not something to guess at here.
        assert_eq!(
            origin_in("basecamp.com vs linear.app").as_deref(),
            Some("https://basecamp.com")
        );
    }

    #[test]
    fn nothing_in_an_empty_prompt() {
        assert_eq!(origin_in(""), None);
        assert_eq!(origin_in("   "), None);
    }
}
