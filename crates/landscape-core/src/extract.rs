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

/// Who a company is, as its own pages state it.
///
/// The fourth question kind, and the one where **what can be checked is weakest**. A price is
/// published to be read; an about page is written to be believed, and the facts are in it in
/// passing. So the fields are few, each is a thing a page either writes down or does not, and
/// none of them is inferred:
///
/// `basecamp.com/about` says *"23 years and running"* and never names a year. The founding
/// year is arithmetic from that, and arithmetic is not extraction — so this type carries no
/// founding year for Basecamp, which is the correct answer rather than a missing one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct IdentityExtraction {
    /// The year the company says it started. `None` unless a year is written on the page.
    pub founded_year: Option<u16>,

    /// Where it says it is — a city, a country, or `the EU`. As the page words it.
    pub headquarters: Option<String>,

    /// How many people it says work there. `None` for "a team of people who care".
    pub employees: Option<u32>,

    /// The words the page used, copied verbatim.
    pub evidence_quote: Option<String>,
}

impl IdentityExtraction {
    /// An extraction that found nothing.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            founded_year: None,
            headquarters: None,
            employees: None,
            evidence_quote: None,
        }
    }

    /// Whether the quote, if there is one, really appears in the source.
    #[must_use]
    pub fn quote_is_verbatim(&self, source: &str) -> bool {
        let Some(quote) = &self.evidence_quote else {
            return true;
        };
        let quote = squash(quote);
        quote.is_empty() || squash(source).contains(&quote)
    }

    /// Whether every value it reports is written in the section it was read from.
    ///
    /// The same check `FeatureExtraction::name_is_from` makes, and here it does more work: a
    /// model that has read about a company **knows** where that company is, and an about page
    /// full of story is exactly the prompt that invites it to say so from memory. A year and a
    /// headcount are digits, which makes them cheap to insist on.
    #[must_use]
    pub fn is_from(&self, source: &str) -> bool {
        let haystack = squash(&source.to_lowercase());
        // Numbers are matched as whole tokens, not substrings. A substring check on a number
        // is not a check: the model answered **0 people** for a page reading *"a team of
        // 10"*, and `"10".contains("0")` waved it through.
        if let Some(year) = self.founded_year {
            if !states_number(&haystack, u32::from(year)) {
                return false;
            }
        }
        if let Some(count) = self.employees {
            if !states_number(&haystack, count) {
                return false;
            }
        }
        if self
            .headquarters
            .as_deref()
            .is_some_and(|place| !states_words(&haystack, place))
        {
            return false;
        }
        true
    }

    /// The same extraction with every value the section does not state removed.
    ///
    /// **Field by field, not all or nothing.** `plausible.io/about` states its founding year
    /// in the window it was asked about and nothing else; an extraction carrying the year and
    /// an invented headcount would have been discarded whole, taking a correct year with it.
    /// Returns the count removed so the run can say how many.
    #[must_use]
    pub fn keeping_only_stated(&self, source: &str) -> (Self, usize) {
        let haystack = squash(&source.to_lowercase());
        let mut kept = self.clone();
        let mut dropped = 0usize;

        if kept
            .founded_year
            .is_some_and(|y| !states_number(&haystack, u32::from(y)))
        {
            kept.founded_year = None;
            dropped += 1;
        }
        if kept.employees.is_some_and(|n| !states_number(&haystack, n)) {
            kept.employees = None;
            dropped += 1;
        }
        if kept
            .headquarters
            .as_deref()
            .is_some_and(|place| !states_words(&haystack, place))
        {
            kept.headquarters = None;
            dropped += 1;
        }
        (kept, dropped)
    }

    /// Whether it found nothing at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.founded_year.is_none() && self.headquarters.is_none() && self.employees.is_none()
    }

    /// How many of the three facts it carries.
    #[must_use]
    pub fn facts(&self) -> usize {
        usize::from(self.founded_year.is_some())
            + usize::from(self.headquarters.is_some())
            + usize::from(self.employees.is_some())
    }
}

/// A value and the words it was read from.
///
/// **A quote belongs to a fact, not to a page.** The first version of [`PageIdentity`] held one
/// list of quotes and paired them to facts by position, and the report it produced put the
/// founding sentence under *"based in the EU"* — evidence for a claim it does not support,
/// which is the one thing this codebase must never render.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stated<T> {
    pub value: T,
    /// The page's own words. `None` when the model gave none.
    pub quote: Option<String>,
}

