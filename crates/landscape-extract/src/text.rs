//! HTML in, the words a reader would see out.
//!
//! Deliberately not a parser. This exists to answer one question — *is a price visible on
//! this page without running JavaScript?* — and a tag stripper answers it for the
//! overwhelming majority of pages at a fraction of the cost of `html5ever`.
//!
//! **The dependency question was decided by what happens when this is wrong.** A real parser
//! is warranted for the extraction pipeline, where a mangled table loses a fact. Here a
//! mistake shifts a percentage, and the percentages get published with fixtures that show
//! what the instrument does — see the tests. `CODING_QUALITY.md` §3.1 makes adding a
//! dependency an architecture change; this is not the change that justifies one.
//!
//! What it must get right, and does:
//!
//! - `<script>` and `<style>` contents are **removed, not stripped of tags**. A Next.js page
//!   ships its entire pricing model inside a `<script>`, and counting that as visible text
//!   would report the JS-rendering gap as zero — the exact wrong answer.
//! - Block elements become line breaks, so `<td>29</td><td>49</td>` does not read as `2949`.
//! - The handful of entities that appear in prices are decoded.

/// Elements whose contents are not words on the page.
const INVISIBLE: [&str; 5] = ["script", "style", "noscript", "template", "svg"];

/// Elements that end a line. Without these, adjacent cells and list items run together and
/// two prices become one number that is neither.
const BLOCK: [&str; 24] = [
    "p", "div", "br", "li", "tr", "td", "th", "h1", "h2", "h3", "h4", "h5", "h6", "section",
    "article", "header", "footer", "nav", "table", "ul", "ol", "dl", "dd", "dt",
];

/// The visible text of an HTML document.
#[must_use]
pub fn visible(html: &str) -> String {
    let without_invisible = strip_invisible(html);
    let text = strip_tags(&without_invisible);
    let decoded = decode_entities(&text);
    tidy(&decoded)
}

/// Remove `<script>…</script>` and friends, contents included.
///
/// Shared with [`crate::markdown`], which needs the same guarantee for a stronger reason:
/// a page's embedded JSON handed to a model as prose costs enormous context for no
/// information.
pub(crate) fn strip_invisible(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let lower = html.to_lowercase();
    let mut i = 0usize;

    'outer: while i < html.len() {
        for tag in INVISIBLE {
            let open = format!("<{tag}");
            if lower[i..].starts_with(&open) {
                let close = format!("</{tag}");
                // An unclosed script runs to the end of the document, and everything after
                // it is inside it. Dropping the remainder is correct and, on a truncated
                // page, is the difference between "no price" and a page of JavaScript.
                let end = lower[i..].find(&close).map_or(html.len(), |rel| {
                    let from = i + rel;
                    lower[from..].find('>').map_or(html.len(), |g| from + g + 1)
                });
                i = end;
                continue 'outer;
            }
        }
        let ch = html[i..].chars().next().unwrap_or(' ');
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

/// Drop the tags, turning block-level ones into line breaks.
fn strip_tags(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut rest = html;

    while let Some(start) = rest.find('<') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        let Some(end) = after.find('>') else {
            // A stray `<` that never closes is text, not a tag.
            out.push_str(&rest[start..]);
            return out;
        };
        let inner = &after[..end];
        let name: String = inner
            .trim_start_matches('/')
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric())
            .collect::<String>()
            .to_lowercase();
        if BLOCK.contains(&name.as_str()) {
            out.push('\n');
        } else {
            // An inline tag still separates words: `<b>$</b>29` is "$ 29", not "$29", and
            // for finding a price adjacent to a number that is close enough.
            out.push(' ');
        }
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    out
}

