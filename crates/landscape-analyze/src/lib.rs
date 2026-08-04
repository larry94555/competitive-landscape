//! One subject in, one report out.
//!
//! Every piece of the pipeline has been runnable on its own for weeks, and `landscape read`
//! has been printing what each of them decided. **That is a run log, not a report.** The
//! difference is not cosmetic:
//!
//! - A run log is ordered by *page*. A report is ordered by *question*, because that is what
//!   a reader came with.
//! - A run log's silence is invisible. A report's silence is a section carrying
//!   [`Coverage::note`] — `PRODUCT_SPEC.md` §4.3's "not found" treatment.
//! - A run log's facts have no citations. **Every [`Claim`] carries a source label and a
//!   verbatim quote**, and [`Report::every_claim_is_traceable`] refuses a report whose
//!   citation does not resolve.
//!
//! # What this does not do yet
//!
//! `PRODUCT_SPEC.md` §4 fixes nine sections, and this builds six — one per question kind that
//! discovery asks. The others (positioning, sentiment themes, market emphasis, the SWOT) are
//! **interpretation over sources this pipeline does not gather yet**, and a section that
//! exists but cannot be filled is worse than one that is honestly absent.
//!
//! [`Coverage::note`]: landscape_core::Coverage::note
//! [`Claim`]: landscape_core::Claim
//! [`Report::every_claim_is_traceable`]: landscape_core::Report::every_claim_is_traceable

use chrono::{DateTime, NaiveDate, Utc};
use landscape_core::{
    Change, Claim, Confidence, Coverage, Disposition, PageChanges, PageFeatures, PageIdentity,
    PagePricing, Report, Section, Source,
};
use landscape_discover::probes::Answers;

mod render;
mod sections;
mod stages;
pub mod subject;

pub use render::is_publishable;
pub use sections::{title_for, SECTIONS};

/// Everything one run produced: the report, and the trace that made it.
#[derive(Debug, Clone)]
pub struct Analysis {
    /// What a reader is shown.
    pub report: Report,
    /// What each of the six questions ended up with, including the ones with nothing.
    pub coverage: Vec<Coverage>,
    /// One entry per page the run touched, in the order it touched them.
    ///
    /// Kept because the pipeline's joins are still the thing most worth watching, and a
    /// report deliberately hides them. `landscape read` prints these; the product will not.
    pub pages: Vec<PageResult>,
}

/// What happened to one page.
#[derive(Debug, Clone)]
pub struct PageResult {
    pub url: String,
    pub question: Answers,
    /// `None` when the page was never opened.
    pub words: Option<usize>,
    pub quality: Option<&'static str>,
    /// How many words of it the model was actually shown.
    pub window_words: Option<usize>,
    /// One line, in the words the CLI has printed since Run 5.
    pub summary: String,
    /// The facts, the dropped answers, the model errors.
    pub details: Vec<String>,
}

/// The prompt set this crate speaks. Bumped when any extractor's wording changes.
pub const PROMPT_VERSION: u32 = 1;

/// Discover, read and extract one company, and assemble what came out.
///
/// `now` and `today` are arguments rather than clock reads. A report states when it was
/// generated and what fell inside a 90-day window, and neither can be tested against a
/// function that asks the operating system.
pub async fn analyse(
    fetcher: &landscape_fetch::Fetcher,
    llm: &landscape_llm::LlamaClient,
    origin: &str,
    now: DateTime<Utc>,
    today: NaiveDate,
) -> Analysis {
    analyse_with(fetcher, llm, origin, now, today, &mut |_| {}).await
}