impl<T> Stated<T> {
    /// The quote, or an empty string.
    #[must_use]
    pub fn quote_or_empty(&self) -> String {
        self.quote.clone().unwrap_or_default()
    }
}

/// What one page said about who the company is.
///
/// Assembled from one window per fact, so the three answers arrive separately and the first
/// page to state a fact keeps it. Later windows fill gaps and never overwrite: two windows
/// disagreeing about a headcount is a page saying two things, and the earlier one is the one
/// the page leads with.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageIdentity {
    pub founded_year: Option<Stated<u16>>,
    pub headquarters: Option<Stated<String>>,
    pub employees: Option<Stated<u32>>,
}

impl PageIdentity {
    /// Merge per-window extractions into what the page says.
    ///
    /// Each value keeps the quote from the extraction it arrived in, **and only if the quote
    /// contains it**. The model picked a neighbouring sentence more than once — a page's
    /// headquarters was rendered under a sentence about web analytics drifting from its
    /// purpose — and a quote that does not contain the fact is not evidence for it. Losing the
    /// quote is the smaller loss: the value was already checked against the whole window.
    #[must_use]
    pub fn assembled(extractions: impl IntoIterator<Item = IdentityExtraction>) -> Self {
        let mut out = Self::default();
        for got in extractions {
            if got.is_empty() {
                continue;
            }
            let quote = got.evidence_quote.filter(|q| !q.trim().is_empty());
            if out.founded_year.is_none() {
                out.founded_year = got.founded_year.map(|value| Stated {
                    quote: quote.clone().filter(|q| states_number(q, u32::from(value))),
                    value,
                });
            }
            if out.headquarters.is_none() {
                out.headquarters = got.headquarters.map(|value| Stated {
                    quote: quote
                        .clone()
                        .filter(|q| q.to_lowercase().contains(&value.to_lowercase())),
                    value,
                });
            }
            if out.employees.is_none() {
                out.employees = got.employees.map(|value| Stated {
                    quote: quote.clone().filter(|q| states_number(q, value)),
                    value,
                });
            }
        }
        out
    }

    /// How many of the three facts the page stated.
    #[must_use]
    pub fn facts(&self) -> usize {
        usize::from(self.founded_year.is_some())
            + usize::from(self.headquarters.is_some())
            + usize::from(self.employees.is_some())
    }

    /// Whether the page stated none of them.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.facts() == 0
    }
}

/// One dated change, as a page states it.
///
/// **Nothing here comes from a model.** [`ARCHITECTURE.md`] §5.4 puts changelog entries on the
/// deterministic side and gives the reason: *"Dates are the most common LLM fabrication in
/// 'recent changes' and are trivially verifiable."* A date is on the page or it is not.
///
/// [`ARCHITECTURE.md`]: ../../../docs/ARCHITECTURE.md
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Change {
    /// When the page says it happened. `None` never occurs from parsing — an entry without a
    /// date is not an entry — but the field is optional so that a change learned some other
    /// way can be held here too.
    pub happened_on: Option<chrono::NaiveDate>,

    /// The entry's title, as written. Empty when a page dated something without titling it.
    pub summary: Option<String>,

    /// The line the date was read from, copied verbatim.
    pub evidence_quote: Option<String>,
}

/// Everything one changelog page says, and what it does not.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageChanges {
    /// In the order the page lists them, which is newest-first on every real changelog.
    pub changes: Vec<Change>,
    /// How many dated entries the page held before any cap was applied.
    pub considered: usize,
}

impl PageChanges {
    /// The window `PRODUCT_SPEC.md` §4 reports on: *"Recent public changes (lookback: 90
    /// days)"*.
    pub const LOOKBACK_DAYS: i64 = 90;

    /// The changes inside the lookback, newest first.
    ///
    /// `today` is passed in rather than read from the clock. A function that asks the
    /// operating system what day it is cannot be tested, and this one decides what a report
    /// shows.
    #[must_use]
    pub fn recent(&self, today: chrono::NaiveDate) -> Vec<&Change> {
        let cutoff = today - chrono::Duration::days(Self::LOOKBACK_DAYS);
        let mut inside: Vec<&Change> = self
            .changes
            .iter()
            .filter(|c| c.happened_on.is_some_and(|d| d > cutoff && d <= today))
            .collect();
        inside.sort_by_key(|c| std::cmp::Reverse(c.happened_on));
        inside
    }

