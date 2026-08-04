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
    prompt.split_whitespace().find_map(origin_of_word)
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
