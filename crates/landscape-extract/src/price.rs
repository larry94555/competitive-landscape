//! Is there a price here?
//!
//! Not *what* the price is — that is the model's job, and the golden set already measures
//! how well it does it. This answers the cruder question the JS-rendering gap needs: did
//! this page show a reader a price at all?
//!
//! # The lesson this is built on
//!
//! `BENCHMARKS.md` Run 3 included a golden-set subject called
//! *numbers-that-are-not-prices*: a page with a founding year, a headcount, a phone number,
//! a street number and a dollar figure describing somebody else's revenue. Anything that
//! counts digits reports that page as priced.
//!
//! So a price is **a currency marker adjacent to a number**, never a number alone. That is
//! stricter than a human reading the page, and stricter is the right direction: this
//! measurement decides whether a browser tier gets built, and over-reporting prices would
//! hide the very gap it exists to size.

/// Currency markers we recognise, symbol or code.
const SYMBOLS: [char; 5] = ['$', '€', '£', '¥', '₹'];
const CODES: [&str; 8] = ["usd", "eur", "gbp", "cad", "aud", "chf", "jpy", "inr"];

/// How far a number may sit from its currency marker and still belong to it.
///
/// `$ 49` is one space. `USD 1,299.00` is four characters of separator at most. Beyond
/// that the two are unrelated things that happen to be near each other — *"pricing in USD"*
/// followed by a paragraph containing a year should not count.
const MAX_GAP: usize = 4;

/// What was found, if anything.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    /// The matched text, for eyeballing a sample of results without re-fetching.
    pub evidence: String,
}

/// Whether this text shows a price.
///
/// # Examples
/// ```
/// use landscape_extract::price;
/// assert!(price::find("Grower is $49 per month").is_some());
/// assert!(price::find("Founded 2019, 42 employees, call (802) 555-0148").is_none());
/// ```
#[must_use]
pub fn find(text: &str) -> Option<Found> {
    for line in text.lines() {
        if let Some(found) = in_line(line) {
            return Some(found);
        }
    }
    None
}

fn in_line(line: &str) -> Option<Found> {
    let lower = line.to_lowercase();
    let bytes: Vec<char> = line.chars().collect();

    // A symbol followed closely by a digit: "$49", "$ 49", "€ 25".
    for (i, ch) in bytes.iter().enumerate() {
        if !SYMBOLS.contains(ch) {
            continue;
        }
        if let Some(evidence) = digits_within(&bytes, i + 1, MAX_GAP) {
            return Some(Found {
                evidence: format!("{ch}{evidence}"),
            });
        }
    }

    // A currency code near a number, either order: "USD 29", "29 USD".
    for code in CODES {
        let mut from = 0usize;
        while let Some(rel) = lower[from..].find(code) {
            let at = from + rel;
            // Whole word only, so "used" does not match "usd" and "aud" does not match
            // "fraud" — both of which appear on real pages.
            let before_ok = at == 0 || !lower.as_bytes()[at - 1].is_ascii_alphanumeric();
            let after = at + code.len();
            let after_ok = after >= lower.len() || !lower.as_bytes()[after].is_ascii_alphanumeric();

            if before_ok && after_ok {
                let chars: Vec<char> = lower.chars().collect();
                // Character index, since `find` gave a byte offset and the two differ on
                // any page with a currency symbol earlier in the line.
                let char_at = lower[..at].chars().count();
                if let Some(evidence) = digits_within(&chars, char_at + code.len(), MAX_GAP) {
                    return Some(Found {
                        evidence: format!("{} {evidence}", code.to_uppercase()),
                    });
                }
                if digits_before(&chars, char_at, MAX_GAP) {
                    return Some(Found {
                        evidence: code.to_uppercase(),
                    });
                }
            }
            from = at + code.len();
        }
    }

    // "Free" as a plan price. A real finding — a free tier is a published price, and the
    // golden set has a subject for exactly that — but only when it reads like a price
    // rather than like "free trial" or "feel free to".
    if is_free_price(&lower) {
        return Some(Found {
            evidence: "free".to_owned(),
        });
    }
    None
}

/// Digits starting within `gap` characters of `from`, and the number they spell.
fn digits_within(chars: &[char], from: usize, gap: usize) -> Option<String> {
    let mut i = from;
    let limit = (from + gap).min(chars.len());
    while i < limit && !chars[i].is_ascii_digit() {
        // Only separators may sit between a marker and its number.
        if !chars[i].is_whitespace() && chars[i] != '\u{a0}' {
            return None;
        }
        i += 1;
    }
    if i >= chars.len() || !chars[i].is_ascii_digit() {
        return None;
    }
    let number: String = chars[i..]
        .iter()
        .take_while(|c| c.is_ascii_digit() || **c == ',' || **c == '.')
        .collect();
    Some(number)
}