/// The same, reporting the report so far after every page.
///
/// **A report is assembled a section at a time and a reader waits ninety seconds for it.**
/// `PRODUCT_SPEC.md` §2.1A wants the pricing section on screen when pricing is done rather
/// than everything at the end, and this callback is where that becomes possible: the worker
/// writes each partial report to the store, and the API streams the difference.
///
/// The partial report is **complete in shape from the first call** — all six sections exist,
/// each carrying its coverage note until it has claims. A reader never sees a section appear
/// out of nowhere; they see one fill in.
pub async fn analyse_with(
    fetcher: &landscape_fetch::Fetcher,
    llm: &landscape_llm::LlamaClient,
    origin: &str,
    now: DateTime<Utc>,
    today: NaiveDate,
    on_progress: &mut dyn FnMut(&Report),
) -> Analysis {
    let found = landscape_discover::discover(fetcher, origin).await;
    let model_ready = llm.is_ready().await;

    let mut pages: Vec<PageResult> = Vec::new();
    let mut sources: Vec<Source> = Vec::new();
    let mut claims: Vec<(Answers, Claim)> = Vec::new();
    let mut opened: Vec<(Answers, usize)> = Vec::new();

    for source in &found.sources {
        let question = source.answers;
        if !sections::has_extractor(question) {
            pages.push(PageResult {
                url: source.url.clone(),
                question,
                words: None,
                quality: None,
                window_words: None,
                summary: format!("no extractor yet for {} pages", question.name()),
                details: Vec::new(),
            });
            continue;
        }

        let Ok(page) = fetcher.get(&source.url).await else {
            pages.push(PageResult {
                url: source.url.clone(),
                question,
                words: None,
                quality: None,
                window_words: None,
                summary: "could not fetch".to_owned(),
                details: Vec::new(),
            });
            continue;
        };
        let markdown = landscape_extract::markdown::from_body(&page.body);
        let assessment = landscape_extract::quality::assess(&markdown);

        if !assessment.quality.worth_extracting() {
            pages.push(PageResult {
                url: source.url.clone(),
                question,
                words: Some(assessment.words),
                quality: Some(assessment.quality.name()),
                window_words: None,
                summary: "skipped - nothing to read".to_owned(),
                details: Vec::new(),
            });
            continue;
        }

        // A changelog is parsed, not generated, so it is read whether or not a model is
        // running — ARCHITECTURE §5.4. Everything else needs one.
        if question != Answers::Changes && !model_ready {
            pages.push(PageResult {
                url: source.url.clone(),
                question,
                words: Some(assessment.words),
                quality: Some(assessment.quality.name()),
                window_words: None,
                summary: "(no model)".to_owned(),
                details: Vec::new(),
            });
            continue;
        }

        opened.push((question, 1));
        let label = format!("S{}", sources.len() + 1);
        let outcome = stages::extract(llm, question, &source.url, &markdown, today).await;

        for text in outcome.claims {
            claims.push((
                question,
                Claim {
                    text: text.text,
                    source_label: label.clone(),
                    evidence_quote: text.quote,
                    confidence: text.confidence,
                    as_of: text.as_of.unwrap_or(now),
                },
            ));
        }
        pages.push(PageResult {
            url: source.url.clone(),
            question,
            words: Some(assessment.words),
            quality: Some(assessment.quality.name()),
            window_words: Some(outcome.window_words),
            summary: outcome.summary,
            details: outcome.details,
        });
        // Only pages that produced something are cited. A source list is what a reader
        // clicks through to; an entry that supports no claim is furniture.
        if claims.iter().any(|(_, c)| c.source_label == label) {
            sources.push(Source {
                label,
                url: source.url.clone(),
                title: page_title(&markdown, &source.url),
                disposition: Disposition::Primary,
                fetched_at: now,
                independence_group: origin.to_owned(),
            });
        }

        // One page done. Whoever is waiting can have what exists so far.
        let (so_far, _) = assemble(&found, &claims, &sources, &opened, origin, llm, now);
        on_progress(&so_far);
    }

    let (report, coverage) = assemble(&found, &claims, &sources, &opened, origin, llm, now);
    Analysis {
        report,
        coverage,
        pages,
    }
}

/// Build the report from what is known so far.
///
/// Called after every page as well as at the end, which is the point: a partial report and a
/// finished one are the same shape, and nothing downstream needs to know which it has.
fn assemble(
    found: &landscape_discover::Discovered,
    claims: &[(Answers, Claim)],
    sources: &[Source],
    opened: &[(Answers, usize)],
    origin: &str,
    llm: &landscape_llm::LlamaClient,
    now: DateTime<Utc>,
) -> (Report, Vec<Coverage>) {
    let coverage: Vec<Coverage> = sections::SECTIONS
        .iter()
        .map(|(question, _)| {
            let read = opened.iter().filter(|(q, _)| q == question).count();
            let facts = claims.iter().filter(|(q, _)| q == question).count();
            found.coverage(*question, read, facts)
        })
        .collect();

    let sections: Vec<Section> = coverage
        .iter()
        .zip(sections::SECTIONS.iter())
        .map(|(coverage, (question, title))| {
            let mut section = coverage.to_section(*title);
            section.claims = claims
                .iter()
                .filter(|(q, _)| q == question)
                .map(|(_, c)| c.clone())
                .collect();
            section
        })
        .collect();

    let report = Report {
        subject: origin.to_owned(),
        searched_as: origin.to_owned(),
        generated_at: now,
        // The server does not name its model over HTTP, so what is recorded is where the
        // answers came from. A report that cannot say which model wrote it is a report
        // nobody can reproduce, and this is the honest half of that.
        model_id: llm.base().to_owned(),
        prompt_version: PROMPT_VERSION,
        sections,
        sources: sources.to_vec(),
    };
    (report, coverage)
}

