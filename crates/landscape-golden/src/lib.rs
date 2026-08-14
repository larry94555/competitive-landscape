//! The golden set: fixed pages, known answers, and a score.
//!
//! `docs/ROADMAP.md` Phase 0 asks for *"10 golden-set subjects with frozen fixtures and
//! human-written reference sheets"*. This is that, and the reason it is a Phase 0 item
//! rather than a Phase 1 one is a thing we found by accident:
//!
//! > A defective quantization of Qwen3-1.7B passed every check we had. It answered
//! > quickly, it never failed to parse, and its output was schema-valid throughout. It
//! > was also wrong. Latency and shape tests cannot see that, and those were all we had.
//!
//! **Constrained decoding guarantees the shape of an answer and says nothing about its
//! truth.** Everything in `landscape-llm` is about shape. This crate is the other half.
//!
//! # What is being measured
//!
//! Four things, of which only the first is usually reported and the second matters most:
//!
//! | Measure | Question |
//! |---|---|
//! | Accuracy | Of the fields the page does contain, how many came back right? |
//! | **Fabrication** | How often did a field come back filled when the page says nothing? |
//! | Miss rate | How often did a fact on the page come back empty? |
//! | Quote fidelity | Is the evidence quote really a substring of the page? |
//!
//! Fabrication is the one that decides whether this product can exist. A report that
//! invents a competitor's price is worse than no report, because a reader cannot tell
//! which figures to check. Three of the ten subjects publish no price at all, and one of
//! those three puts a real price for a *different* product on the same page.
//!
//! Quote fidelity is worth its own column because it needs no reference answer: a quote
//! that is not in the source is fabricated evidence, and that is decidable by `contains`.
//! It generalizes to pages nobody has hand-labeled, which is what makes it the check we
//! can afford to run on everything later.
//!
//! # Running it
//!
//! ```text
//! cargo test -p landscape-golden                        # validates the set; no model needed
//! LLAMA_URL=http://127.0.0.1:8080 \
//!   cargo test -p landscape-golden -- --ignored --nocapture   # scores a model
//! ```

pub mod discovery;
pub mod pages;

use std::path::{Path, PathBuf};

use landscape_core::{BillingPeriod, PricingExtraction};
use serde::Deserialize;

/// Bumped whenever [`prompt_for`] changes.
///
/// Scores from two prompt versions are not comparable, and a table that mixes them silently
/// is worse than no table. Every scorecard prints this.
pub const PROMPT_VERSION: u32 = 2;

/// What a correct extraction of one subject contains.
#[derive(Debug, Clone, Deserialize)]
pub struct Expected {
    pub plan_name: Option<String>,
    pub price_usd: Option<f64>,
    pub billing_period: Option<BillingPeriod>,
}

/// One frozen page, and the answer a careful human read off it.
#[derive(Debug, Clone, Deserialize)]
pub struct Subject {
    pub id: String,
    /// Why this subject is in the set — the trap it sets, in prose.
    ///
    /// Required, and checked to be substantial. A subject nobody can justify is a subject
    /// that will be quietly deleted the first time it fails, which is exactly when it was
    /// doing its job.
    pub why: String,
    /// The plan being asked about, as it appears on the page.
    pub ask: String,
    /// The page, as it would look after boilerplate stripping. Frozen: never re-fetched.
    pub source: String,
    pub expect: Expected,
}

impl Subject {
    /// Whether this subject expects a price to be found.
    ///
    /// Drives two things: whether a filled price counts as a fabrication, and whether a
    /// quote is required. Evidence is required exactly when there is a claim to support.
    #[must_use]
    pub const fn expects_a_price(&self) -> bool {
        self.expect.price_usd.is_some()
    }
}

/// Where the fixtures live.
#[must_use]
pub fn subjects_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("subjects")
}

/// Load every subject, in filename order.
///
/// Filenames are numbered so the order is stable: a scorecard whose rows move between runs
/// is much harder to diff, and diffing two scorecards is the main thing anyone does with
/// them.
///
/// # Errors
/// If the directory cannot be read, or any file is not a valid [`Subject`].
pub fn load() -> Result<Vec<Subject>, String> {
    let dir = subjects_dir();
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&dir)
        .map_err(|e| format!("cannot read {}: {e}", dir.display()))?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    paths.sort();

    paths
        .iter()
        .map(|path| {
            let text = std::fs::read_to_string(path)
                .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
            serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
        })
        .collect()
}

