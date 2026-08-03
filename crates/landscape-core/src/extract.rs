//! What one extraction pass pulls out of a single page.
//!
//! This is the narrowest useful unit of the analysis pipeline: one source, one subject,
//! one small set of fields. A report is many of these, assembled.
//!
//! **Every field is optional, and that is the design.** The product's promise is that a
//! gap is reported as a gap rather than filled in — see `FACT_CHECKING.md` §5.4. If
//! `price_usd` were a bare `f64`, the type itself would demand a number for a page that
//! does not publish one, and constrained decoding would *guarantee* the model invented
//! something. The grammar cannot emit a shape the type does not allow, so the type has to
//! allow honesty.
//!
//! That inverts the usual instinct. Here `Option` is not laziness about validation; it is
//! the only representation in which "the page does not say" is expressible.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// How often a published price recurs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BillingPeriod {
    Monthly,
    Yearly,
    /// A single payment, not a subscription.
    OneOff,
}

/// The pricing facts for one named plan, as published on one page.
///
/// **Declaration order here is not the order the model answers in.** The schema reaches
/// llama.cpp through `serde_json`, whose maps are sorted, so the grammar walks the fields
/// alphabetically: `billing_period`, `evidence_quote`, `plan_name`, `price_usd`. Reordering
/// this struct changes nothing.
///
/// Worth knowing because the order is a real lever — a model that must write the quote
/// before the price is reading to answer, and one that writes it afterwards is justifying
/// an answer it already gave. Taking hold of that lever means turning on `serde_json`'s
/// `preserve_order`, and it should be done as its own change, measured against the golden
/// set rather than assumed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct PricingExtraction {
    /// The plan's name as the page writes it. `None` when the page prices something
    /// without naming a plan.
    pub plan_name: Option<String>,

    /// The recurring price in US dollars. `None` when the page publishes no price for
    /// this plan — "contact sales" is a `None`, not a zero.
    ///
    /// `0.0` and `None` mean different things and both occur: a free tier publishes a
    /// price, and that price is zero.
    pub price_usd: Option<f64>,

    /// `None` when a price is given with no stated period.
    pub billing_period: Option<BillingPeriod>,

    /// The span of the page that supports the answer, copied verbatim.
    ///
    /// Verbatim is checkable without a human: a quote that is not a substring of the
    /// source is fabricated evidence, and that can be asserted mechanically. It is the
    /// cheapest strong signal we have that a model is reading rather than recalling.
    pub evidence_quote: Option<String>,
}

impl PricingExtraction {
    /// An extraction that found nothing. What a page with no pricing should produce.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            plan_name: None,
            price_usd: None,
            billing_period: None,
            evidence_quote: None,
        }
    }

    /// Whether the quote, if there is one, really appears in the source.
    ///
    /// Whitespace is normalised first because an extracted span crosses line breaks that
    /// the original wraps differently, and rejecting a correct quote over a newline would
    /// train us to ignore this check.
    #[must_use]
    pub fn quote_is_verbatim(&self, source: &str) -> bool {
        let Some(quote) = &self.evidence_quote else {
            return true;
        };
        let quote = squash(quote);
        // An empty quote is not evidence of anything, but neither is it a fabrication;
        // it is caught as a missing quote instead, where the message is clearer.
        quote.is_empty() || squash(source).contains(&quote)
    }
}

/// Collapse every run of whitespace to one space.
fn squash(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    const PAGE: &str = "The Grower plan is $49 per month\n  and includes unlimited orders.";

    fn with_quote(quote: Option<&str>) -> PricingExtraction {
        PricingExtraction {
            evidence_quote: quote.map(str::to_owned),
            ..PricingExtraction::empty()
        }
    }

    #[test]
    fn a_quote_copied_from_the_page_is_verbatim() {
        assert!(with_quote(Some("$49 per month")).quote_is_verbatim(PAGE));
    }

    #[test]
    fn a_quote_spanning_a_line_break_is_still_verbatim() {
        // The page wraps between "month" and "and". A reader copying the sentence would
        // not preserve that, and neither should this check.
        assert!(with_quote(Some("$49 per month and includes")).quote_is_verbatim(PAGE));
    }

    #[test]
    fn a_paraphrase_is_not_verbatim() {
        // The facts are right and the words are not. This is the case worth catching:
        // it is what a model does when it is recalling rather than reading.
        assert!(!with_quote(Some("$49 monthly")).quote_is_verbatim(PAGE));
    }

    #[test]
    fn no_quote_is_not_a_violation() {
        // Nothing was claimed, so nothing was fabricated. A missing quote is scored
        // separately, as a miss.
        assert!(with_quote(None).quote_is_verbatim(PAGE));
    }

    #[test]
    fn a_free_tier_is_not_an_absence() {
        // The distinction the whole type exists for.
        let free = PricingExtraction {
            price_usd: Some(0.0),
            ..PricingExtraction::empty()
        };
        assert_ne!(free.price_usd, None);
    }

    #[test]
    fn the_schema_allows_every_field_to_be_absent() {
        // If this ever fails, the grammar has started requiring a value the page may not
        // contain, and the model will invent one. That is the failure this type prevents,
        // so it is asserted rather than assumed.
        let schema = serde_json::to_value(schemars::schema_for!(PricingExtraction)).unwrap();
        let required = schema.get("required").and_then(|r| r.as_array());
        assert!(
            required.is_none_or(|r| r.is_empty()),
            "no field may be required: {schema:#}"
        );
    }
}
