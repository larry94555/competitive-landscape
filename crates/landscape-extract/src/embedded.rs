//! Tier 2 — the price that was in the bytes all along.
//!
//! `ARCHITECTURE.md` §5.5 calls this **"the big one"**, and the claim is specific enough to
//! be wrong:
//!
//! > *A Next.js pricing page ships its pricing as JSON **in the initial HTML**. The page
//! > looks JS-rendered; the data is already in the bytes we fetched.*
//!
//! If that holds, most of the JS-rendering gap closes for free and tier 5 — a headless
//! browser on a box that has no spare memory for one — is never built. If it does not hold,
//! the plan has a hole in it. **This module exists to find out which**, so the two counters
//! in §5.5 can be measured rather than assumed.
//!
//! # Where the price hides
//!
//! | Shape | Looks like |
//! |---|---|
//! | JSON-LD | `<script type="application/ld+json">` with `Product` / `Offer` / `price` |
//! | Next.js | `<script id="__NEXT_DATA__" type="application/json">` |
//! | Nuxt | `window.__NUXT__ = {...}` |
//! | Generic | any inline `<script>` whose JSON contains a price-shaped key |
//!
//! The generic case is the one that matters most and is the least tidy. Rather than parse
//! JavaScript, this looks for **price-shaped keys with numeric values** inside `<script>`
//! contents. That is a heuristic, and it is deliberately reported separately from the named
//! framework cases so a reader can discount it.

/// Which shape the price was found in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    /// `application/ld+json`, the structured-data standard. The most trustworthy hit: it is
    /// a machine-readable price a site published on purpose.
    JsonLd,
    /// `__NEXT_DATA__` or `__NUXT__` — a framework's server-rendered state.
    FrameworkState,
    /// A price-shaped key in some other inline script. The weakest of the three, and
    /// counted separately for that reason.
    InlineJson,
}

impl Shape {
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::JsonLd => "json-ld",
            Self::FrameworkState => "framework state",
            Self::InlineJson => "inline json",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    pub shape: Shape,
    /// The matched fragment, trimmed. Kept so a sample can be checked by eye.
    pub evidence: String,
}

/// Keys a price hides behind. Checked as `"key"` followed by a value, so a page merely
/// containing the word "price" in prose does not count.
const PRICE_KEYS: [&str; 10] = [
    "price",
    "amount",
    "unitprice",
    "unit_price",
    "priceamount",
    "monthlyprice",
    "yearlyprice",
    "amount_decimal",
    "unit_amount",
    "lowprice",
];

/// Look for a price inside the page's scripts.
#[must_use]
pub fn find(html: &str) -> Option<Found> {
    let scripts = script_blocks(html);

    // Ordered by how much the hit is worth trusting, not by how likely it is. A JSON-LD
    // price is a published machine-readable fact; an inline-JSON hit is a guess.
    for (attrs, body) in &scripts {
        if attrs.contains("ld+json") {
            if let Some(evidence) = price_key_with_number(body) {
                return Some(Found {
                    shape: Shape::JsonLd,
                    evidence,
                });
            }
        }
    }
    for (attrs, body) in &scripts {
        if attrs.contains("__next_data__") || body.contains("__NUXT__") || body.contains("__next_f")
        {
            if let Some(evidence) = price_key_with_number(body) {
                return Some(Found {
                    shape: Shape::FrameworkState,
                    evidence,
                });
            }
        }
    }
    for (_, body) in &scripts {
        if let Some(evidence) = price_key_with_number(body) {
            return Some(Found {
                shape: Shape::InlineJson,
                evidence,
            });
        }
    }
    None
}

/// `(lowercased attributes, body)` for every `<script>` in the document.
fn script_blocks(html: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let lower = html.to_lowercase();
    let mut from = 0usize;

    while let Some(rel) = lower[from..].find("<script") {
        let open = from + rel;
        let Some(gt) = lower[open..].find('>') else {
            break;
        };
        let attrs = lower[open + "<script".len()..open + gt].to_owned();
        let body_start = open + gt + 1;
        let end = lower[body_start..]
            .find("</script")
            .map_or(html.len(), |r| body_start + r);
        if body_start <= end {
            out.push((attrs, html[body_start..end].to_owned()));
        }
        from = end.max(body_start);
        if from >= html.len() {
            break;
        }
    }
    out
}

/// A price-shaped key whose value is a number.
///
/// The value check is what keeps this from firing on every page with the word "price" in a
/// CSS class or a translation string. `"price":"Contact us"` is not a price; `"price":49`
/// and `"price":"49.00"` are.
fn price_key_with_number(body: &str) -> Option<String> {
    let lower = body.to_lowercase();
    for key in PRICE_KEYS {
        let needle = format!("\"{key}\"");
        let mut from = 0usize;
        while let Some(rel) = lower[from..].find(&needle) {
            let at = from + rel + needle.len();
            if let Some(value) = numeric_value_after(&lower[at..]) {
                // Zero is a real price — a free tier — but a lone 0 is also what an empty
                // template renders, so it is not enough on its own to call a page priced.
                if value != "0" {
                    let end = (at + 40).min(lower.len());
                    return Some(
                        lower[at.saturating_sub(key.len() + 2)..end]
                            .trim()
                            .to_owned(),
                    );
                }
            }
            from = at;
        }
    }
    None
}