    /// How many dated entries fall outside the lookback.
    ///
    /// Reported rather than dropped. **A quiet quarter and an unread page look identical in a
    /// report that only shows what it found**, and `PRODUCT_SPEC.md` §4's coverage note exists
    /// to tell them apart: *"Not 'no changes.'"*
    #[must_use]
    pub fn older_than_lookback(&self, today: chrono::NaiveDate) -> usize {
        let cutoff = today - chrono::Duration::days(Self::LOOKBACK_DAYS);
        self.changes
            .iter()
            .filter(|c| c.happened_on.is_some_and(|d| d <= cutoff))
            .count()
    }

    /// Entries dated after today.
    ///
    /// A changelog announcing next month's price rise is publishing a fact, and it is not a
    /// change that has shipped. Counted so it can be said out loud rather than folded in.
    #[must_use]
    pub fn dated_ahead(&self, today: chrono::NaiveDate) -> usize {
        self.changes
            .iter()
            .filter(|c| c.happened_on.is_some_and(|d| d > today))
            .count()
    }

    /// Whether the page yielded no dated entry at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }
}

/// One capability, as a page describes it.
///
/// The second question kind. It differs from pricing in what can be checked: a price is on the
/// page or it is not, and a capability is a *claim the vendor makes about itself*. So the
/// fields here are deliberately thin — the name of the thing and the words the page used — and
/// there is no field for whether it works.
///
/// `PRODUCT_SPEC.md` §3's matrix marks a capability ✓ **P** for "stated on a primary source",
/// never "verified". This type is what fills that cell, and it can only ever mean *the vendor
/// says so*.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct FeatureExtraction {
    /// What the thing is called, as short as the page allows. `None` when the section
    /// describes something without naming it.
    pub capability: Option<String>,

    /// A condition the page attaches to it — `beta`, `Business plan and above`, `coming
    /// soon`. `None` when the page states it plainly, which is the common case.
    ///
    /// This is the field that stops a matrix from lying. A ✓ for something available only on
    /// the top tier is not the same fact as a ✓, and the difference is what a competitor
    /// report is *for*.
    pub qualifier: Option<String>,

    /// The words the page used, copied verbatim — checkable the same way a price's quote is.
    pub evidence_quote: Option<String>,
}

impl FeatureExtraction {
    /// An extraction that found nothing.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            capability: None,
            qualifier: None,
            evidence_quote: None,
        }
    }

    /// Whether the quote, if there is one, really appears in the source.
    #[must_use]
    pub fn quote_is_verbatim(&self, source: &str) -> bool {
        let Some(quote) = &self.evidence_quote else {
            return true;
        };
        let quote = squash(quote);
        quote.is_empty() || squash(source).contains(&quote)
    }

    /// Whether every word of the name appears in the section it was taken from.
    ///
    /// **A capability is the one field that cannot be checked verbatim** — naming is the
    /// normalisation the model is there to do, so the answer is a paraphrase by design. This
    /// is the weaker check that still holds: the words have to come from the page.
    ///
    /// It exists because of two real answers. A 4B model handed a section it could not name
    /// returned `string`, the field's own type; and while an example lived in the prompt —
    /// *"a heading reading Message Boards…"* — it returned **Message Boards** for a page of
    /// Linear's documentation. **A worked example in a prompt is a source of facts**, and that
    /// one laundered a Basecamp feature into a Linear report.
    #[must_use]
    pub fn name_is_from(&self, source: &str) -> bool {
        let Some(name) = &self.capability else {
            return true;
        };
        let haystack = squash(&source.to_lowercase());
        let lowered = name.to_lowercase();
        let mut words = lowered
            .split_whitespace()
            .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()))
            .filter(|w| !w.is_empty())
            .peekable();
        words.peek().is_some() && words.all(|w| haystack.contains(w))
    }
}

/// Everything one page says the product does.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PageFeatures {
    /// In the order the page presents them.
    pub features: Vec<FeatureExtraction>,
    /// How many candidate sections the page offered before any cap was applied.
    ///
    /// A features page can list forty things and we read twelve. **Carrying the number is the
    /// difference between a short list and a wrong one**, and it is the same discipline
    /// `FACT_CHECKING.md` §5.4 applies to an absent fact: say what was looked at.
    pub considered: usize,
}