/// Decode the entities that actually turn up around prices.
///
/// Shared with [`crate::markdown`].
///
/// Not a full table on purpose. `&nbsp;` is the one that matters — it is what sits between a
/// currency symbol and its number on a great many pages, and leaving it as literal text
/// breaks the adjacency test that decides whether a price was found.
pub(crate) fn decode_entities(s: &str) -> String {
    let mut out = s
        .replace("&nbsp;", " ")
        .replace("&#160;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&dollar;", "$")
        .replace("&#36;", "$")
        .replace("&euro;", "€")
        .replace("&pound;", "£");
    // Numeric entities for the currency symbols, which some CMSs emit.
    for (entity, ch) in [("&#8364;", "€"), ("&#163;", "£"), ("&#8377;", "₹")] {
        out = out.replace(entity, ch);
    }
    out
}

/// Collapse runs of whitespace, keeping line structure.
fn tidy(s: &str) -> String {
    s.lines()
        .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn ordinary_markup_becomes_readable_text() {
        let html =
            "<html><body><h1>Pricing</h1><p>Grower is <b>$49</b> per month.</p></body></html>";
        let text = visible(html);
        assert!(text.contains("Pricing"));
        assert!(text.contains("$49"), "got: {text}");
    }

    #[test]
    fn script_contents_are_removed_not_merely_untagged() {
        // The single most important behavior in this file. A Next.js pricing page ships
        // its whole pricing model inside a <script>; counting that as visible text would
        // report the JS-rendering gap as zero, which is the exact wrong answer.
        let html = r#"<body><script>var data = {"price": "$49"}</script><p>Contact us</p></body>"#;
        let text = visible(html);
        assert!(!text.contains("49"), "script contents leaked: {text}");
        assert!(text.contains("Contact us"));
    }

    #[test]
    fn style_and_svg_are_removed_too() {
        let html = "<style>.a{content:'$99'}</style><svg><title>$77</title></svg><p>Free</p>";
        let text = visible(html);
        assert!(!text.contains("99"), "style leaked: {text}");
        assert!(!text.contains("77"), "svg leaked: {text}");
        assert!(text.contains("Free"));
    }

    #[test]
    fn an_unclosed_script_swallows_the_rest_of_the_document() {
        // A truncated page — ours are capped at 2 MB — can end mid-script. Treating the
        // remainder as visible text would find "prices" in JavaScript source.
        let html = "<p>Plans</p><script>var x = '$49';";
        let text = visible(html);
        assert!(text.contains("Plans"));
        assert!(!text.contains("49"), "unclosed script leaked: {text}");
    }

    #[test]
    fn table_cells_do_not_run_together() {
        // Without block-level line breaks, 29 and 49 become 2949 — a number that is
        // neither price, and one that would pass a naive "is there a price" check.
        let text = visible("<table><tr><td>29</td><td>49</td></tr></table>");
        assert!(!text.contains("2949"), "cells ran together: {text}");
    }

    #[test]
    fn a_non_breaking_space_between_symbol_and_number_survives() {
        // Very common, and it is exactly what the adjacency test in `price` depends on.
        let text = visible("<p>$&nbsp;49 per month</p>");
        assert!(text.contains("$ 49"), "got: {text}");
    }

    #[test]
    fn a_stray_angle_bracket_is_text_rather_than_a_tag() {
        let text = visible("<p>Plans under $50 < $100</p>");
        assert!(text.contains("$50"), "got: {text}");
    }

    #[test]
    fn currency_entities_decode() {
        assert!(visible("<p>&euro;25</p>").contains('€'));
        assert!(visible("<p>&#163;22</p>").contains('£'));
    }

    #[test]
    fn empty_and_rubbish_input_do_not_panic() {
        // Rubbish in produces rubbish out, and that is the right answer: `<<<>>>` has no
        // tags in it, so leaving the stray brackets as text is correct. What matters is
        // that nothing panics and no *price* is invented — a page of garbage must not
        // count toward the measurement in either direction.
        assert_eq!(visible(""), "");
        for junk in ["<<<>>>", "<script", "<p>unclosed", "<<", ">>", "<>"] {
            let out = visible(junk);
            assert!(
                crate::price::find(&out).is_none(),
                "found a price in junk {junk:?}: {out:?}"
            );
        }
    }
}