fn digits_before(chars: &[char], before: usize, gap: usize) -> bool {
    let start = before.saturating_sub(gap);
    chars[start..before].iter().any(char::is_ascii_digit)
}

/// "Free" used as a price, not as an adjective.
///
/// `free trial`, `free forever` and `feel free` all appear on pricing pages and none of them
/// is the price of a plan. This wants the word standing where a number would stand.
fn is_free_price(lower: &str) -> bool {
    const NOT_A_PRICE: [&str; 8] = [
        "free trial",
        "free for 14",
        "free for 30",
        "feel free",
        "free of charge for",
        "toll-free",
        // "…for free" is an adverb, not a price. Found on databricks.com's *contact*
        // page — "Learn professional Data and AI tools for free" — which the measurement
        // then counted as a priced page. A control-group page reporting a price is how a
        // measuring instrument tells you it is broken.
        "for free",
        "free of charge",
    ];
    if NOT_A_PRICE.iter().any(|p| lower.contains(p)) {
        return false;
    }
    // "$0", "0/month" and the like are caught by the symbol rule above. This is the word
    // on its own line or beside a plan word, which is how a pricing table writes it.
    let trimmed = lower.trim();
    // Deliberately narrow: the word standing where a number would stand. `ends_with(" free")`
    // was here and matched any sentence ending in the word, which is most of the reason the
    // first run over-reported tier 1.
    trimmed == "free"
        || trimmed.starts_with("free/")
        || trimmed.starts_with("free ·")
        || trimmed.starts_with("free -")
        || trimmed.starts_with("free forever")
        || trimmed.contains("free plan")
        || trimmed.contains("free tier")
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn found(text: &str) -> bool {
        find(text).is_some()
    }

    #[test]
    fn ordinary_prices_are_found() {
        assert!(found("Grower is $49 per month"));
        assert!(found("€25 per month"));
        assert!(found("£22/mo"));
        assert!(found("USD 29 per user"));
        assert!(found("1,299.00 USD"));
        assert!(found("$ 49"));
    }

    #[test]
    fn a_page_full_of_numbers_and_no_prices_finds_nothing() {
        // The golden set's `numbers-that-are-not-prices` subject, in miniature. Anything
        // counting digits reports this page as priced.
        let page = "Founded in 2019\n42 employees across two offices\n\
                    1200 Preston Road, Suite 300\ncall (802) 555-0148\n\
                    18,000 tons of produce\nWe are hiring for 6 roles";
        assert!(!found(page), "found a price in: {page}");
    }

    #[test]
    fn a_currency_code_inside_another_word_does_not_count() {
        // "used", "fraud", "caudal" — all real words on real pages, all containing a
        // currency code.
        assert!(!found("500 units used last year"));
        assert!(!found("fraud detection for 300 merchants"));
    }

    #[test]
    fn a_currency_marker_far_from_a_number_does_not_count() {
        // "All pricing in USD." followed by a sentence with a year in it is not a price.
        assert!(!found("All pricing is quoted in USD. Founded 2019."));
    }

    #[test]
    fn a_free_tier_counts_as_a_published_price() {
        // A free plan is a price — the golden set has a subject for it, and a page saying
        // "Free" has told the reader what it costs.
        assert!(found("Free"));
        assert!(found("Free forever, no card required"));
        assert!(found("Free plan"));
        assert!(found("$0 per month"));
    }

    #[test]
    fn free_used_as_an_adjective_does_not_count() {
        assert!(!found("Start your free trial"));
        assert!(!found("Feel free to get in touch"));
        assert!(!found("Free for 14 days"));
        // Found in the wild, on a page with no prices on it at all: the first run of this
        // measurement counted databricks.com/company/contact as priced because of it.
        assert!(!found("Learn professional Data and AI tools for free"));
        assert!(!found("Get started for free"));
        assert!(!found("Download it free of charge"));
    }

    #[test]
    fn a_contact_sales_page_finds_nothing() {
        // The commonest real page with no price. If this returned a match, the JS-gap
        // measurement would report the gap as smaller than it is.
        let page = "Enterprise\nBuilt for teams running more than fifty routes.\n\
                    Includes single sign-on and a private data region.\nContact sales for pricing.";
        assert!(!found(page), "found a price in a contact-sales page");
    }

    #[test]
    fn the_evidence_is_kept_so_a_sample_can_be_eyeballed() {
        // The measurement drives a build-or-not decision, so being able to look at what it
        // matched — without re-fetching a hundred pages — is part of trusting the number.
        assert_eq!(
            find("Grower is $49 a month").expect("found").evidence,
            "$49"
        );
        assert_eq!(
            find("1,299.00 USD billed yearly").expect("found").evidence,
            "USD"
        );
    }

    #[test]
    fn a_symbol_after_a_currency_symbol_does_not_confuse_the_offsets() {
        // The line has a multi-byte character before the code, so a byte offset used as a
        // character index would land mid-number.
        assert!(found("€25 or USD 29"));
    }
}
