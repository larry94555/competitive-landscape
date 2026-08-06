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

mod order;
mod render;
mod sections;
mod stages;
pub mod subject;

pub use order::{
    is_deterministic, model_calls_for, plan, yields_content, Plan, PAGES_PER_QUESTION,
};
pub use render::is_publishable;
pub use sections::{title_for, SECTIONS};

/// Whether the caller still wants what this run is producing.
///
/// **A worker can be replaced while it is working.** The staleness sweep hands its row to
/// somebody else, every write it makes from then on is refused, and until this existed it kept
/// reading pages and calling a model for a report that would be thrown away — up to a hundred
/// and eighty seconds of the machine's only scarce resource, spent on nothing.
///
/// Returned from the progress callback because that is where the answer already is: the caller
/// finds out its claim is gone *by writing*, and the progress callback is the write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wanted {
    /// Carry on.
    Yes,
    /// Stop. Nobody is waiting for this any more.
    No,
}

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
    /// Whether the run was abandoned because the caller said [`Wanted::No`].
    ///
    /// The report is whatever had been assembled at that point, and **it should not be
    /// published**: it is a partial answer from a worker that has already been replaced.
    pub stopped_early: bool,
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
    analyse_with(fetcher, llm, origin, now, today, &mut |_| Wanted::Yes).await
}

/// Several companies, one report.
///
/// **A profile of one company is not a competitive analysis.** Until this existed the pipeline
/// took the first site named in a prompt and dropped the rest, so `basecamp.com vs linear.app`
/// produced a report about Basecamp with nothing on the page saying the other had been ignored.
///
/// # Why this merges rather than generalising `analyse_with`
///
/// One company is the unit that discovery, fetching and extraction are built around, and that
/// path is the one with every test behind it. This runs it once per subject and joins the
/// results, so nothing about reading a single company changes — and a bug in the joining cannot
/// reach into the reading.
///
/// # What joining means
///
/// Sections merge by question: *What it costs* holds every company's prices, which is what
/// makes the report a comparison rather than several reports stapled together. Source labels
/// are reassigned as they merge, because each run numbers its own sources from `S1` and two
/// runs would otherwise both claim it.
///
/// Progress is reported after every window of every subject, over the *whole* report so far, so
/// a reader watching sees the first company fill in while the second is still being read.
pub async fn analyse_many(
    fetcher: &landscape_fetch::Fetcher,
    llm: &landscape_llm::LlamaClient,
    origins: &[String],
    now: DateTime<Utc>,
    today: NaiveDate,
    on_progress: &mut dyn FnMut(&Report) -> Wanted,
) -> Analysis {
    // The cap is applied here rather than in the parser, so what it costs can be said out
    // loud. Dropping the fourth company in silence would be the defect this feature exists to
    // remove, at a higher count.
    let (analysing, dropped) = origins.split_at(origins.len().min(subject::MAX_SUBJECTS));
    let notes = if dropped.is_empty() {
        Vec::new()
    } else {
        vec![format!(
            "Comparing the first {} sites named. Not analysed: {}. Each company is its own              discovery, fetches and model calls, so more of them is a longer wait rather than              a bigger report - run them separately if you need all of these.",
            analysing.len(),
            dropped.join(", ")
        )]
    };
    let origins = analysing;

    let mut finished: Vec<Analysis> = Vec::with_capacity(origins.len());

    for origin in origins {
        let stopped = {
            let so_far = &finished;
            let mut merge_and_report = |partial: &Report| {
                on_progress(&joined(
                    so_far,
                    Some(partial),
                    origins,
                    notes.clone(),
                    llm,
                    now,
                ))
            };
            let one = analyse_with(fetcher, llm, origin, now, today, &mut merge_and_report).await;
            let stopped = one.stopped_early;
            finished.push(one);
            stopped
        };
        // A revoked claim stops the whole report, not just the company being read: the run has
        // been given to somebody else, and the remaining subjects are the largest part of what
        // there is left to save.
        if stopped {
            break;
        }
    }

    let report = joined(&finished, None, origins, notes, llm, now);
    Analysis {
        report,
        coverage: coverage_by_question(&finished),
        pages: finished.iter().flat_map(|a| a.pages.clone()).collect(),
        stopped_early: finished.iter().any(|a| a.stopped_early),
    }
}