/// The prompt one subject is asked with.
///
/// Kept here rather than in the test so that the prompt is part of the fixture: changing it
/// changes the score, and a score whose prompt lives somewhere else is not reproducible.
///
/// # A line about overfitting
///
/// The rules below are *domain* rules — they would be written the same way if this set did
/// not exist. That is the line worth holding. Adding "the Enterprise plan on this page has
/// no price" would raise the score and measure nothing; stating that a billing period
/// cannot exist without a price is true of every pricing page there has ever been.
///
/// The period rule was added at v2 after both Qwen3-1.7B and Qwen3-4B returned
/// `billing_period: "monthly"` for plans they had correctly reported as having no published
/// price. The type cannot express that constraint — see the note in
/// [`landscape_core::extract`] on why the obvious fix, a tagged union, measured *worse*.
#[must_use]
pub fn prompt_for(subject: &Subject) -> String {
    format!(
        "You are reading one page from a company's website and extracting what it says \
         about the pricing of a single plan.\n\n\
         Plan to extract: {ask}\n\n\
         Rules:\n\
         - Use only what this page states. Do not use anything you know from elsewhere.\n\
         - If the page does not state something, leave that field null. A missing price is \
         a fact worth reporting; a guessed one is not.\n\
         - Report the price of this plan only. A price on this page for a different plan, \
         a different product, or an add-on is not this plan's price.\n\
         - If price_usd is null then billing_period must be null too: a plan with no \
         published price has no billing period.\n\
         - price_usd is in US dollars. Ignore prices given in other currencies.\n\
         - evidence_quote must be copied from the page word for word.\n\n\
         PAGE\n\
         ---\n\
         {source}\n\
         ---\n\n\
         Return the extraction as JSON.",
        ask = subject.ask,
        source = subject.source,
    )
}

/// How one field came back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldVerdict {
    /// Right, including correctly left empty.
    Correct,
    /// The page says nothing, and the model filled it in anyway. The dangerous one.
    Fabricated,
    /// The page says it, and the model left it empty. Costs coverage, not trust.
    Missed,
    /// Both present, and different.
    Wrong,
}

impl FieldVerdict {
    const fn symbol(self) -> &'static str {
        match self {
            Self::Correct => "ok  ",
            Self::Fabricated => "FAB ",
            Self::Missed => "miss",
            Self::Wrong => "WRNG",
        }
    }
}

/// How the evidence quote came back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuoteVerdict {
    /// Present, and a substring of the page.
    Verbatim,
    /// Present, and not on the page. Fabricated evidence.
    NotOnPage,
    /// Absent where a claim was made, so the claim is unsupported.
    MissingForAClaim,
    /// Absent, and nothing was claimed. Correct.
    NotNeeded,
}

impl QuoteVerdict {
    const fn symbol(self) -> &'static str {
        match self {
            Self::Verbatim => "ok  ",
            Self::NotOnPage => "FAB ",
            Self::MissingForAClaim => "miss",
            Self::NotNeeded => "n/a ",
        }
    }
}

/// How one subject came back.
#[derive(Debug, Clone)]
pub struct SubjectResult {
    pub id: String,
    pub plan_name: FieldVerdict,
    pub price: FieldVerdict,
    pub billing_period: FieldVerdict,
    pub quote: QuoteVerdict,
    pub latency_ms: u128,
    /// Exactly what the model returned.
    ///
    /// Kept because a verdict without the value behind it is not diagnosable: `WRNG` tells
    /// you to go and re-run the case by hand, and re-running by hand is the step that does
    /// not happen. The point of a golden set is to make a failure explain itself.
    pub got: PricingExtraction,
}

impl SubjectResult {
    /// Whether a *price* was invented for a page that publishes none.
    ///
    /// Separated from the other fabrications because it is the one that decides whether
    /// this product can ship. A wrong billing period on a plan that has no price is an
    /// error; a dollar figure attached to a competitor who never published one is a false
    /// claim about a named company, and a reader has no way to know it is false.
    #[must_use]
    pub fn invented_a_price(&self) -> bool {
        self.price == FieldVerdict::Fabricated
    }

