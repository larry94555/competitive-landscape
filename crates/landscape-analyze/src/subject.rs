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
    description needs the search channel that is not built yet (FACT_CHECKING.md §3.3). Try a \
    domain — for example: basecamp.com";

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
