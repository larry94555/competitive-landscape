//! Recognizing the same page published in another language.
//!
//! `BENCHMARKS.md` Run 7 read `todoist.com/cs/pricing` and `todoist.com/da/pricing` — Czech
//! and Danish — and never read the English page at all. Run 8 added `notion.com/es-es/pricing`
//! to the list.
//!
//! **Two of the eight slots went to the same page twice, and the answers were in a language
//! nothing downstream expects.** The extractor asks for prices in US dollars and quotes
//! evidence verbatim; a Danish page can satisfy both and still be the wrong page to have read.
//!
//! # Why this is not a filter
//!
//! Dropping localized URLs outright would be wrong: some sites publish *only* localized paths,
//! and `/de/preise` may be the only pricing page there is. So a locale is treated as what it
//! is — **a variant of a page, not a different page** — and variants collapse into one
//! candidate the same way `/pricing` and `/pricing/` already do.
//!
//! Which variant wins is then a preference rather than an exclusion: no locale at all, then
//! English, then whatever the site listed first. A site with only Danish pricing still gets its
//! pricing page read.

/// Language codes seen as a leading path segment on real sites.
///
/// A list rather than "any two letters", because two-letter segments are not all languages:
/// `/hr` is Croatian on one site and human resources on the next, and `/id` is Indonesian and
/// also an identifier. The list is the common web-facing set; a site using a code outside it
/// loses nothing but the deduplication.
const LANGUAGES: [&str; 40] = [
    "ar", "bg", "cs", "da", "de", "el", "en", "es", "et", "fa", "fi", "fr", "he", "hi", "hr", "hu",
    "id", "it", "ja", "ko", "lt", "lv", "ms", "nb", "nl", "no", "pl", "pt", "ro", "ru", "sk", "sl",
    "sr", "sv", "th", "tr", "uk", "vi", "zh", "ca",
];

/// The locale segment leading this path, if it has one.
///
/// Matches `cs`, `es-es`, `zh-CN`, `pt_BR`. **Only as a prefix of something else** — `/it` on
/// its own is far more likely to be a page about IT than the Italian homepage, and a locale
/// segment with nothing under it is not a variant of any page.
#[must_use]
pub fn leading(path: &str) -> Option<&str> {
    let trimmed = path.trim_start_matches('/');
    let (first, rest) = trimmed.split_once('/')?;
    if rest.is_empty() {
        return None;
    }
    is_locale(first).then_some(first)
}

/// Whether a segment is a language tag, with or without a region.
///
/// Public because classification needs to skip a leading locale too: `/de/blog/a-post` is a
/// post in German, and the segment that says so should not be mistaken for the publication.
#[must_use]
pub fn is_locale_segment(segment: &str) -> bool {
    is_locale(segment)
}

/// Whether a segment is a language tag, with or without a region.
fn is_locale(segment: &str) -> bool {
    let lowered = segment.to_lowercase();
    let (language, region) = match lowered.split_once(['-', '_']) {
        Some((l, r)) => (l, Some(r)),
        None => (lowered.as_str(), None),
    };
    if !LANGUAGES.contains(&language) {
        return false;
    }
    // A region is two letters or a three-digit UN code. Anything else is a path that starts
    // with a language-shaped word — `/no-code-tools` is not Norwegian.
    region.is_none_or(|r| {
        (r.len() == 2 && r.chars().all(|c| c.is_ascii_alphabetic()))
            || (r.len() == 3 && r.chars().all(|c| c.is_ascii_digit()))
    })
}

/// The same URL with its locale segment removed, so variants share a key.
#[must_use]
pub fn stripped(url: &str) -> String {
    let Some((prefix, path)) = split_path(url) else {
        return url.to_owned();
    };
    match leading(path) {
        Some(locale) => {
            let rest = path
                .trim_start_matches('/')
                .get(locale.len()..)
                .unwrap_or_default();
            format!("{prefix}{rest}")
        }
        None => url.to_owned(),
    }
}

/// How much we would rather read this URL than another variant of the same page.
///
/// Lower is better. English rather than "the site's default" because everything downstream is
/// written in it: the prompts, the golden set, and the report a reader is going to be shown.
#[must_use]
pub fn preference(url: &str) -> u8 {
    let Some((_, path)) = split_path(url) else {
        return 0;
    };
    match leading(path) {
        None => 0,
        Some(locale) if locale.to_lowercase().starts_with("en") => 1,
        Some(_) => 2,
    }
}

/// Split `https://host` from `/the/path`, or `None` if there is no path.
fn split_path(url: &str) -> Option<(&str, &str)> {
    let after_scheme = url.find("://").map_or(0, |i| i + 3);
    let slash = url[after_scheme..].find('/')? + after_scheme;
    Some((&url[..slash], &url[slash..]))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn the_locales_run_7_and_8_actually_read_are_recognized() {
        assert_eq!(leading("/cs/pricing"), Some("cs"));
        assert_eq!(leading("/da/pricing"), Some("da"));
        assert_eq!(leading("/es-es/pricing"), Some("es-es"));
    }

    #[test]
    fn a_locale_with_nothing_under_it_is_not_a_variant() {
        // `/it` is far more likely to be a page about IT than the Italian homepage, and a
        // locale segment with no page under it is a variant of nothing.
        assert_eq!(leading("/it"), None);
        assert_eq!(leading("/de/"), None);
    }

    #[test]
    fn a_path_that_merely_starts_with_a_language_shaped_word_is_not_a_locale() {
        assert_eq!(leading("/no-code-tools/pricing"), None);
        assert_eq!(leading("/pricing/enterprise"), None);
        assert_eq!(leading("/id-verification/api"), None);
    }

    #[test]
    fn variants_of_one_page_share_a_key() {
        for url in [
            "https://todoist.com/cs/pricing",
            "https://todoist.com/da/pricing",
            "https://todoist.com/en-us/pricing",
        ] {
            assert_eq!(stripped(url), "https://todoist.com/pricing", "{url}");
        }
    }

    #[test]
    fn a_url_with_no_locale_is_left_alone() {
        assert_eq!(
            stripped("https://basecamp.com/pricing"),
            "https://basecamp.com/pricing"
        );
        assert_eq!(stripped("https://basecamp.com"), "https://basecamp.com");
    }

    #[test]
    fn the_page_in_no_language_is_preferred_then_english() {
        assert!(preference("https://e.com/pricing") < preference("https://e.com/en/pricing"));
        assert!(preference("https://e.com/en/pricing") < preference("https://e.com/da/pricing"));
        assert_eq!(
            preference("https://e.com/en-gb/pricing"),
            preference("https://e.com/en-us/pricing")
        );
    }
}
