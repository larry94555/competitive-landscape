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

/// Every plan one page publishes.
///
/// **A pricing page is a list, and [`PricingExtraction`] is one item of it.** Reporting the
/// single best-scoring plan was the shape of the first version, and it is worse than it
/// sounds: a competitor report showing a rival's cheapest plan and silently dropping the rest
/// does not look incomplete, it looks *wrong*, and there is nothing on the page to tell the
/// reader which they are getting.
///
/// The plans arrive one per span — see `landscape_extract::span::every_plan` — so this type
/// is where the page-level facts live: how many there are, in what order, and which of them
/// are the same plan named twice.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PagePricing {
    /// In the order the page presents them, which is information: pricing pages lead with
    /// the cheap plan.
    pub plans: Vec<PricingExtraction>,
}

impl PagePricing {
    /// Assemble the per-span extractions into what the page says.
    ///
    /// Two things are dropped, and both are artefacts of segmenting a page rather than
    /// judgements about it:
    ///
    /// - **Extractions that found nothing.** A span that scored as a plan and yielded neither
    ///   a name nor a price was a bad window, not a plan with no facts.
    /// - **The same plan named twice.** Pages publish a plan card *and* a comparison table,
    ///   and pages render a monthly and an annual view into the same HTML. Both put the name
    ///   through twice.
    ///
    /// When a duplicate carries a price and the entry already kept does not, the duplicate
    /// wins — a comparison table often states what a plan card only gestures at.
    #[must_use]
    pub fn assembled(extractions: impl IntoIterator<Item = PricingExtraction>) -> Self {
        let mut plans: Vec<PricingExtraction> = Vec::new();

        for got in extractions {
            if got.plan_name.is_none() && got.price_usd.is_none() {
                continue;
            }
            let key = got.plan_name.as_deref().map(normalise);
            let existing = key.as_ref().and_then(|k| {
                plans
                    .iter()
                    .position(|p| p.plan_name.as_deref().map(normalise).as_ref() == Some(k))
            });
            match existing {
                Some(i) if plans[i].price_usd.is_none() && got.price_usd.is_some() => {
                    plans[i] = got;
                }
                Some(_) => {}
                None => plans.push(got),
            }
        }
        Self { plans }
    }

    /// Whether the page published no plan at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.plans.is_empty()
    }
}

/// Collapse every run of whitespace to one space.
fn squash(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A plan name reduced to what makes two of them the same plan.
///
/// `Pro` and `PRO ` and `Pro plan` are one plan on one page. The suffix goes because pages
/// are inconsistent about it between a plan card and a comparison table — which is exactly
/// where the duplicates come from.
fn normalise(name: &str) -> String {
    let lower = name.to_lowercase();
    let trimmed = lower.trim();
    let base = trimmed
        .strip_suffix(" plan")
        .or_else(|| trimmed.strip_suffix(" tier"))
        .unwrap_or(trimmed);
    squash(base)
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

    fn plan(name: &str, price: Option<f64>) -> PricingExtraction {
        PricingExtraction {
            plan_name: Some(name.to_owned()),
            price_usd: price,
            ..PricingExtraction::empty()
        }
    }

    #[test]
    fn a_page_keeps_its_plans_in_the_order_it_published_them() {
        // Pricing pages lead with the cheap plan, and a report that reorders them is
        // answering a question the page did not ask.
        let page = PagePricing::assembled([
            plan("Free", Some(0.0)),
            plan("Basic", Some(10.0)),
            plan("Business", Some(16.0)),
        ]);
        let names: Vec<_> = page
            .plans
            .iter()
            .filter_map(|p| p.plan_name.clone())
            .collect();
        assert_eq!(names, ["Free", "Basic", "Business"]);
    }

    #[test]
    fn the_same_plan_named_twice_is_one_plan() {
        // Real pages do this constantly: a plan card and then a comparison table, or a
        // monthly and an annual view rendered into the same HTML. sentry.io produces every
        // plan twice.
        let page = PagePricing::assembled([
            plan("Developer", Some(0.0)),
            plan("Team", Some(26.0)),
            plan("developer ", Some(0.0)),
        ]);
        assert_eq!(page.plans.len(), 2, "{page:?}");
    }

    #[test]
    fn a_duplicate_that_states_a_price_beats_one_that_does_not() {
        // The plan card says "starting at"; the comparison table says $10. Keeping the
        // first blindly would report a gap the page does not have.
        let page = PagePricing::assembled([plan("Plus", None), plan("Plus", Some(10.0))]);
        assert_eq!(page.plans.len(), 1);
        assert_eq!(page.plans[0].price_usd, Some(10.0));
    }

    #[test]
    fn an_extraction_that_found_nothing_is_not_a_plan() {
        // A span scored as a plan and the model found neither a name nor a price in it.
        // That is a bad window, and reporting it as a plan with no facts would put the
        // window's mistake in front of a reader as if the page had said something.
        let page = PagePricing::assembled([PricingExtraction::empty(), plan("Pro", Some(15.0))]);
        assert_eq!(page.plans.len(), 1);
    }

    #[test]
    fn a_free_plan_survives_assembly() {
        // `price_usd: Some(0.0)` is a published fact and must not be mistaken for absence
        // by any of the filtering above — the distinction this module exists for.
        let page = PagePricing::assembled([plan("Free", Some(0.0))]);
        assert_eq!(page.plans.len(), 1);
        assert_eq!(page.plans[0].price_usd, Some(0.0));
    }

    #[test]
    fn a_priced_plan_with_no_name_is_still_reported() {
        // A page can price something without naming it. Dropping it would be inventing an
        // absence, which is the one thing this crate must never do.
        let anonymous = PricingExtraction {
            price_usd: Some(9.0),
            ..PricingExtraction::empty()
        };
        assert_eq!(PagePricing::assembled([anonymous]).plans.len(), 1);
    }

    #[test]
    fn a_page_with_no_plans_says_so() {
        assert!(PagePricing::assembled([]).is_empty());
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