/// One coverage record per question, however many companies were read.
///
/// **`Analysis::render` zips sections with coverage**, and the merged report has six sections
/// however many subjects it covers. Concatenating each company's six would leave the renderer
/// consuming the first company's and silently ignoring every other company's negative evidence
/// — so a section that found nothing would describe only the first site's attempts, which is
/// this project's honest-negative treatment failing in exactly the place it matters.
///
/// Attribution survives the merge because every [`landscape_core::Attempt`] carries the origin
/// it was tried against. It is **not** carried by the path: `attempts_for` stores `/pricing`,
/// not `https://basecamp.com/pricing`, because the note used to be read under a heading that
/// already named the company. I wrote the opposite here, and review checked the producer.
fn coverage_by_question(finished: &[Analysis]) -> Vec<Coverage> {
    let mut merged: Vec<Coverage> = Vec::new();
    for one in finished.iter().flat_map(|a| a.coverage.iter()) {
        match merged.iter_mut().find(|c| c.question == one.question) {
            Some(into) => {
                into.sources.extend(one.sources.iter().cloned());
                into.attempts.extend(one.attempts.iter().cloned());
                into.pages_read += one.pages_read;
                into.facts += one.facts;
            }
            None => merged.push(one.clone()),
        }
    }
    merged
}