    /// Whether anything was invented — a field or a quote.
    #[must_use]
    pub fn fabricated(&self) -> bool {
        self.field_verdicts().contains(&FieldVerdict::Fabricated)
            || self.quote == QuoteVerdict::NotOnPage
    }

    /// Whether every field and the quote are right.
    #[must_use]
    pub fn perfect(&self) -> bool {
        self.field_verdicts()
            .iter()
            .all(|v| *v == FieldVerdict::Correct)
            && matches!(self.quote, QuoteVerdict::Verbatim | QuoteVerdict::NotNeeded)
    }

    const fn field_verdicts(&self) -> [FieldVerdict; 3] {
        [self.plan_name, self.price, self.billing_period]
    }
}

/// Score one extraction against its reference.
///
/// Pure, so the scoring rules are unit-tested without a model running. The rules are the
/// part most likely to be wrong in a way that flatters a result, and a scoring bug that
/// only shows up when a GPU is warm is a scoring bug nobody finds.
#[must_use]
pub fn score(subject: &Subject, got: &PricingExtraction, latency_ms: u128) -> SubjectResult {
    SubjectResult {
        id: subject.id.clone(),
        plan_name: verdict(
            subject.expect.plan_name.as_deref(),
            got.plan_name.as_deref(),
            same_name,
        ),
        price: verdict(subject.expect.price_usd, got.price_usd, same_price),
        billing_period: verdict(subject.expect.billing_period, got.billing_period, |a, b| {
            a == b
        }),
        quote: quote_verdict(subject, got),
        latency_ms,
        got: got.clone(),
    }
}

fn verdict<T>(expected: Option<T>, got: Option<T>, same: impl Fn(T, T) -> bool) -> FieldVerdict {
    match (expected, got) {
        (None, None) => FieldVerdict::Correct,
        (None, Some(_)) => FieldVerdict::Fabricated,
        (Some(_), None) => FieldVerdict::Missed,
        (Some(e), Some(g)) => {
            if same(e, g) {
                FieldVerdict::Correct
            } else {
                FieldVerdict::Wrong
            }
        }
    }
}

/// Whether a quote field actually holds a quote.
///
/// Models asked for a nullable string sometimes write the *word* rather than the value —
/// `"evidence_quote": "null"`, or `"N/A"`. That is a formatting slip on the way to
/// abstaining, and scoring it as fabricated evidence would put a model that declined to
/// quote anything in the same column as one that invented a sentence. Those need telling
/// apart: the first is a prompt to fix, the second disqualifies the model.
///
/// Reading this too generously has a real cost, so it is deliberately narrow — only when
/// the placeholder is the *entire* field. A quote that contains the word "none" among
/// other words is still a quote, and still checked against the page.
fn is_placeholder(quote: &str) -> bool {
    matches!(
        quote.trim().trim_matches('"').to_lowercase().as_str(),
        "" | "null" | "none" | "n/a" | "na" | "not stated" | "not available"
    )
}

fn quote_verdict(subject: &Subject, got: &PricingExtraction) -> QuoteVerdict {
    let has_quote = got
        .evidence_quote
        .as_deref()
        .is_some_and(|q| !is_placeholder(q));

    if !has_quote {
        return if subject.expects_a_price() {
            QuoteVerdict::MissingForAClaim
        } else {
            QuoteVerdict::NotNeeded
        };
    }
    if got.quote_is_verbatim(&subject.source) {
        QuoteVerdict::Verbatim
    } else {
        QuoteVerdict::NotOnPage
    }
}

/// Prices are equal when they are equal to the cent.
///
/// `clippy::float_cmp` is denied workspace-wide for good reason, and this is the one place
/// that needs an answer rather than an exemption: money read off a page is exact to two
/// decimal places, so a half-cent tolerance separates "the same price" from "a different
/// price" without ever calling two real prices equal.
fn same_price(a: f64, b: f64) -> bool {
    (a - b).abs() < 0.005
}