/// The number after `:` (and optional quotes), if the value is one.
fn numeric_value_after(rest: &str) -> Option<String> {
    let mut chars = rest.chars().peekable();
    // Skip to the colon, allowing whitespace only — `"price" : 49` is legal JSON.
    for c in chars.by_ref() {
        if c == ':' {
            break;
        }
        if !c.is_whitespace() {
            return None;
        }
    }
    let mut number = String::new();
    let mut seen_quote = false;
    for c in chars {
        if c.is_whitespace() {
            if number.is_empty() {
                continue;
            }
            break;
        }
        if c == '"' {
            if number.is_empty() && !seen_quote {
                seen_quote = true;
                continue;
            }
            break;
        }
        if c.is_ascii_digit() || (c == '.' && !number.is_empty()) {
            number.push(c);
            continue;
        }
        // A currency symbol immediately before the digits is fine: "price":"$49".
        if number.is_empty() && "$€£¥₹".contains(c) {
            continue;
        }
        break;
    }
    // Strip a trailing dot so "49." does not parse as something odd downstream.
    let number = number.trim_end_matches('.').to_owned();
    (!number.is_empty()).then_some(number)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn a_json_ld_offer_is_found() {
        let html = r#"<html><head>
          <script type="application/ld+json">
          {"@type":"Product","name":"Grower","offers":{"@type":"Offer","price":49,"priceCurrency":"USD"}}
          </script></head><body><p>Contact us</p></body></html>"#;
        let found = find(html).expect("should find the offer");
        assert_eq!(found.shape, Shape::JsonLd);
    }

    #[test]
    fn a_next_data_blob_is_found() {
        // The claim ARCHITECTURE §5.5 rests on: the page looks JS-rendered, and the price
        // is already in the bytes we fetched.
        let html = r#"<body><div id="__next"></div>
          <script id="__NEXT_DATA__" type="application/json">
          {"props":{"pageProps":{"plans":[{"name":"Grower","price":"49.00"}]}}}
          </script></body>"#;
        let found = find(html).expect("should find the next data");
        assert_eq!(found.shape, Shape::FrameworkState);
    }

    #[test]
    fn a_nuxt_state_blob_is_found() {
        let html = r#"<body><script>window.__NUXT__={data:[{"price":29}]}</script></body>"#;
        assert_eq!(find(html).expect("found").shape, Shape::FrameworkState);
    }

    #[test]
    fn json_ld_wins_over_a_weaker_hit_in_the_same_page() {
        // Ordered by trustworthiness rather than document order: a published
        // machine-readable price beats a guess at a key name.
        let html = r#"<script>var conf={"price":11}</script>
          <script type="application/ld+json">{"offers":{"price":49}}</script>"#;
        assert_eq!(find(html).expect("found").shape, Shape::JsonLd);
    }

    #[test]
    fn a_price_key_with_a_non_numeric_value_is_not_a_price() {
        // `"price":"Contact us"` is exactly what a quote-only plan ships, and counting it
        // would report the gap as closed when the reader still cannot see a number.
        let html = r#"<script>{"price":"Contact us","plan":"Enterprise"}</script>"#;
        assert!(find(html).is_none(), "matched a non-numeric price");
    }

    #[test]
    fn the_word_price_in_prose_or_css_does_not_count() {
        let html = r#"<script>var t={"pricePageTitle":"Our pricing"};</script>
          <style>.price-table{color:red}</style><p>See our price page</p>"#;
        assert!(find(html).is_none(), "matched prose or a class name");
    }

    #[test]
    fn a_lone_zero_is_not_enough_to_call_a_page_priced() {
        // A free tier is a real price and is caught by the visible-text rule. Here a bare
        // 0 is more often an empty template than a published price.
        let html = r#"<script>{"price":0}</script>"#;
        assert!(find(html).is_none());
    }

    #[test]
    fn a_page_with_no_scripts_finds_nothing() {
        assert!(find("<html><body><p>Contact sales</p></body></html>").is_none());
    }

    #[test]
    fn malformed_scripts_do_not_panic() {
        for html in [
            "<script",
            "<script>",
            "<script>{\"price\":",
            "<script></script></script>",
            "<script type=\"application/ld+json\">",
        ] {
            let _ = find(html);
        }
    }

    #[test]
    fn a_currency_symbol_inside_the_json_value_still_reads_as_a_number() {
        let html = r#"<script>{"monthlyPrice":"$49"}</script>"#;
        assert_eq!(find(html).expect("found").shape, Shape::InlineJson);
    }
}