/// A page's own title, or its path if it has none.
fn page_title(markdown: &str, url: &str) -> String {
    markdown
        .lines()
        .find(|l| l.trim_start().starts_with("# "))
        .map(|l| l.trim().trim_start_matches('#').trim().to_owned())
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| url.to_owned())
}

/// One fact on its way to becoming a [`Claim`].
pub(crate) struct Finding {
    pub text: String,
    pub quote: String,
    pub confidence: Confidence,
    /// When the *source* says it happened, for a dated change. `None` means "when we read it".
    pub as_of: Option<DateTime<Utc>>,
}

/// What one page's extractor produced.
pub(crate) struct Outcome {
    pub claims: Vec<Finding>,
    pub summary: String,
    pub details: Vec<String>,
    pub window_words: usize,
}

/// Turn the four assembled page types into claims. Kept here so the wording of a claim is in
/// one place rather than three.
pub(crate) fn claims_from_pricing(page: &PagePricing) -> Vec<Finding> {
    page.plans
        .iter()
        .map(|plan| {
            let name = plan.plan_name.as_deref().unwrap_or("an unnamed plan");
            let text = match plan.price_usd {
                Some(price) => format!("{name} costs ${price}"),
                None => format!("{name} is listed with no published price"),
            };
            Finding {
                text,
                quote: plan.evidence_quote.clone().unwrap_or_default(),
                // A price is published to be read, and the quote is verbatim or the run said
                // so. This is the strongest kind of fact this pipeline produces.
                confidence: Confidence::High,
                as_of: None,
            }
        })
        .collect()
}

pub(crate) fn claims_from_features(page: &PageFeatures) -> Vec<Finding> {
    page.features
        .iter()
        .map(|feature| Finding {
            text: format!(
                "states it offers {}",
                feature
                    .capability
                    .as_deref()
                    .unwrap_or("an unnamed capability")
            ),
            quote: feature.evidence_quote.clone().unwrap_or_default(),
            // A capability name is a paraphrase by design — the model's job here is to
            // shorten a heading — so it can never carry the confidence a quoted price does.
            confidence: Confidence::Medium,
            as_of: None,
        })
        .collect()
}

pub(crate) fn claims_from_changes(page: &PageChanges, today: NaiveDate) -> Vec<Finding> {
    page.recent(today)
        .into_iter()
        .map(|change: &Change| Finding {
            text: change
                .summary
                .clone()
                .unwrap_or_else(|| "an undescribed change".to_owned()),
            quote: change.evidence_quote.clone().unwrap_or_default(),
            confidence: Confidence::High,
            // The date the page states, not the date we read it. A changelog entry that is
            // three weeks old should say so.
            as_of: change
                .happened_on
                .and_then(|d| d.and_hms_opt(0, 0, 0))
                .map(|dt| dt.and_utc()),
        })
        .collect()
}

