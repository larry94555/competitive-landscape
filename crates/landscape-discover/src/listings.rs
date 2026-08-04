//! Pages a site tells us about, rather than pages we guess at.
//!
//! Two files, both listed in `FACT_CHECKING.md` §3.3's probe set, and both far better than
//! guessing because **the site wrote them for exactly this purpose**:
//!
//! - `sitemap.xml` — what exists. Often the only way to find a pricing page that lives at
//!   `/en-gb/plans-and-pricing`.
//! - `llms.txt` — a newer convention: a short markdown file naming the pages a site would
//!   like an automated reader to read. Rare, and worth honouring where present, because a
//!   site that publishes one has told us what it considers important.
//!
//! # Bounded on purpose
//!
//! A sitemap can list fifty thousand URLs, and a sitemap index can point at more sitemaps.
//! Both are somebody else's file, so both are read with a hard cap rather than a promise —
//! the size limit in `landscape-fetch` bounds the bytes, and [`MAX_URLS`] bounds what we do
//! with them.

use crate::probes::{self, Answers};

/// The most URLs taken from one sitemap.
///
/// Far more than a subject needs, far less than a large site publishes. Anything past this
/// is a catalogue rather than a set of pages about the company, and reading further would
/// spend the analysis budget on product listings.
pub const MAX_URLS: usize = 200;

/// A URL a site told us about, and what it looks like it answers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listed {
    pub url: String,
    pub answers: Answers,
}

/// URLs worth keeping from a `sitemap.xml`.
///
/// Only those whose path classifies as answering something — a sitemap's value here is
/// finding the pricing page at an unguessable path, not enumerating a blog.
///
/// `origin` resolves **relative `<loc>` values**. The sitemap specification requires
/// absolute URLs and real sitemaps do not always contain them — basecamp.com publishes
/// `<loc>/pricing</loc>`. Left relative, those URLs cannot be fetched *and* do not
/// deduplicate against the same page found by a probe, so one page silently takes two of
/// the eight slots.
#[must_use]
pub fn from_sitemap(xml: &str, origin: &str) -> Vec<Listed> {
    let origin = origin.trim_end_matches('/');
    let mut out = Vec::new();
    for loc in locs(xml).into_iter().take(MAX_URLS) {
        let url = absolute(&loc, origin);
        let path = path_of(&url);
        if let Some(answers) = probes::guess(&path) {
            out.push(Listed { url, answers });
        }
    }
    out
}

/// Make a `<loc>` absolute, if it is not already.
fn absolute(loc: &str, origin: &str) -> String {
    if loc.contains("://") {
        return loc.to_owned();
    }
    if let Some(rest) = loc.strip_prefix("//") {
        // Protocol-relative. Keep the origin's scheme.
        let scheme = origin.split_once("://").map_or("https", |(s, _)| s);
        return format!("{scheme}://{rest}");
    }
    if loc.starts_with('/') {
        return format!("{origin}{loc}");
    }
    format!("{origin}/{loc}")
}

/// Nested sitemap URLs from a `<sitemapindex>`, if this is one.
///
/// Returned separately rather than followed here: fetching is the caller's business, and a
/// module that quietly makes network requests while parsing is a module nobody can test.
#[must_use]
pub fn nested_sitemaps(xml: &str) -> Vec<String> {
    if !xml.to_lowercase().contains("<sitemapindex") {
        return Vec::new();
    }
    // Only one level. A sitemap index pointing at sitemap indexes is legal, rare, and not
    // worth the recursion budget or the risk of a cycle.
    locs(xml).into_iter().take(20).collect()
}

/// Every `<loc>` value, in document order.
fn locs(xml: &str) -> Vec<String> {
    let mut out = Vec::new();
    let lower = xml.to_lowercase();
    let mut from = 0usize;

    while let Some(rel) = lower[from..].find("<loc>") {
        let start = from + rel + "<loc>".len();
        let Some(end_rel) = lower[start..].find("</loc>") else {
            break;
        };
        let end = start + end_rel;
        let value = xml[start..end].trim();
        if !value.is_empty() {
            out.push(unescape(value));
        }
        from = end;
    }
    out
}

/// Pages named in an `llms.txt`.
///
/// The format is markdown, and the useful part is its links. Parsed loosely on purpose: the
/// convention is young and inconsistently followed, and a strict parser would reject files
/// whose author clearly meant well.
#[must_use]
pub fn from_llms_txt(body: &str, origin: &str) -> Vec<Listed> {
    let mut out = Vec::new();
    for line in body.lines().take(500) {
        let Some(open) = line.find("](") else {
            continue;
        };
        let rest = &line[open + 2..];
        let Some(close) = rest.find(')') else {
            continue;
        };
        let target = rest[..close].split_whitespace().next().unwrap_or("").trim();
        if target.is_empty() {
            continue;
        }

        let url = if target.starts_with("http://") || target.starts_with("https://") {
            target.to_owned()
        } else if target.starts_with('/') {
            format!("{}{target}", origin.trim_end_matches('/'))
        } else {
            continue;
        };

        if let Some(answers) = probes::guess(&path_of(&url)) {
            out.push(Listed { url, answers });
        }
        if out.len() >= MAX_URLS {
            break;
        }
    }
    out
}

/// The path part of a URL, for classification.
///
/// Query and fragment are dropped **before** classifying, and keeping them was a real bug:
/// `/pricing?ref=nav` classified as nothing, because `pricing?ref=nav` is not the segment
/// `pricing`. The URL itself keeps them — some sites select a plan that way — but the
/// question "what kind of page is this" is answered by the path alone.
fn path_of(url: &str) -> String {
    let after_scheme = url.split_once("://").map_or(url, |(_, rest)| rest);
    let path = after_scheme
        .split_once('/')
        .map_or_else(|| "/".to_owned(), |(_, path)| format!("/{path}"));
    path.split(['?', '#']).next().unwrap_or("/").to_owned()
}