/// Plan names match loosely, on purpose.
///
/// "Grower", "grower" and "Grower plan" are the same finding, and a set that scored those
/// as three different answers would measure string formatting instead of reading. The
/// price field carries the precision; this one carries the identification.
fn same_name(a: &str, b: &str) -> bool {
    let (a, b) = (normalize(a), normalize(b));
    !a.is_empty() && !b.is_empty() && (a.contains(&b) || b.contains(&a))
}

fn normalize(s: &str) -> String {
    s.to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Everything one model scored, across the whole set.
#[derive(Debug, Clone)]
pub struct Scorecard {
    pub label: String,
    pub results: Vec<SubjectResult>,
    /// Subjects that never produced an extraction at all — unreachable server, unparseable
    /// output. Counted apart from wrong answers, because "the model is broken" and "the
    /// model is mistaken" call for entirely different responses.
    pub errors: Vec<(String, String)>,
}

impl Scorecard {
    #[must_use]
    pub fn fabrications(&self) -> usize {
        self.results.iter().filter(|r| r.fabricated()).count()
    }

    /// Subjects where a price was invented. The number that has to be zero.
    #[must_use]
    pub fn invented_prices(&self) -> usize {
        self.results.iter().filter(|r| r.invented_a_price()).count()
    }

    #[must_use]
    pub fn perfect_subjects(&self) -> usize {
        self.results.iter().filter(|r| r.perfect()).count()
    }

    /// Correct fields over fields scored. Quote fidelity is reported separately.
    #[must_use]
    pub fn field_accuracy(&self) -> f64 {
        let total = self.results.len() * 3;
        if total == 0 {
            return 0.0;
        }
        let correct: usize = self
            .results
            .iter()
            .map(|r| {
                r.field_verdicts()
                    .iter()
                    .filter(|v| **v == FieldVerdict::Correct)
                    .count()
            })
            .sum();
        #[allow(clippy::cast_precision_loss)]
        {
            correct as f64 / total as f64
        }
    }

    #[must_use]
    pub fn median_latency_ms(&self) -> u128 {
        let mut all: Vec<u128> = self.results.iter().map(|r| r.latency_ms).collect();
        if all.is_empty() {
            return 0;
        }
        all.sort_unstable();
        all[all.len() / 2]
    }

    /// A fixed-width table, meant to be pasted into `docs/BENCHMARKS.md`.
    #[must_use]
    pub fn render(&self) -> String {
        use std::fmt::Write as _;
        let mut out = String::new();
        let _ = writeln!(
            out,
            "\n{}  (prompt v{PROMPT_VERSION})\n{}",
            self.label,
            "-".repeat(64)
        );
        let _ = writeln!(out, "{:<34} plan price period quote", "subject");
        for r in &self.results {
            let _ = writeln!(
                out,
                "{:<34} {} {}  {}   {}  {:>6} ms",
                r.id,
                r.plan_name.symbol(),
                r.price.symbol(),
                r.billing_period.symbol(),
                r.quote.symbol(),
                r.latency_ms,
            );
        }
        for (id, err) in &self.errors {
            let _ = writeln!(out, "{id:<34} ERROR: {err}");
        }
        let _ = writeln!(
            out,
            "{}\nfields correct {:.0}%   perfect {}/{}   median {} ms\n\
             invented prices {}   any fabrication {}",
            "-".repeat(64),
            self.field_accuracy() * 100.0,
            self.perfect_subjects(),
            self.results.len(),
            self.median_latency_ms(),
            self.invented_prices(),
            self.fabrications(),
        );
        if !self.errors.is_empty() {
            let _ = writeln!(out, "{} subject(s) errored", self.errors.len());
        }
        out
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn subject(price: Option<f64>) -> Subject {
        Subject {
            id: "t".into(),
            why: "a test".into(),
            ask: "Grower".into(),
            source: "The Grower plan is $49 per month.".into(),
            expect: Expected {
                plan_name: Some("Grower".into()),
                price_usd: price,
                billing_period: price.map(|_| BillingPeriod::Monthly),
            },
        }
    }

    fn got(price: Option<f64>, quote: Option<&str>) -> PricingExtraction {
        PricingExtraction {
            plan_name: Some("Grower".into()),
            price_usd: price,
            billing_period: price.map(|_| BillingPeriod::Monthly),
            evidence_quote: quote.map(str::to_owned),
        }
    }

    #[test]
    fn a_correct_extraction_is_perfect() {
        let r = score(
            &subject(Some(49.0)),
            &got(Some(49.0), Some("$49 per month")),
            0,
        );
        assert!(r.perfect());
        assert!(!r.fabricated());
    }

    #[test]
    fn filling_in_a_price_the_page_does_not_state_is_a_fabrication() {
        // The failure mode the whole set exists to catch.
        let r = score(&subject(None), &got(Some(49.0), Some("$49 per month")), 0);
        assert_eq!(r.price, FieldVerdict::Fabricated);
        assert!(r.fabricated());
    }

    #[test]
    fn leaving_out_a_price_the_page_states_is_a_miss_not_a_fabrication() {
        // Costs coverage, not trust. Scored differently because it is answered
        // differently: a miss is a prompt problem, a fabrication is a model problem.
        let r = score(&subject(Some(49.0)), &got(None, None), 0);
        assert_eq!(r.price, FieldVerdict::Missed);
        assert!(!r.fabricated());
    }

    #[test]
    fn a_quote_that_is_not_on_the_page_is_a_fabrication() {
        let r = score(
            &subject(Some(49.0)),
            &got(Some(49.0), Some("$49 monthly")),
            0,
        );
        assert_eq!(r.quote, QuoteVerdict::NotOnPage);
        assert!(r.fabricated());
        assert!(!r.perfect());
    }

    #[test]
    fn a_claim_with_no_quote_is_unsupported() {
        let r = score(&subject(Some(49.0)), &got(Some(49.0), None), 0);
        assert_eq!(r.quote, QuoteVerdict::MissingForAClaim);
        assert!(!r.perfect());
    }

    #[test]
    fn no_claim_and_no_quote_needs_nothing() {
        let mut g = got(None, None);
        g.plan_name = Some("Grower".into());
        let r = score(&subject(None), &g, 0);
        assert_eq!(r.quote, QuoteVerdict::NotNeeded);
        assert!(r.perfect());
    }

    #[test]
    fn a_free_tier_is_not_scored_as_an_absence() {
        // Some(0.0) against an expected None must read as a fabrication, not a match.
        // If f64 defaulting ever crept in, this is the test that catches it.
        let r = score(&subject(None), &got(Some(0.0), None), 0);
        assert_eq!(r.price, FieldVerdict::Fabricated);
    }

    #[test]
    fn a_price_out_by_a_cent_is_wrong() {
        let r = score(
            &subject(Some(49.0)),
            &got(Some(49.01), Some("$49 per month")),
            0,
        );
        assert_eq!(r.price, FieldVerdict::Wrong);
    }

    #[test]
    fn the_word_null_in_a_quote_field_is_an_abstention_not_a_fabrication() {
        // Observed: Qwen3-4B returns "evidence_quote": "null" when it has nothing to
        // quote. Calling that fabricated evidence would rank it alongside a model that
        // invented a sentence, and those two failures are answered differently.
        let r = score(&subject(None), &got(None, Some("null")), 0);
        assert_eq!(r.quote, QuoteVerdict::NotNeeded);
        assert!(!r.fabricated());
    }

    #[test]
    fn a_quote_merely_containing_the_word_none_is_still_checked() {
        // The leniency above must not become a way to smuggle a fabricated sentence past
        // the check by mentioning "none" in it.
        let r = score(
            &subject(Some(49.0)),
            &got(Some(49.0), Some("none of this text is on the page")),
            0,
        );
        assert_eq!(r.quote, QuoteVerdict::NotOnPage);
    }

    #[test]
    fn plan_names_match_across_case_and_a_trailing_word() {
        assert!(same_name("Grower", "grower plan"));
        assert!(same_name("Harvest Plan", "harvest"));
        assert!(!same_name("Grower", "Harvest"));
        assert!(!same_name("Grower", ""));
    }

    #[test]
    fn the_scorecard_reports_zero_rather_than_dividing_by_zero() {
        let empty = Scorecard {
            label: "none".into(),
            results: vec![],
            errors: vec![],
        };
        assert!(empty.field_accuracy() < f64::EPSILON);
        assert_eq!(empty.median_latency_ms(), 0);
        assert!(empty.render().contains("fields correct 0%"));
    }
}