pub(crate) fn claims_from_identity(page: &PageIdentity) -> Vec<Finding> {
    let mut out = Vec::new();
    // Each fact carries the quote it was read with. Pairing them by position put a founding
    // sentence under "based in the EU" — evidence for a claim it does not support.
    if let Some(founded) = &page.founded_year {
        out.push(Finding {
            text: format!("says it was founded in {}", founded.value),
            quote: founded.quote_or_empty(),
            confidence: Confidence::High,
            as_of: None,
        });
    }
    if let Some(place) = &page.headquarters {
        out.push(Finding {
            text: format!("says it is based in {}", place.value),
            quote: place.quote_or_empty(),
            confidence: Confidence::High,
            as_of: None,
        });
    }
    if let Some(people) = &page.employees {
        out.push(Finding {
            text: format!("says {} people work there", people.value),
            quote: people.quote_or_empty(),
            confidence: Confidence::High,
            as_of: None,
        });
    }
    out
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use landscape_core::{FeatureExtraction, PricingExtraction};

    #[test]
    fn a_priced_plan_becomes_a_claim_that_reads_as_a_sentence() {
        let page = PagePricing::assembled([PricingExtraction {
            plan_name: Some("Pro".to_owned()),
            price_usd: Some(15.0),
            billing_period: None,
            evidence_quote: Some("$15/user, billed monthly".to_owned()),
        }]);
        let claims = claims_from_pricing(&page);
        assert_eq!(claims.len(), 1);
        assert_eq!(claims[0].text, "Pro costs $15");
        assert_eq!(claims[0].quote, "$15/user, billed monthly");
    }

    #[test]
    fn a_plan_with_no_price_is_a_claim_about_the_absence() {
        // "Contact sales" is a fact about the company, and reporting it as nothing at all
        // would lose it.
        let page = PagePricing::assembled([PricingExtraction {
            plan_name: Some("Enterprise".to_owned()),
            ..PricingExtraction::empty()
        }]);
        assert!(claims_from_pricing(&page)[0]
            .text
            .contains("no published price"));
    }

    #[test]
    fn a_capability_is_never_as_confident_as_a_price() {
        // The name is a paraphrase by design — that is what the model is for here — so it
        // cannot carry the confidence a quoted number does.
        let page = PageFeatures::assembled(
            [FeatureExtraction {
                capability: Some("Message Boards".to_owned()),
                qualifier: None,
                evidence_quote: Some("## Message Boards".to_owned()),
            }],
            1,
        );
        assert_eq!(
            claims_from_features(&page)[0].confidence,
            Confidence::Medium
        );
    }

    #[test]
    fn a_change_is_dated_by_the_page_rather_than_by_the_run() {
        let today = NaiveDate::from_ymd_opt(2026, 8, 4).unwrap();
        let page = PageChanges {
            changes: vec![Change {
                happened_on: NaiveDate::from_ymd_opt(2026, 7, 14),
                summary: Some("Shipped annotations".to_owned()),
                evidence_quote: Some("- Jul 14, 2026".to_owned()),
            }],
            considered: 1,
        };
        let claims = claims_from_changes(&page, today);
        assert_eq!(claims.len(), 1);
        assert_eq!(
            claims[0].as_of.map(|d| d.date_naive()),
            NaiveDate::from_ymd_opt(2026, 7, 14)
        );
    }

    #[test]
    fn a_change_outside_the_window_is_not_a_claim() {
        let today = NaiveDate::from_ymd_opt(2026, 8, 4).unwrap();
        let page = PageChanges {
            changes: vec![Change {
                happened_on: NaiveDate::from_ymd_opt(2025, 1, 1),
                summary: Some("Ancient history".to_owned()),
                evidence_quote: None,
            }],
            considered: 1,
        };
        assert!(claims_from_changes(&page, today).is_empty());
    }

    #[test]
    fn each_identity_fact_is_its_own_claim() {
        use landscape_core::Stated;
        let page = PageIdentity {
            founded_year: Some(Stated {
                value: 2018,
                quote: Some("started Plausible in December 2018".to_owned()),
            }),
            headquarters: Some(Stated {
                value: "the EU".to_owned(),
                quote: Some("a company based in the EU".to_owned()),
            }),
            employees: Some(Stated {
                value: 10,
                quote: Some("a team of 10".to_owned()),
            }),
        };
        let claims = claims_from_identity(&page);
        assert_eq!(claims.len(), 3);
        assert!(claims[0].text.contains("2018"));
        assert!(claims[1].text.contains("the EU"));
        assert!(claims[2].text.contains("10 people"));
        // And each carries its own words, not the ones before it.
        assert_eq!(claims[1].quote, "a company based in the EU");
    }

    #[test]
    fn a_page_title_falls_back_to_the_url() {
        assert_eq!(page_title("# Pricing\n$15", "https://e.com/x"), "Pricing");
        assert_eq!(
            page_title("no heading", "https://e.com/x"),
            "https://e.com/x"
        );
    }
}