impl PageFeatures {
    /// Assemble per-window extractions into what the page says.
    ///
    /// Drops the ones that named nothing, and the same capability named twice — a page that
    /// lists a feature in a summary and again in detail states one fact, not two.
    #[must_use]
    pub fn assembled(
        extractions: impl IntoIterator<Item = FeatureExtraction>,
        considered: usize,
    ) -> Self {
        let mut features: Vec<FeatureExtraction> = Vec::new();
        for got in extractions {
            let Some(name) = got.capability.as_deref().map(normalise) else {
                continue;
            };
            if name.is_empty() || features.iter().any(|f| named_the_same(f, &name)) {
                continue;
            }
            features.push(got);
        }
        Self {
            features,
            considered,
        }
    }

    /// Whether the page named nothing at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.features.is_empty()
    }
}

/// Whether an already-kept feature carries this name.
fn named_the_same(kept: &FeatureExtraction, name: &str) -> bool {
    kept.capability.as_deref().map(normalise).as_deref() == Some(name)
}

/// Collapse every run of whitespace to one space.
fn squash(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Whether a number appears in the text as a number rather than inside another one.
fn states_number(haystack: &str, number: u32) -> bool {
    let wanted = number.to_string();
    haystack
        .split(|c: char| !c.is_ascii_digit())
        .any(|token| token == wanted)
}

/// Whether every word of a phrase appears in the text.
fn states_words(haystack: &str, phrase: &str) -> bool {
    let lowered = phrase.to_lowercase();
    let mut words = lowered
        .split_whitespace()
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()))
        .filter(|w| !w.is_empty())
        .peekable();
    words.peek().is_some() && words.all(|w| haystack.contains(w))
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

    fn day(y: i32, m: u32, d: u32) -> chrono::NaiveDate {
        chrono::NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn change(on: chrono::NaiveDate) -> Change {
        Change {
            happened_on: Some(on),
            summary: Some("Shipped something".to_owned()),
            evidence_quote: Some("2026-07-14".to_owned()),
        }
    }

    #[test]
    fn the_lookback_is_the_ninety_days_the_spec_reports_on() {
        let today = day(2026, 8, 4);
        let page = PageChanges {
            changes: vec![change(day(2026, 7, 14)), change(day(2026, 1, 5))],
            considered: 2,
        };
        assert_eq!(page.recent(today).len(), 1);
        assert_eq!(page.older_than_lookback(today), 1);
    }

    #[test]
    fn what_falls_outside_the_window_is_counted_rather_than_dropped() {
        // PRODUCT_SPEC §4: a coverage note, because "no entries in the window" and "we did
        // not read the changelog" look identical in a report that only shows what it found.
        let today = day(2026, 8, 4);
        let page = PageChanges {
            changes: vec![change(day(2025, 11, 24)), change(day(2025, 11, 17))],
            considered: 2,
        };
        assert!(page.recent(today).is_empty());
        assert_eq!(page.older_than_lookback(today), 2);
        assert!(!page.is_empty(), "the page had entries; none were recent");
    }

    #[test]
    fn a_change_dated_after_today_has_not_shipped() {
        // A changelog announcing next month's price rise is publishing a fact, and it is not
        // a change that has happened.
        let today = day(2026, 8, 4);
        let page = PageChanges {
            changes: vec![change(day(2026, 9, 1))],
            considered: 1,
        };
        assert!(page.recent(today).is_empty());
        assert_eq!(page.dated_ahead(today), 1);
    }

    #[test]
    fn recent_changes_come_back_newest_first() {
        let today = day(2026, 8, 4);
        let page = PageChanges {
            changes: vec![change(day(2026, 6, 10)), change(day(2026, 7, 14))],
            considered: 2,
        };
        let dates: Vec<_> = page
            .recent(today)
            .iter()
            .filter_map(|c| c.happened_on)
            .collect();
        assert_eq!(dates, [day(2026, 7, 14), day(2026, 6, 10)]);
    }

    #[test]
    fn a_page_with_no_dated_entries_says_so() {
        let page = PageChanges::default();
        assert!(page.is_empty());
        assert!(page.recent(day(2026, 8, 4)).is_empty());
    }

    fn identity(year: Option<u16>, place: Option<&str>, people: Option<u32>) -> IdentityExtraction {
        IdentityExtraction {
            founded_year: year,
            headquarters: place.map(str::to_owned),
            employees: people,
            evidence_quote: None,
        }
    }

    #[test]
    fn a_year_the_section_never_wrote_is_not_from_it() {
        // The failure this check exists for. A model that has read about a company knows when
        // it was founded, and an about page full of story is exactly the prompt that invites
        // it to say so from memory.
        let section = "We're here for them, 23 years and running.";
        assert!(!identity(Some(2003), None, None).is_from(section));
        assert!(identity(None, None, None).is_from(section));
    }

    #[test]
    fn a_number_inside_another_number_is_not_a_statement_of_it() {
        // The check that was not a check. The model answered "0 people" for a page reading
        // "a team of 10", and a substring test let it through.
        let section = "Today Plausible is a team of 10.";
        assert!(!identity(None, None, Some(0)).is_from(section));
        assert!(identity(None, None, Some(10)).is_from(section));
        // And the same for a year inside a longer number.
        assert!(!identity(Some(2020), None, None).is_from("we have 120200 customers"));
    }

    #[test]
    fn a_headcount_and_a_place_are_checked_the_same_way() {
        let section = "Today Plausible is a team of 10, based in the EU.";
        assert!(identity(None, Some("the EU"), Some(10)).is_from(section));
        assert!(!identity(None, Some("Berlin"), None).is_from(section));
        assert!(!identity(None, None, Some(50)).is_from(section));
    }

    #[test]
    fn an_invented_field_does_not_take_a_correct_one_with_it() {
        // plausible.io/about states its founding year in the window it was asked about and
        // nothing else. Discarding the extraction whole threw the year away too.
        let section = "Uku Taht started Plausible in December 2018, building it alone.";
        let mixed = identity(Some(2018), Some("Berlin"), None);
        let (kept, dropped) = mixed.keeping_only_stated(section);
        assert_eq!(kept.founded_year, Some(2018));
        assert_eq!(kept.headquarters, None);
        assert_eq!(dropped, 1);
    }

    #[test]
    fn an_extraction_with_nothing_left_is_empty_rather_than_wrong() {
        let (kept, dropped) = identity(Some(2003), None, Some(80)).keeping_only_stated("no facts");
        assert!(kept.is_empty());
        assert_eq!(dropped, 2);
    }

    #[test]
    fn the_three_facts_are_merged_across_windows() {
        // One window per fact, so they arrive separately and each fills its own gap.
        let page = PageIdentity::assembled([
            identity(Some(2019), None, None),
            identity(None, Some("the EU"), None),
            identity(None, None, Some(10)),
        ]);
        assert_eq!(page.facts(), 3);
        assert_eq!(page.founded_year.map(|s| s.value), Some(2019));
        assert_eq!(
            page.headquarters.map(|s| s.value).as_deref(),
            Some("the EU")
        );
        assert_eq!(page.employees.map(|s| s.value), Some(10));
    }

    #[test]
    fn the_first_window_to_state_a_fact_keeps_it() {
        // Two windows disagreeing is a page saying two things, and the one it leads with is
        // the one it means.
        let page = PageIdentity::assembled([
            identity(None, None, Some(10)),
            identity(None, None, Some(400)),
        ]);
        assert_eq!(page.employees.map(|s| s.value), Some(10));
    }

    #[test]
    fn a_page_that_states_none_of_them_is_empty_rather_than_wrong() {
        let page = PageIdentity::assembled([IdentityExtraction::empty()]);
        assert!(page.is_empty());
        assert_eq!(page.facts(), 0);
    }

    #[test]
    fn a_quote_that_does_not_contain_the_fact_is_not_evidence_for_it() {
        // The model picks a neighbouring sentence out of the window it was given. The value
        // survives — it was checked against the whole window — and the quote does not.
        let got = IdentityExtraction {
            evidence_quote: Some("We built it because analytics had drifted".to_owned()),
            ..identity(None, Some("the EU"), None)
        };
        let page = PageIdentity::assembled([got]);
        let place = page.headquarters.unwrap();
        assert_eq!(place.value, "the EU");
        assert_eq!(place.quote, None, "kept a quote that does not say it");
    }

    #[test]
    fn a_fact_keeps_the_quote_it_arrived_with() {
        // The report caught this: quotes were held in one list and paired to facts by
        // position, so the founding sentence was rendered under "based in the EU". Evidence
        // for a claim it does not support is the one thing never to render.
        let founding = IdentityExtraction {
            evidence_quote: Some("started Plausible in December 2018".to_owned()),
            ..identity(Some(2018), None, None)
        };
        let place = IdentityExtraction {
            evidence_quote: Some("a company based in the EU".to_owned()),
            ..identity(None, Some("the EU"), None)
        };
        let page = PageIdentity::assembled([founding, place]);
        assert_eq!(
            page.founded_year.unwrap().quote.as_deref(),
            Some("started Plausible in December 2018")
        );
        assert_eq!(
            page.headquarters.unwrap().quote.as_deref(),
            Some("a company based in the EU")
        );
    }

    #[test]
    fn the_identity_schema_allows_every_field_to_be_absent() {
        let schema = serde_json::to_value(schemars::schema_for!(IdentityExtraction)).unwrap();
        let required = schema.get("required").and_then(|r| r.as_array());
        assert!(
            required.is_none_or(|r| r.is_empty()),
            "no field may be required: {schema:#}"
        );
    }

    fn capability(name: &str) -> FeatureExtraction {
        FeatureExtraction {
            capability: Some(name.to_owned()),
            ..FeatureExtraction::empty()
        }
    }

    #[test]
    fn a_capability_named_twice_is_one_capability() {
        // Pages summarise a feature and then describe it. That is one fact.
        let page = PageFeatures::assembled(
            [capability("Message Boards"), capability("message boards ")],
            2,
        );
        assert_eq!(page.features.len(), 1, "{page:?}");
    }

    #[test]
    fn a_window_that_named_nothing_is_not_a_capability() {
        let page = PageFeatures::assembled([FeatureExtraction::empty(), capability("Reports")], 2);
        assert_eq!(page.features.len(), 1);
    }

    #[test]
    fn the_number_considered_survives_assembly() {
        // Twelve read out of forty is a short list. Twelve with no number beside it is a
        // wrong one — FACT_CHECKING §5.4's rule about saying what was looked at.
        let page = PageFeatures::assembled([capability("Reports")], 40);
        assert_eq!(page.considered, 40);
    }

    #[test]
    fn a_qualifier_is_kept_because_it_changes_the_fact() {
        // A capability available only on the top tier is not the same fact as one that is
        // simply available, and a matrix that flattens the two is lying politely.
        let beta = FeatureExtraction {
            capability: Some("Code Intelligence".to_owned()),
            qualifier: Some("beta".to_owned()),
            evidence_quote: Some("Code Intelligence (beta)".to_owned()),
        };
        let page = PageFeatures::assembled([beta.clone()], 1);
        assert_eq!(page.features[0].qualifier.as_deref(), Some("beta"));
        assert!(beta.quote_is_verbatim("- Loops\n- Code Intelligence (beta)\n- Linear Insights"));
    }

    #[test]
    fn a_name_the_section_never_used_is_not_from_it() {
        // Both halves of this happened. `string` is the field's own type, returned by a 4B
        // model that could not name a section; `Message Boards` is a Basecamp feature the
        // model read out of an example in the prompt and reported for a Linear page.
        let section = "### SLA notifications
Get notified when an SLA is close to breaching.";
        for invented in ["string", "Message Boards"] {
            let got = capability(invented);
            assert!(!got.name_is_from(section), "accepted {invented}");
        }
        assert!(capability("SLA notifications").name_is_from(section));
    }

    #[test]
    fn a_shortened_name_is_still_from_the_section() {
        // The normalisation the model is there for: the words must come from the page, but
        // they need not be contiguous or in order.
        let section = "## Message Boards for announcements and discussions
They replace email.";
        assert!(capability("Message Boards").name_is_from(section));
    }

    #[test]
    fn naming_nothing_is_not_a_fabrication() {
        assert!(FeatureExtraction::empty().name_is_from("anything"));
    }

    #[test]
    fn a_paraphrased_capability_quote_is_not_verbatim() {
        let claimed = FeatureExtraction {
            evidence_quote: Some("Code Intelligence, in beta".to_owned()),
            ..FeatureExtraction::empty()
        };
        assert!(!claimed.quote_is_verbatim("- Code Intelligence (beta)"));
    }

    #[test]
    fn the_feature_schema_allows_every_field_to_be_absent() {
        // Same assertion as pricing, for the same reason: the grammar cannot emit a shape the
        // type forbids, so a required field would guarantee an invented capability.
        let schema = serde_json::to_value(schemars::schema_for!(FeatureExtraction)).unwrap();
        let required = schema.get("required").and_then(|r| r.as_array());
        assert!(
            required.is_none_or(|r| r.is_empty()),
            "no field may be required: {schema:#}"
        );
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