/// The five XML entities. A `&amp;` in a sitemap URL is extremely common.
fn unescape(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    const SITEMAP: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
      <urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
        <url><loc>https://example.com/</loc></url>
        <url><loc>https://example.com/en-gb/plans</loc></url>
        <url><loc>https://example.com/blog/some-post</loc></url>
        <url><loc>https://example.com/changelog</loc></url>
      </urlset>"#;

    #[test]
    fn a_pricing_page_at_an_unguessable_path_is_found() {
        // The whole reason to read a sitemap. No probe would ever try /en-gb/plans.
        let found = from_sitemap(SITEMAP, "https://example.com");
        assert!(
            found
                .iter()
                .any(|l| l.url.ends_with("/en-gb/plans") && l.answers == Answers::Pricing),
            "got: {found:?}"
        );
    }

    #[test]
    fn pages_that_answer_nothing_are_left_out() {
        // A sitemap's value is finding the pricing page, not enumerating a blog.
        let found = from_sitemap(SITEMAP, "https://example.com");
        assert!(!found.iter().any(|l| l.url.contains("/blog/some-post")));
        assert!(!found.iter().any(|l| l.url == "https://example.com/"));
    }

    #[test]
    fn a_huge_sitemap_is_bounded() {
        // Somebody else's file decides how long this loop runs, unless we decide instead.
        let many: String = (0..5000)
            .map(|i| format!("<url><loc>https://e.com/pricing/{i}</loc></url>"))
            .collect();
        let xml = format!("<urlset>{many}</urlset>");
        assert!(from_sitemap(&xml, "https://e.com").len() <= MAX_URLS);
    }

    #[test]
    fn a_sitemap_index_is_recognised_and_a_plain_sitemap_is_not() {
        let index = r"<sitemapindex><sitemap><loc>https://e.com/sitemap-1.xml</loc></sitemap></sitemapindex>";
        assert_eq!(nested_sitemaps(index).len(), 1);
        assert!(
            nested_sitemaps(SITEMAP).is_empty(),
            "a urlset is not an index"
        );
    }

    #[test]
    fn nesting_stops_at_one_level() {
        // Legal, rare, and not worth the recursion budget or the cycle risk.
        let many: String = (0..100)
            .map(|i| format!("<sitemap><loc>https://e.com/s{i}.xml</loc></sitemap>"))
            .collect();
        let index = format!("<sitemapindex>{many}</sitemapindex>");
        assert!(nested_sitemaps(&index).len() <= 20);
    }

    #[test]
    fn xml_entities_in_a_url_are_decoded() {
        let xml = "<urlset><url><loc>https://e.com/pricing?a=1&amp;b=2</loc></url></urlset>";
        assert_eq!(
            from_sitemap(xml, "https://e.com")[0].url,
            "https://e.com/pricing?a=1&b=2"
        );
    }

    #[test]
    fn a_relative_loc_is_resolved_against_the_origin() {
        // basecamp.com publishes `<loc>/pricing</loc>`. The spec says absolute; reality
        // says otherwise. Left relative it cannot be fetched, and it will not deduplicate
        // against the same page found by a probe — so one page takes two of eight slots.
        let xml = "<urlset><url><loc>/pricing</loc></url></urlset>";
        let found = from_sitemap(xml, "https://basecamp.com");
        assert_eq!(found[0].url, "https://basecamp.com/pricing");
    }

    #[test]
    fn an_absolute_loc_is_left_alone() {
        let xml = "<urlset><url><loc>https://cdn.example/pricing</loc></url></urlset>";
        assert_eq!(
            from_sitemap(xml, "https://example.com")[0].url,
            "https://cdn.example/pricing"
        );
    }

    #[test]
    fn malformed_xml_does_not_panic_or_hang() {
        for xml in [
            "",
            "<urlset>",
            "<loc>",
            "<loc>unclosed",
            "<loc></loc>",
            "not xml at all",
        ] {
            let _ = from_sitemap(xml, "https://e.com");
            let _ = nested_sitemaps(xml);
        }
    }

    #[test]
    fn llms_txt_links_are_read_absolute_or_relative() {
        let body = "# Example\n\n- [Pricing](/pricing)\n- [Changelog](https://example.com/changelog)\n- [Home](/)\n";
        let found = from_llms_txt(body, "https://example.com");
        assert_eq!(found.len(), 2, "got: {found:?}");
        assert!(found.iter().any(|l| l.url == "https://example.com/pricing"));
        assert!(found
            .iter()
            .any(|l| l.url == "https://example.com/changelog"));
    }

    #[test]
    fn an_llms_txt_that_is_not_markdown_yields_nothing_rather_than_rubbish() {
        // The convention is young and inconsistently followed. Yielding nothing is the
        // right failure: a guessed URL is a request against somebody's server.
        assert!(from_llms_txt("just some prose about our company", "https://e.com").is_empty());
        assert!(from_llms_txt("", "https://e.com").is_empty());
    }

    #[test]
    fn a_relative_link_that_is_not_rooted_is_skipped() {
        // `[Pricing](pricing)` in an llms.txt is ambiguous about what it is relative to,
        // and resolving it wrongly means fetching a URL nobody published.
        assert!(from_llms_txt("- [Pricing](pricing)", "https://e.com").is_empty());
    }
}