/// One report from several, with source labels reassigned so none collides.
fn joined(
    finished: &[Analysis],
    in_flight: Option<&Report>,
    origins: &[String],
    notes: Vec<String>,
    llm: &landscape_llm::LlamaClient,
    now: DateTime<Utc>,
) -> Report {
    let mut sections: Vec<Section> = Vec::new();
    let mut sources: Vec<Source> = Vec::new();
    // From the companies this run set out to cover — the same reading as `Report::subjects`,
    // and for the same reason: a company that says nothing is still one of the two being
    // compared, and the one that spoke keeps its label.
    let several = origins.len() > 1;

    for report in finished.iter().map(|a| &a.report).chain(in_flight) {
        // Every source this report brought, renumbered from where the last one left off.
        let mut renamed: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        for source in &report.sources {
            let label = format!("S{}", sources.len() + 1);
            renamed.insert(source.label.clone(), label.clone());
            sources.push(Source {
                label,
                ..source.clone()
            });
        }

        for section in &report.sections {
            // **The lines a reader sees, which are not the merged `Coverage`.** Each company is
            // assembled on its own, so `to_section` rendered `/pricing (404)` while it was still
            // true that one company was all there was. Joining those strings leaves the web
            // report — which renders `section.checked` and never sees a `Coverage` — with two
            // identical paths and no way to tell whose gap is whose. Review found this after
            // the structured coverage had already been fixed, which is the lesson: the surface
            // a person looks at is its own assertion.
            let checked: Vec<String> = section
                .checked
                .iter()
                .map(|line| {
                    if several {
                        landscape_core::attributed(&report.subject, line)
                    } else {
                        line.clone()
                    }
                })
                .collect();

            let claims: Vec<Claim> = section
                .claims
                .iter()
                .map(|claim| Claim {
                    // A label with no new name is one the source list never had, which
                    // `Report::every_claim_is_traceable` refuses. Leaving it unchanged keeps
                    // that refusal working rather than inventing a label that resolves.
                    source_label: renamed
                        .get(&claim.source_label)
                        .cloned()
                        .unwrap_or_else(|| claim.source_label.clone()),
                    // **Stamped here rather than where the claim was built.** Each run's
                    // report knows the company it is about; the extractors do not need to, and
                    // putting it here means the one place that has to get it right is the one
                    // place with tests around it.
                    subject: report.subject.clone(),
                    ..claim.clone()
                })
                .collect();

            match sections.iter_mut().find(|s| s.key == section.key) {
                Some(existing) => {
                    existing.claims.extend(claims);
                    existing.checked.extend(checked);
                    existing.notes.extend(section.notes.iter().cloned());
                    if !existing.claims.is_empty() {
                        existing.status = landscape_core::SectionStatus::Populated;
                    }
                }
                None => {
                    let mut merged = section.clone();
                    merged.claims = claims;
                    merged.checked = checked;
                    sections.push(merged);
                }
            }
        }
    }

    // **Each company's own notes travel with it.** The budget note is written per company -
    // one may have had a second pricing page and the other not - and a merge that kept only
    // the caller's notes would drop the sentence saying what was left unread, which is the
    // half of the shorter wait a reader is owed.
    let mut notes = notes;
    for report in finished.iter().map(|a| &a.report).chain(in_flight) {
        for note in &report.notes {
            if !notes.contains(note) {
                notes.push(note.clone());
            }
        }
    }

    Report {
        subject: origins.join(", "),
        searched_as: origins.join(", "),
        generated_at: now,
        model_id: llm.base().to_owned(),
        prompt_version: PROMPT_VERSION,
        // The companies this report set out to cover, which is not the same list as the
        // companies that produced a claim — and the difference is the case that matters.
        subjects: origins.to_vec(),
        sections,
        sources,
        notes,
    }
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
    on_progress: &mut dyn FnMut(&Report) -> Wanted,
) -> Analysis {
    let found = landscape_discover::discover(fetcher, origin).await;
    let model_ready = llm.is_ready().await;

    // **When each page is read, and how many are read at all** — `order`. The page that needs
    // no model goes first so the first thing on screen costs a fetch rather than a chain of
    // model calls, and each question is worth one page that does need one. The pages that are
    // left are named on the report rather than dropped in silence.
    let plan = order::plan(&found.sources);
    let notes: Vec<String> = plan.note().into_iter().collect();

    let reading = Reading {
        found: &found,
        origin,
        llm,
        now,
        notes: &notes,
    };

    let mut pages: Vec<PageResult> = Vec::new();
    let mut sources: Vec<Source> = Vec::new();
    let mut claims: Vec<(Answers, Claim)> = Vec::new();
    let mut opened: Vec<(Answers, usize)> = Vec::new();
    let mut stopped_early = false;

    for source in &plan.read {
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
        // Each fact this page produces, as it produces it. A page is too large a unit to
        // wait on: plausible.io's first page is twelve windows and took four minutes, which
        // a reader watching an empty screen reads as nothing happening.
        let outcome = {
            let claims = &claims;
            let sources = &sources;
            let opened = &opened;
            let label = label.clone();
            let mut emit = |partial: &[Finding]| {
                let mut with_partial: Vec<(Answers, Claim)> = claims.clone();
                with_partial.extend(partial.iter().map(|f| {
                    (
                        question,
                        Claim {
                            text: f.text.clone(),
                            subject: String::new(),
                            source_label: label.clone(),
                            evidence_quote: f.quote.clone(),
                            confidence: f.confidence,
                            as_of: f.as_of.unwrap_or(now),
                        },
                    )
                }));
                // The source is cited from the first claim that needs it, so nothing a
                // reader sees mid-run points at a label the source list does not have.
                let mut with_source = sources.to_vec();
                if !partial.is_empty() && !with_source.iter().any(|s| s.label == label) {
                    with_source.push(Source {
                        label: label.clone(),
                        url: source.url.clone(),
                        title: page_title(&markdown, &source.url),
                        disposition: Disposition::Primary,
                        fetched_at: now,
                        independence_group: origin.to_owned(),
                    });
                }
                let (so_far, _) = assemble(&reading, &with_partial, &with_source, opened);
                on_progress(&so_far)
            };
            stages::extract(llm, question, &source.url, &markdown, today, &mut emit).await
        };

        for text in outcome.claims {
            claims.push((
                question,
                Claim {
                    text: text.text,
                    subject: String::new(),
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
        let (so_far, _) = assemble(&reading, &claims, &sources, &opened);
        if on_progress(&so_far) == Wanted::No {
            stopped_early = true;
            break;
        }
    }

    let (report, coverage) = assemble(&reading, &claims, &sources, &opened);
    Analysis {
        report,
        coverage,
        pages,
        stopped_early,
    }
}

/// Build the report from what is known so far.
///
/// Called after every page as well as at the end, which is the point: a partial report and a
/// finished one are the same shape, and nothing downstream needs to know which it has.
/// What does not change while one company is being read.
///
/// Grouped because [`assemble`] is called four times a run - after every window, after every
/// page, and at the end - and threading seven arguments through each of those is how a caller
/// ends up passing the right value in the wrong position.
struct Reading<'a> {
    found: &'a landscape_discover::Discovered,
    origin: &'a str,
    llm: &'a landscape_llm::LlamaClient,
    now: DateTime<Utc>,
    /// Anything true of the whole report - today, the pages the budget left unread.
    notes: &'a [String],
}

fn assemble(
    reading: &Reading<'_>,
    claims: &[(Answers, Claim)],
    sources: &[Source],
    opened: &[(Answers, usize)],
) -> (Report, Vec<Coverage>) {
    let Reading {
        found,
        origin,
        llm,
        now,
        notes,
    } = *reading;
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
        subjects: Vec::new(),
        sections,
        sources: sources.to_vec(),
        notes: notes.to_vec(),
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

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod joining {
    //! Merging several companies into one report.
    //!
    //! `analyse_many` itself needs a fetcher and a model, so what is asserted here is the join
    //! — which is where a bug would silently mislabel somebody's evidence.

    use super::*;
    use landscape_core::{Confidence, SectionStatus};

    fn at() -> DateTime<Utc> {
        chrono::DateTime::parse_from_rfc3339("2026-08-05T09:00:00Z")
            .map(|d| d.with_timezone(&Utc))
            .unwrap_or_default()
    }

    /// A one-company report: one source labelled `S1`, one claim citing it.
    fn one_company(origin: &str, says: &str) -> Analysis {
        let report = Report {
            subject: origin.to_owned(),
            searched_as: origin.to_owned(),
            generated_at: at(),
            model_id: "test".to_owned(),
            prompt_version: PROMPT_VERSION,
            subjects: Vec::new(),
            sections: vec![Section {
                key: "pricing".to_owned(),
                title: "What it costs".to_owned(),
                status: SectionStatus::Populated,
                claims: vec![Claim {
                    text: says.to_owned(),
                    subject: origin.to_owned(),
                    source_label: "S1".to_owned(),
                    evidence_quote: says.to_owned(),
                    confidence: Confidence::High,
                    as_of: at(),
                }],
                checked: vec![format!("{origin}/pricing")],
                notes: Vec::new(),
            }],
            notes: Vec::new(),
            sources: vec![Source {
                label: "S1".to_owned(),
                url: format!("{origin}/pricing"),
                title: "Pricing".to_owned(),
                disposition: Disposition::Primary,
                fetched_at: at(),
                independence_group: origin.to_owned(),
            }],
        };
        Analysis {
            report,
            coverage: Vec::new(),
            pages: Vec::new(),
            stopped_early: false,
        }
    }

    fn llm() -> landscape_llm::LlamaClient {
        landscape_llm::LlamaClient::new("http://127.0.0.1:8080")
    }

    #[test]
    fn one_section_holds_every_company() {
        // The thing that makes it a comparison rather than two reports stapled together: a
        // reader looking at "What it costs" sees both prices in one place.
        let origins = vec!["https://a.com".to_owned(), "https://b.com".to_owned()];
        let merged = joined(
            &[
                one_company("https://a.com", "Pro costs $15"),
                one_company("https://b.com", "Business costs $16"),
            ],
            None,
            &origins,
            Vec::new(),
            &llm(),
            at(),
        );

        let pricing = merged
            .sections
            .iter()
            .find(|s| s.key == "pricing")
            .expect("one pricing section, not two");
        assert_eq!(merged.sections.len(), 1, "sections merged by question");
        assert_eq!(pricing.claims.len(), 2);
        assert!(pricing.claims.iter().any(|c| c.text == "Pro costs $15"));
        assert!(pricing
            .claims
            .iter()
            .any(|c| c.text == "Business costs $16"));
    }

    #[test]
    fn every_claim_still_points_at_its_own_source() {
        // **The defect this join could most easily introduce.** Each run numbers its sources
        // from `S1`, so merging without renaming gives two companies the same label — and a
        // reader following a citation for A's price arrives at B's pricing page. Evidence
        // attached to the wrong claim is the worst output this product can produce.
        let origins = vec!["https://a.com".to_owned(), "https://b.com".to_owned()];
        let merged = joined(
            &[
                one_company("https://a.com", "Pro costs $15"),
                one_company("https://b.com", "Business costs $16"),
            ],
            None,
            &origins,
            Vec::new(),
            &llm(),
            at(),
        );

        assert!(
            merged.dangling_source_labels().is_empty(),
            "a citation that does not resolve: {:?}",
            merged.dangling_source_labels()
        );
        let labels: Vec<&str> = merged.sources.iter().map(|s| s.label.as_str()).collect();
        assert_eq!(
            labels,
            vec!["S1", "S2"],
            "labels are unique across companies"
        );

        // **Real claim text, deliberately.** The first version of this test invented
        // `A costs $10` and `B costs $20`, which name their company — and review pointed out
        // that this is exactly what hid the defect, because the real extractors produce
        // `Pro costs $15`: what the page says, and nothing about who said it.
        for claim in &merged.sections[0].claims {
            assert!(
                !claim.text.contains("a.com") && !claim.text.contains("b.com"),
                "the fixture names the company in the claim text, which no extractor does"
            );
            let source = merged
                .sources
                .iter()
                .find(|s| s.label == claim.source_label)
                .expect("every claim resolves");
            assert!(
                source.url.starts_with(&claim.subject),
                "{} says it is about {} and cites {}, which is another company's page",
                claim.text,
                claim.subject,
                source.url
            );
        }
    }

    #[test]
    fn a_company_still_being_read_is_part_of_the_report() {
        // What a reader watching sees: the first company's answers, plus whatever the second
        // has produced so far. Without the in-flight half, the page would show nothing new
        // until a whole company finished.
        let origins = vec!["https://a.com".to_owned(), "https://b.com".to_owned()];
        let partial = one_company("https://b.com", "Business costs $16").report;
        let merged = joined(
            &[one_company("https://a.com", "Pro costs $15")],
            Some(&partial),
            &origins,
            Vec::new(),
            &llm(),
            at(),
        );

        assert_eq!(merged.sections[0].claims.len(), 2);
        assert!(merged.dangling_source_labels().is_empty());
    }

    #[test]
    fn every_claim_says_which_company_it_is_about() {
        // Without this a merged pricing section is two numbers and no names: the claim text is
        // whatever the page said, and the web UI renders the text and a bare `[S1]`.
        let origins = vec!["https://a.com".to_owned(), "https://b.com".to_owned()];
        let merged = joined(
            &[
                one_company("https://a.com", "Pro costs $15"),
                one_company("https://b.com", "Business costs $16"),
            ],
            None,
            &origins,
            Vec::new(),
            &llm(),
            at(),
        );

        let subjects: Vec<&str> = merged.sections[0]
            .claims
            .iter()
            .map(|c| c.subject.as_str())
            .collect();
        assert_eq!(subjects, vec!["https://a.com", "https://b.com"]);
    }

    #[test]
    fn coverage_is_one_record_per_question_however_many_companies() {
        // `Analysis::render` zips sections with coverage, and the merged report has one section
        // per question however many subjects it covers. Concatenating each company's six would
        // leave the renderer reading the first company's and silently dropping the rest — so a
        // section that found nothing would describe only the first site's attempts, which is
        // the honest-negative treatment failing exactly where it matters most.
        let a = Analysis {
            coverage: vec![Coverage {
                question: "pricing".to_owned(),
                sources: vec!["https://a.com/pricing".to_owned()],
                pages_read: 1,
                facts: 0,
                attempts: vec![landscape_core::Attempt {
                    subject: "https://a.com".to_owned(),
                    path: "/pricing".to_owned(),
                    outcome: "404".to_owned(),
                }],
            }],
            ..one_company("https://a.com", "Pro costs $15")
        };
        let b = Analysis {
            coverage: vec![Coverage {
                question: "pricing".to_owned(),
                sources: vec!["https://b.com/plans".to_owned()],
                pages_read: 1,
                facts: 0,
                attempts: vec![landscape_core::Attempt {
                    subject: "https://b.com".to_owned(),
                    path: "/plans".to_owned(),
                    outcome: "404".to_owned(),
                }],
            }],
            ..one_company("https://b.com", "Business costs $16")
        };

        let merged = coverage_by_question(&[a, b]);
        assert_eq!(merged.len(), 1, "one record per question, not per company");
        assert_eq!(merged[0].pages_read, 2);
        // And both companies' negative evidence survives, because a path names its company.
        // **Paths, as discovery really stores them** — review caught the first version of this
        // hand-building full URLs, which is the property under test supplied by the fixture.
        let attributed: Vec<String> = merged[0]
            .attempts
            .iter()
            .map(|a| format!("{}{}", a.subject, a.path))
            .collect();
        assert_eq!(
            attributed,
            vec!["https://a.com/pricing", "https://b.com/plans"],
            "one company's attempts were dropped or lost their company"
        );
    }

    /// Origins in `.invalid`, which by RFC 2606 can never resolve.
    ///
    /// Enough to drive `analyse_many` end to end without a model or a reachable page: every
    /// fetch fails fast, every subject produces an empty report, and what is being asserted is
    /// the *joining* — which is the part that has no other way of being reached.
    fn unreachable(n: usize) -> Vec<String> {
        (0..n).map(|i| format!("https://s{i}.invalid")).collect()
    }

    #[tokio::test]
    async fn analyse_many_says_which_companies_it_left_out() {
        // Review's point: capping at three and dropping the fourth in silence is the same
        // defect as taking the first and dropping the second, one count higher. Asserted on
        // the real function rather than on a note handed to the joiner by a test.
        let origins = unreachable(subject::MAX_SUBJECTS + 1);
        let outcome = analyse_many(
            &landscape_fetch::Fetcher::new(),
            &llm(),
            &origins,
            at(),
            at().date_naive(),
            &mut |_| Wanted::Yes,
        )
        .await;

        let last = origins.last().expect("a dropped origin");
        assert!(
            outcome.report.notes.iter().any(|n| n.contains(last)),
            "{last} was dropped without the report saying so: {:?}",
            outcome.report.notes
        );
    }

    #[tokio::test]
    async fn analyse_many_reports_one_coverage_record_per_question() {
        // `Analysis::render` zips sections with coverage. Two companies' worth concatenated
        // would leave the second company's negative evidence unreachable — and this asserts it
        // on the function the worker actually calls, not on the merge helper alone.
        let origins = unreachable(2);
        let outcome = analyse_many(
            &landscape_fetch::Fetcher::new(),
            &llm(),
            &origins,
            at(),
            at().date_naive(),
            &mut |_| Wanted::Yes,
        )
        .await;

        assert_eq!(
            outcome.coverage.len(),
            outcome.report.sections.len(),
            "the renderer zips these two, so a mismatch drops somebody's evidence"
        );
        let mut questions: Vec<&str> = outcome
            .coverage
            .iter()
            .map(|c| c.question.as_str())
            .collect();
        questions.sort_unstable();
        let before = questions.len();
        questions.dedup();
        assert_eq!(
            before,
            questions.len(),
            "a question appears twice: {questions:?}"
        );

        // **Real attempts, from real discovery, for two origins.** Everything above could be
        // satisfied by hand-built coverage; this cannot. `attempts_for` stores a *path*, so the
        // only thing separating two companies' `/pricing` is the subject beside it.
        let subjects: std::collections::BTreeSet<&str> = outcome
            .coverage
            .iter()
            .flat_map(|c| c.attempts.iter())
            .map(|a| a.subject.as_str())
            .filter(|s| !s.is_empty())
            .collect();
        assert_eq!(
            subjects.len(),
            origins.len(),
            "discovery's attempts do not say which company they belong to: {subjects:?}"
        );

        // **The lines the web report renders, not the coverage behind them.** Review found
        // this after the coverage above was already right: `joined()` concatenates strings each
        // company rendered while it was still the only one, so `report.sections[*].checked` -
        // which the interface reads and a `Coverage` never reaches - still said `/pricing (404)`
        // twice. Asserting the model and not the surface is how a fixed defect stays visible.
        let checked: Vec<&str> = outcome
            .report
            .sections
            .iter()
            .flat_map(|s| s.checked.iter())
            .map(String::as_str)
            .collect();
        assert!(
            !checked.is_empty(),
            "two unreachable origins produced no checked lines, so this asserts nothing"
        );
        for origin in &origins {
            let host = origin.trim_start_matches("https://");
            assert!(
                checked.iter().any(|line| line.starts_with(host)),
                "no checked line in the report belongs to {host}: {checked:?}"
            );
        }
        let unattributed: Vec<&&str> = checked.iter().filter(|l| l.starts_with('/')).collect();
        assert!(
            unattributed.is_empty(),
            "checked lines a reader cannot attribute: {unattributed:?}"
        );
    }

    #[test]
    fn each_company_keeps_its_own_note_about_what_it_did_not_read() {
        // The budget is decided per company: one may have had a second pricing page and the
        // other not. A merge that kept only the caller's notes would drop the sentence saying
        // what the shorter wait cost — which is the half of the trade a reader is owed, and it
        // would go missing exactly on the comparisons that are slowest.
        let origins = vec!["https://a.com".to_owned(), "https://b.com".to_owned()];
        let mut first = one_company("https://a.com", "Pro costs $15");
        first.report.notes = vec!["a.com: 2 further page(s) were found and not read".to_owned()];
        let mut second = one_company("https://b.com", "Business costs $16");
        second.report.notes = vec!["b.com: 1 further page(s) were found and not read".to_owned()];

        let merged = joined(
            &[first, second],
            None,
            &origins,
            vec!["Comparing the first 2 sites named.".to_owned()],
            &llm(),
            at(),
        );

        assert!(
            merged.notes.iter().any(|n| n.contains("a.com: 2 further")),
            "the first company's note was dropped: {:?}",
            merged.notes
        );
        assert!(
            merged.notes.iter().any(|n| n.contains("b.com: 1 further")),
            "the second company's note was dropped: {:?}",
            merged.notes
        );
        assert!(
            merged
                .notes
                .iter()
                .any(|n| n.contains("Comparing the first")),
            "the caller's own note was lost: {:?}",
            merged.notes
        );
    }

    #[test]
    fn one_note_written_by_both_companies_is_said_once() {
        // Two companies with the same shape of skipped page produce the same sentence. Saying
        // it twice above the sections reads as two different findings.
        let origins = vec!["https://a.com".to_owned(), "https://b.com".to_owned()];
        let note = "1 further page(s) were found and not read".to_owned();
        let mut first = one_company("https://a.com", "Pro costs $15");
        first.report.notes = vec![note.clone()];
        let mut second = one_company("https://b.com", "Business costs $16");
        second.report.notes = vec![note.clone()];

        let merged = joined(&[first, second], None, &origins, Vec::new(), &llm(), at());
        assert_eq!(
            merged.notes.iter().filter(|n| **n == note).count(),
            1,
            "{:?}",
            merged.notes
        );
    }

    #[test]
    fn a_silent_company_does_not_take_the_label_off_the_one_that_spoke() {
        // Review found this. Deriving "is this a comparison" from the claims looks reasonable
        // and is wrong exactly here: ask about two companies, have one say nothing, and the
        // survivor's prices lose their label in a report that is still a comparison.
        let origins = vec!["https://a.com".to_owned(), "https://b.com".to_owned()];
        let silent = Analysis {
            report: Report {
                sections: vec![Section::not_found("pricing", "What it costs", Vec::new())],
                sources: Vec::new(),
                ..one_company("https://b.com", "unused").report
            },
            ..one_company("https://b.com", "unused")
        };
        let merged = joined(
            &[one_company("https://a.com", "Pro costs $15"), silent],
            None,
            &origins,
            Vec::new(),
            &llm(),
            at(),
        );

        assert_eq!(
            merged.subjects, origins,
            "the report has to carry what it set out to cover, not what answered"
        );
        let rendered = Analysis {
            report: merged,
            coverage: vec![Coverage {
                question: "pricing".to_owned(),
                sources: Vec::new(),
                pages_read: 0,
                facts: 1,
                attempts: Vec::new(),
            }],
            pages: Vec::new(),
            stopped_early: false,
        }
        .render();
        assert!(
            rendered.contains("**a.com**"),
            "the one company that answered lost its label because the other said nothing:
{rendered}"
        );
    }

    #[test]
    fn merged_attempts_keep_their_company_in_the_shape_discovery_stores_them() {
        // Review's second point, and the sharper one: I claimed every attempt was a URL, and
        // `attempts_for` stores a *path* — `/pricing` — because the heading above it used to
        // name the company. Two companies then contribute indistinguishable lines, and my
        // first test hand-built full URLs, which is the fixture supplying the property under
        // test all over again.
        let attempts = |origin: &str| {
            vec![landscape_core::Attempt {
                subject: origin.to_owned(),
                path: "/pricing".to_owned(),
                outcome: "404".to_owned(),
            }]
        };
        let coverage = |origin: &str| Coverage {
            question: "pricing".to_owned(),
            sources: Vec::new(),
            pages_read: 0,
            facts: 0,
            attempts: attempts(origin),
        };
        let merged = coverage_by_question(&[
            Analysis {
                coverage: vec![coverage("https://a.com")],
                ..one_company("https://a.com", "unused")
            },
            Analysis {
                coverage: vec![coverage("https://b.com")],
                ..one_company("https://b.com", "unused")
            },
        ]);

        // Both are `/pricing`. Without the company they are one line written twice.
        let paths: Vec<&str> = merged[0].attempts.iter().map(|a| a.path.as_str()).collect();
        assert_eq!(paths, vec!["/pricing", "/pricing"], "the real shape");

        let section = merged[0].to_section("What it costs");
        assert!(
            section.checked.iter().any(|c| c.contains("a.com"))
                && section.checked.iter().any(|c| c.contains("b.com")),
            "a reader cannot tell which company found nothing: {:?}",
            section.checked
        );
    }

    #[test]
    fn the_report_says_which_companies_it_is_about() {
        let origins = vec!["https://a.com".to_owned(), "https://b.com".to_owned()];
        let merged = joined(&[], None, &origins, Vec::new(), &llm(), at());
        assert_eq!(merged.subject, "https://a.com, https://b.com");
        assert_eq!(merged.searched_as, "https://a.com, https://b.com");
    }
}
