//! Running one page's extractor, and the prompts that go with it.
//!
//! Moved here from the `read` command whole. The prompts are worth reading beside each other:
//! four of the six ask a model for a small thing about a small window, and **two ask nothing of
//! a model at all**, because [`ARCHITECTURE.md`] §5.4 puts what a parser can read on the
//! deterministic side — and dates are what models most often invent.
//!
//! Every answer is checked before it becomes a claim. The checks differ because the facts do:
//!
//! | | checked by |
//! |---|---|
//! | a price | the quote is verbatim |
//! | a capability name | every word of it is in the section — it is a paraphrase by design |
//! | a date | nothing. It was parsed, not generated |
//! | a founding year | the digits are in the section, as a whole number |
//! | a compliance standard | the scanner found the name; the model only says whether it is held |
//! | a job title | nothing. It is the line |
//!
//! [`ARCHITECTURE.md`]: ../../../docs/ARCHITECTURE.md

use chrono::NaiveDate;
use landscape_core::{
    FeatureExtraction, IdentityExtraction, PageChanges, PageFeatures, PageIdentity, PagePricing,
    PricingExtraction,
};
use landscape_discover::probes::Answers;
use landscape_extract::span::Span;

use crate::Outcome;

/// Decoding settings shared by every extractor.
///
/// Temperature zero and a fixed seed because a report that changes between two runs of the
/// same page cannot be debugged, and `BENCHMARKS.md` compares runs.
fn decode() -> landscape_llm::Decode {
    landscape_llm::Decode {
        max_tokens: 300,
        temperature: 0.0,
        seed: Some(7),
    }
}

/// Reports the facts a page has yielded so far, each time one more arrives.
///
/// **Per window, not per page.** Watching a real run through the browser showed why: the
/// first page of `plausible.io` is twelve capability windows, and a reader saw nothing for
/// four minutes while they were read one at a time. `PRODUCT_SPEC.md` §2.1A asks for content
/// in twenty to forty seconds, and a page is far too large a unit to deliver that.
///
/// **The return value is the caller saying whether to carry on.** A model call is the most
/// expensive thing this program does, so the cheapest place to abandon a run nobody is waiting
/// for is between two of them.
pub(crate) type Progress<'a> = &'a mut dyn FnMut(&[crate::Finding]) -> crate::Wanted;

/// Read one page for one question.
pub(crate) async fn extract(
    llm: &landscape_llm::LlamaClient,
    question: Answers,
    url: &str,
    markdown: &str,
    today: NaiveDate,
    so_far: Progress<'_>,
) -> Outcome {
    match question {
        Answers::Pricing => pricing(llm, url, markdown, so_far).await,
        Answers::Features => features(llm, url, markdown, so_far).await,
        Answers::Changes => changes(markdown, today),
        Answers::Identity => identity(llm, url, markdown, so_far).await,
        Answers::Trust => trust(llm, url, markdown, so_far).await,
        Answers::Direction => direction(markdown),
    }
}

/// Open roles, read off the page. **No model runs here either.**
///
/// The second deterministic question, and the last extractor of the six. A job title is a line
/// somebody wrote down on purpose; reading it is transcription, and the rule
/// `ARCHITECTURE.md` §5.4 applies to a changelog applies here for the same reason.
fn direction(markdown: &str) -> Outcome {
    let found = landscape_extract::hiring::every_role(markdown);
    let page = landscape_core::PageHiring {
        roles: found
            .roles
            .iter()
            .map(|role| landscape_core::Role {
                title: role.title.clone(),
                evidence_quote: Some(role.quote.clone()),
            })
            .collect(),
        considered: found.considered,
        announced: found.announced,
    };

    if page.is_empty() {
        // **Two silences, and they are different facts.** A company in a hiring freeze
        // publishes a careers page with nothing on it, which is news about the company. A page
        // that never says which part of itself is the list is our gap, and saying "no open
        // roles" about it would be this pipeline stating something it does not know.
        return Outcome {
            claims: Vec::new(),
            summary: if page.announced {
                "no open roles listed on the page".to_owned()
            } else {
                "the page does not say where its open roles are listed, so none were read"
                    .to_owned()
            },
            details: Vec::new(),
            window_words: 0,
            // Nothing to ask, so nothing could fail: this is the whole answer.
            settled: crate::Settled::Complete,
        };
    }

    let mut details: Vec<String> = page
        .roles
        .iter()
        .take(6)
        .map(|role| role.title.clone())
        .collect();
    if page.roles.len() > 6 {
        details.push(format!("... and {} more", page.roles.len() - 6));
    }
    if page.passed_over() > 0 {
        details.push(format!(
            "read {} of {} roles the page lists",
            page.roles.len(),
            page.considered
        ));
    }
    Outcome {
        claims: crate::claims_from_hiring(&page),
        summary: format!(
            "{} open role{} listed",
            page.roles.len(),
            plural(page.roles.len())
        ),
        details,
        window_words: page.roles.len(),
        // No model ran, so there is nothing here that can half-happen.
        settled: crate::Settled::Complete,
    }
}

/// Every plan the page publishes, one window each.
async fn pricing(
    llm: &landscape_llm::LlamaClient,
    url: &str,
    markdown: &str,
    so_far: Progress<'_>,
) -> Outcome {
    let spans = landscape_extract::span::every_plan(markdown);
    if spans.is_empty() {
        return Outcome {
            claims: Vec::new(),
            summary: "no pricing content on the page".to_owned(),
            details: Vec::new(),
            window_words: 0,
            // Nothing to ask, so nothing could fail: this is the whole answer.
            settled: crate::Settled::Complete,
        };
    }

    // **Whether this answer is worth remembering.** A window whose model call failed, or a run
    // nobody is waiting for any more, produces a real report and an incomplete one — and
    // `memo::Extractions` must not keep it, or a transient outage is replayed for the life of
    // the process. See `crate::Settled`.
    let mut whole = true;
    let mut extracted: Vec<PricingExtraction> = Vec::with_capacity(spans.len());
    let mut details = Vec::new();
    let mut unsupported = 0usize;
    for span in &spans {
        match llm
            .generate::<PricingExtraction>(&pricing_prompt(url, span), &decode())
            .await
        {
            Ok(got) => {
                if !got.quote_is_verbatim(&span.prompt_text()) {
                    unsupported += 1;
                }
                extracted.push(got);
            }
            Err(e) => {
                details.push(format!("model error: {e}"));
                whole = false;
            }
        }
        // **After every window, whatever became of it.** One plan is enough to put the section
        // on somebody's screen — and asking here rather than inside the `Ok` arm is what makes
        // a run whose answers are failing still able to hear that nobody wants it.
        if so_far(&crate::claims_from_pricing(&PagePricing::assembled(
            extracted.clone(),
        ))) == crate::Wanted::No
        {
            whole = false;
            break;
        }
    }
    let page = PagePricing::assembled(extracted);
    let claims = crate::claims_from_pricing(&page);

    let mut lines: Vec<String> = page.plans.iter().map(describe_plan).collect();
    if unsupported > 0 {
        lines.push(format!(
            "{unsupported} quote(s) not found in the section they came from"
        ));
    }
    lines.extend(details);
    Outcome {
        claims,
        summary: format!(
            "{} plan{} found in {} window{}",
            page.plans.len(),
            plural(page.plans.len()),
            spans.len(),
            plural(spans.len())
        ),
        details: lines,
        window_words: window_words(&spans),
        settled: if whole {
            crate::Settled::Complete
        } else {
            crate::Settled::Partial
        },
    }
}

/// Every capability the page names, one window each.
async fn features(
    llm: &landscape_llm::LlamaClient,
    url: &str,
    markdown: &str,
    so_far: Progress<'_>,
) -> Outcome {
    let found = landscape_extract::capability::every_capability(markdown);
    if found.windows.is_empty() {
        return Outcome {
            claims: Vec::new(),
            summary: "no named capabilities on the page".to_owned(),
            details: Vec::new(),
            window_words: 0,
            // Nothing to ask, so nothing could fail: this is the whole answer.
            settled: crate::Settled::Complete,
        };
    }

    // **Whether this answer is worth remembering.** A window whose model call failed, or a run
    // nobody is waiting for any more, produces a real report and an incomplete one — and
    // `memo::Extractions` must not keep it, or a transient outage is replayed for the life of
    // the process. See `crate::Settled`.
    let mut whole = true;
    let mut extracted: Vec<FeatureExtraction> = Vec::with_capacity(found.windows.len());
    let mut details = Vec::new();
    let (mut unsupported, mut ungrounded) = (0usize, 0usize);
    for window in &found.windows {
        match llm
            .generate::<FeatureExtraction>(&capability_prompt(url, window), &decode())
            .await
        {
            Ok(got) => {
                let section = window.prompt_text();
                if got.name_is_from(&section) {
                    if !got.quote_is_verbatim(&section) {
                        unsupported += 1;
                    }
                    extracted.push(got);
                } else {
                    // Dropped, and it used to `continue` straight past the question below.
                    ungrounded += 1;
                }
            }
            Err(e) => {
                details.push(format!("model error: {e}"));
                whole = false;
            }
        }
        if so_far(&crate::claims_from_features(&PageFeatures::assembled(
            extracted.clone(),
            found.considered,
        ))) == crate::Wanted::No
        {
            whole = false;
            break;
        }
    }
    let page = PageFeatures::assembled(extracted, found.considered);
    let claims = crate::claims_from_features(&page);

    let mut lines: Vec<String> = page
        .features
        .iter()
        .map(|f| match f.qualifier.as_deref() {
            Some(q) if !q.is_empty() => {
                format!("{} ({q})", f.capability.as_deref().unwrap_or("unnamed"))
            }
            _ => f.capability.clone().unwrap_or_else(|| "unnamed".to_owned()),
        })
        .collect();
    if ungrounded > 0 {
        lines.push(format!(
            "{ungrounded} name(s) dropped - not words from the section"
        ));
    }
    if unsupported > 0 {
        lines.push(format!(
            "{unsupported} quote(s) not found in the section they came from"
        ));
    }
    lines.extend(details);

    // The cap is stated rather than applied quietly: twelve read out of eighteen is a short
    // list, and twelve with no number beside it is a wrong one.
    let capped = if found.considered > found.windows.len() {
        format!(" (of {} the page names)", found.considered)
    } else {
        String::new()
    };
    Outcome {
        claims,
        summary: format!(
            "{} capabilit{} stated{capped}",
            page.features.len(),
            if page.features.len() == 1 { "y" } else { "ies" }
        ),
        details: lines,
        window_words: found
            .windows
            .iter()
            .map(|w| w.text.split_whitespace().count())
            .sum(),
        settled: if whole {
            crate::Settled::Complete
        } else {
            crate::Settled::Partial
        },
    }
}

/// What a trust page says it holds, one named standard at a time.
///
/// **The scanner finds the name; the model reads the claim.** A compliance standard is a closed
/// vocabulary, so nothing can be reported that `assurance::every_assurance` did not first find
/// written on the page — which removes the failure the feature extractor has to defend against
/// with grounding alone, where a model asked to *"list the certifications"* invents one.
///
/// What is left for the model is the part that is genuinely reading: whether the page says they
/// **have** it or are **working toward** it. Both spellings contain the same words, and a
/// report that treated them alike would be this project's characteristic wrong answer —
/// correct-looking, fully cited, about a different fact.
async fn trust(
    llm: &landscape_llm::LlamaClient,
    url: &str,
    markdown: &str,
    so_far: Progress<'_>,
) -> Outcome {
    let found = landscape_extract::assurance::every_assurance(markdown);
    if found.named.is_empty() {
        return Outcome {
            claims: Vec::new(),
            // Said in the words of what was looked for, because "nothing found" on a security
            // page usually means the page is reassurance rather than a list of standards - and
            // that is a finding about the company rather than about us.
            summary: "no named standard on the page".to_owned(),
            details: Vec::new(),
            window_words: 0,
            // Nothing to ask, so nothing could fail: this is the whole answer.
            settled: crate::Settled::Complete,
        };
    }

    // **Whether this answer is worth remembering.** A window whose model call failed, or a run
    // nobody is waiting for any more, produces a real report and an incomplete one — and
    // `memo::Extractions` must not keep it, or a transient outage is replayed for the life of
    // the process. See `crate::Settled`.
    let mut whole = true;
    let mut extracted: Vec<landscape_core::TrustExtraction> = Vec::with_capacity(found.named.len());
    let mut details = Vec::new();
    let (mut unsupported, mut mismatched) = (0usize, 0usize);

    for named in &found.named {
        match llm
            .generate::<landscape_core::AssuranceClaim>(
                &assurance_prompt(url, &named.standard, &named.span),
                &decode(),
            )
            .await
        {
            // **The standard is the scanner's, not the model's.** It is attached here rather
            // than asked for, so a window naming two standards cannot produce an answer about
            // the other one, and a shortened spelling cannot undo the precise-spelling rule.
            // Review found both, and both stop being expressible rather than being checked.
            Ok(claim) => {
                match judge_assurance(&claim, &named.span.prompt_text(), &named.standard) {
                    Judged::Keep => extracted.push(claim.about(&named.standard)),
                    Judged::QuoteNotInTheSection => {
                        unsupported += 1;
                        extracted
                            .push(landscape_core::AssuranceClaim::empty().about(&named.standard));
                    }
                    // The page does name this standard - the scanner found it - so the mention is
                    // kept and the claim is not. Answering about a neighbor is the failure mode
                    // of handing one window over twice.
                    Judged::EvidenceIsAboutAnother => {
                        mismatched += 1;
                        extracted
                            .push(landscape_core::AssuranceClaim::empty().about(&named.standard));
                    }
                }
            }
            Err(e) => {
                details.push(format!("model error: {e}"));
                whole = false;
            }
        }
        if so_far(&crate::claims_from_trust(
            &landscape_core::PageTrust::assembled(extracted.clone(), found.considered),
        )) == crate::Wanted::No
        {
            whole = false;
            break;
        }
    }

    let page = landscape_core::PageTrust::assembled(extracted, found.considered);
    let claims = crate::claims_from_trust(&page);

    let mut lines: Vec<String> = page
        .assurances
        .iter()
        .map(|a| {
            let name = a.standard.as_deref().unwrap_or("unnamed");
            match a.status {
                Some(status) => format!("{name} ({})", status.wording()),
                None => format!("{name} (named, nothing claimed)"),
            }
        })
        .collect();
    if mismatched > 0 {
        lines.push(format!(
            "{mismatched} answer(s) quoted a different standard - the mention is kept, the claim is not"
        ));
    }
    if unsupported > 0 {
        lines.push(format!(
            "{unsupported} claim(s) dropped - the quote was not in the section. The mention is kept"
        ));
    }
    lines.extend(details);

    // The cap is stated rather than applied quietly, the same as the capability cap.
    let capped = if found.considered > found.named.len() {
        format!(" (of {} the page names)", found.considered)
    } else {
        String::new()
    };
    Outcome {
        claims,
        summary: format!("{} standard(s) read{capped}", page.assurances.len()),
        details: lines,
        window_words: found
            .named
            .iter()
            .map(|n| n.span.text.split_whitespace().count())
            .sum(),
        settled: if whole {
            crate::Settled::Complete
        } else {
            crate::Settled::Partial
        },
    }
}

/// What to make of one claim, decided without a model in the room.
///
/// **A pure function so the decision can be tested.** The rest of the stage cannot run without
/// a `llama-server`, so a judgment made inline is a judgment nothing holds still.
///
/// # The model cannot choose the name, and that was not enough
///
/// Taking the standard from the scanner stopped the model *labeling* an answer. It did not
/// stop it *answering about the wrong thing*: a window is three lines, and two standards often
/// sit within three lines of each other — or on one line — so the same window is handed over
/// twice, once per standard. A model asked about ISO 27001 can reply `holds` while quoting the
/// sentence about SOC 2, and every check passed: the quote is verbatim in the section, and the
/// name is the scanner's.
///
/// That publishes *"states ISO 27001"* on evidence about a different certification. Review
/// found it **in the regression I had just written to prove the class was closed**.
///
/// So the evidence is checked against the standard it is supposed to be about. A quote naming
/// **no** standard is the ordinary honest case — *"We are certified and audited annually."*
/// beside the name — and is kept.
/// # Both failures take the same conservative path
///
/// `ARCHITECTURE.md` is explicit: *"a claim whose evidence quote is absent from its cited source
/// is deleted"*. The first version of this counted a non-verbatim quote and **published the
/// status anyway**, on the argument that the standard was real so the finding was worth keeping.
/// Review pointed out what that ships: *"states ISO 27001"* against a page whose only sentence
/// is *"Questions about ISO 27001? Contact us."* — an unsupported compliance claim, with a line
/// in a run log nobody reads standing in for a check.
///
/// So an unsupported claim is dropped and the **mention is kept**, exactly as it is when the
/// evidence turns out to be about a neighboring standard. That the page names the standard is
/// the scanner's finding and stays true; that they hold it is the model's, and it goes when its
/// evidence does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Judged {
    /// Quoted from the section, and about the standard in question.
    Keep,
    /// The quote is missing, empty, or not in the section. **The claim is dropped and the
    /// mention is kept.**
    QuoteNotInTheSection,
    /// The quote names a different standard and not this one. **The claim is dropped and the
    /// mention is kept**: that the page names this standard is true and worth reporting; that
    /// they hold it is a fact this evidence does not support.
    EvidenceIsAboutAnother,
}

pub(crate) fn judge_assurance(
    claim: &landscape_core::AssuranceClaim,
    section: &str,
    requested: &str,
) -> Judged {
    let quote = claim.evidence_quote.as_deref().unwrap_or_default().trim();

    if !quote.is_empty() {
        let named = landscape_extract::assurance::standards_named(quote);
        if !named.is_empty()
            && !named
                .iter()
                .any(|n| landscape_extract::assurance::same_standard(n, requested))
        {
            return Judged::EvidenceIsAboutAnother;
        }
    }

    // **A status with nothing behind it is not a claim.** An answer of `holds` and no quote at
    // all used to pass, because "there is no quote to check" reads as "the quote is fine" —
    // which is the same shape as every check-that-cannot-fail in the register.
    if quote.is_empty() {
        return if claim.status.is_some() {
            Judged::QuoteNotInTheSection
        } else {
            Judged::Keep
        };
    }

    if claim.quote_is_verbatim(section) {
        Judged::Keep
    } else {
        Judged::QuoteNotInTheSection
    }
}

/// One standard at a time, and the only question is whether they say they have it.
fn assurance_prompt(url: &str, standard: &str, window: &Span) -> String {
    let section: String = window.prompt_text().chars().take(6000).collect();
    format!(
        "You are reading one section of a company's security or trust page. The section \
         mentions {standard}. Say what the section claims about it.

         Page: {url}

         Rules:
         - status is holds when the section says they have it, are certified, are compliant, \
           or that a report or certificate exists.
         - status is pursuing when the section says they are working toward it, it is in \
           progress, planned, expected, or on a roadmap.
         - Leave status null when the section only mentions the standard without saying \
           either - a question, a link, or a contact address is not a claim.
         - Use only the words of this section. Do not use anything you know about this \
           company from elsewhere.
         - evidence_quote must be copied from the section word for word.

         SECTION
---
{section}
---

Return the extraction as JSON."
    )
}

/// Dated entries, parsed. **No model runs here.**
fn changes(markdown: &str, today: NaiveDate) -> Outcome {
    let found = landscape_extract::changes::every_change(markdown);
    let page = PageChanges {
        changes: found
            .entries
            .iter()
            .map(|e| landscape_core::Change {
                happened_on: NaiveDate::from_ymd_opt(e.year, e.month, e.day),
                summary: (!e.title.is_empty()).then(|| e.title.clone()),
                evidence_quote: Some(e.quote.clone()),
            })
            .collect(),
        considered: found.considered,
    };
    if page.is_empty() {
        return Outcome {
            claims: Vec::new(),
            summary: "no dated entries on the page".to_owned(),
            details: Vec::new(),
            window_words: 0,
            // Nothing to ask, so nothing could fail: this is the whole answer.
            settled: crate::Settled::Complete,
        };
    }

    let recent = page.recent(today);
    let older = page.older_than_lookback(today);
    let ahead = page.dated_ahead(today);
    let mut details: Vec<String> = recent
        .iter()
        .take(6)
        .map(|c| {
            let when = c
                .happened_on
                .map_or_else(|| "?".to_owned(), |d| d.to_string());
            format!("{when}  {}", c.summary.as_deref().unwrap_or("(untitled)"))
        })
        .collect();
    if recent.len() > 6 {
        details.push(format!(
            "... and {} more inside the window",
            recent.len() - 6
        ));
    }
    if recent.is_empty() {
        details.push(format!(
            "nothing inside the window - the newest entry is older than {} days",
            PageChanges::LOOKBACK_DAYS
        ));
    }
    if ahead > 0 {
        details.push(format!(
            "{ahead} entr(y/ies) dated ahead of today - announced, not shipped"
        ));
    }
    if found.considered > found.entries.len() {
        details.push(format!(
            "read {} of {} dated entries on the page",
            found.entries.len(),
            found.considered
        ));
    }

    Outcome {
        claims: crate::claims_from_changes(&page, today),
        summary: format!(
            "{} change(s) in {} days, {older} older",
            recent.len(),
            PageChanges::LOOKBACK_DAYS
        ),
        details,
        // Parsed rather than generated — see the module docs. Nothing can half-happen.
        settled: crate::Settled::Complete,
        window_words: found.entries.len(),
    }
}

/// Who they are, where, how big — one window per fact.
async fn identity(
    llm: &landscape_llm::LlamaClient,
    url: &str,
    markdown: &str,
    so_far: Progress<'_>,
) -> Outcome {
    let windows = landscape_extract::identity::every_fact(markdown);
    if windows.is_empty() {
        return Outcome {
            claims: Vec::new(),
            summary: "no stated facts about the company".to_owned(),
            details: Vec::new(),
            window_words: 0,
            // Nothing to ask, so nothing could fail: this is the whole answer.
            settled: crate::Settled::Complete,
        };
    }

    // **Whether this answer is worth remembering.** A window whose model call failed, or a run
    // nobody is waiting for any more, produces a real report and an incomplete one — and
    // `memo::Extractions` must not keep it, or a transient outage is replayed for the life of
    // the process. See `crate::Settled`.
    let mut whole = true;
    let mut extracted: Vec<IdentityExtraction> = Vec::with_capacity(windows.len());
    let mut details = Vec::new();
    let (mut unsupported, mut ungrounded) = (0usize, 0usize);
    for (fact, window) in &windows {
        match llm
            .generate::<IdentityExtraction>(&identity_prompt(url, *fact, window), &decode())
            .await
        {
            Ok(got) => {
                // Field by field. An extraction carrying a correct year and an invented
                // headcount used to be discarded whole, which threw away the answer the
                // window had been asked for.
                let (kept, dropped) = got.keeping_only_stated(&window.text);
                ungrounded += dropped;
                if !kept.quote_is_verbatim(&window.text) {
                    unsupported += 1;
                }
                extracted.push(kept);
            }
            Err(e) => {
                details.push(format!("model error: {e}"));
                whole = false;
            }
        }
        // After every window, whatever became of it — see the note in `pricing`.
        if so_far(&crate::claims_from_identity(&PageIdentity::assembled(
            extracted.clone(),
        ))) == crate::Wanted::No
        {
            whole = false;
            break;
        }
    }
    let page = PageIdentity::assembled(extracted);
    let claims = crate::claims_from_identity(&page);

    let mut lines: Vec<String> = claims.iter().map(|c| c.text.clone()).collect();
    if ungrounded > 0 {
        lines.push(format!(
            "{ungrounded} answer(s) dropped - not written in the window"
        ));
    }
    if unsupported > 0 {
        lines.push(format!(
            "{unsupported} quote(s) not found in the section they came from"
        ));
    }
    lines.extend(details);
    Outcome {
        claims,
        summary: format!("{} of 3 facts stated", page.facts()),
        details: lines,
        window_words: windows
            .iter()
            .map(|(_, w)| w.text.split_whitespace().count())
            .sum(),
        settled: if whole {
            crate::Settled::Complete
        } else {
            crate::Settled::Partial
        },
    }
}

fn plural(n: usize) -> &'static str {
    if n == 1 {
        ""
    } else {
        "s"
    }
}

fn window_words(spans: &[Span]) -> usize {
    spans
        .iter()
        .map(|s| s.text.split_whitespace().count())
        .sum()
}

/// One plan, as a line a person can read.
fn describe_plan(plan: &PricingExtraction) -> String {
    let period = plan.billing_period.map_or_else(String::new, |p| {
        match p {
            landscape_core::BillingPeriod::Monthly => "/mo",
            landscape_core::BillingPeriod::Yearly => "/yr",
            landscape_core::BillingPeriod::OneOff => " once",
        }
        .to_owned()
    });
    match (plan.plan_name.as_deref(), plan.price_usd) {
        (Some(name), Some(price)) => format!("{name} at ${price}{period}"),
        (Some(name), None) => format!("{name}, no price published"),
        (None, Some(price)) => format!("${price}{period}, no plan named"),
        (None, None) => "nothing found".to_owned(),
    }
}

/// The pricing prompt.
///
/// **The section names the plan.** An earlier version asked for *"the first plan this page
/// presents"*, which is a question about a page the model can no longer see — it is given one
/// section, and the plan is the one in that section's heading.
fn pricing_prompt(url: &str, span: &Span) -> String {
    let page: String = span.prompt_text().chars().take(6000).collect();
    let plan = span.heading.as_deref().map_or_else(
        || "the plan this section is about".to_owned(),
        |h| {
            format!(
                "the plan named in the heading \"{}\"",
                h.trim_start_matches('#').trim()
            )
        },
    );
    format!(
        "You are reading one section of a page from a company's website and extracting          what it says about the pricing of one plan.

         Plan to extract: {plan}.

         Page: {url}

         Rules:
         - Use only what this section states. Do not use anything you know from elsewhere.
         - If it does not state something, leave that field null. A missing price is a fact          worth reporting; a guessed one is not.
         - Report the price of that one plan only. A price here for a different plan, a          different product, or an add-on is not this plan's price.
         - If price_usd is null then billing_period must be null too.
         - billing_period is how often the price itself recurs, not how often the invoice          arrives. A price of 10 dollars per user/month, billed yearly, recurs monthly.
         - price_usd is in US dollars. Ignore prices given in other currencies.
         - price_usd must be a number written on the page. Do not calculate one. A price          given per 1,000 of something is not a price per one of them.
         - evidence_quote must be copied from the section word for word.

         SECTION
---
{page}
---

Return the extraction as JSON."
    )
}

/// The capability prompt. Normalization, which is all §5.4 asks a model for here.
///
/// **It carried a worked example once, and the model used it as a fact.** With *Message
/// Boards* in the instructions, a page of Linear's documentation came back reporting Message
/// Boards — a Basecamp feature. A worked example in a prompt is a source with no URL.
fn capability_prompt(url: &str, window: &Span) -> String {
    let section: String = window.prompt_text().chars().take(6000).collect();
    format!(
        "You are reading one section of a company's features page. The section describes          one capability of the product. Name it.

         Page: {url}

         Rules:
         - capability is the shortest name the section itself uses for the thing, as a noun          phrase. Not a sentence, and not a benefit. Where a heading names a thing and then          says what it is for, the name is the part before that.
         - Every word of capability must appear in the section. Never write a placeholder,          a field type, or a name taken from these instructions.
         - Use only the words of this section. Do not use anything you know from elsewhere.
         - qualifier is a condition the section attaches to the capability - beta, coming          soon, a plan it requires. Leave it null unless the section states one.
         - Do not record whether the capability is any good, only what it is called.
         - evidence_quote must be copied from the section word for word.

         SECTION
---
{section}
---

Return the extraction as JSON."
    )
}

/// One identity question at a time, because the three answers are in different sentences.
fn identity_prompt(url: &str, fact: landscape_extract::identity::Fact, window: &Span) -> String {
    use landscape_extract::identity::Fact;

    let asked = match fact {
        Fact::Founded => "the year the company was founded",
        Fact::Headquarters => "where the company is based",
        Fact::Employees => "how many people work there",
    };
    let section: String = window.prompt_text().chars().take(6000).collect();
    format!(
        "You are reading a few lines from a company's about page. Extract only what they          state about {asked}.

         Page: {url}

         Rules:
         - Every value must be written in these lines. Do not use anything you know about          this company from elsewhere.
         - Leave a field null unless these lines state it. Most about pages state none of          them, and that is the expected answer.
         - Do not calculate. A company saying it has been running for 23 years has not          stated the year it was founded.
         - founded_year is a four-digit year. employees is a count of people.
         - evidence_quote must be copied from these lines word for word.

         SECTION
---
{section}
---

Return the extraction as JSON."
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    /// A `llama-server` that answers instantly with the same valid extraction, and counts.
    ///
    /// **The saving from stopping a run is model calls, so model calls are what to count.**
    /// `landscape-golden`'s model tests are `#[ignore]`d because CI has no GPU, and the same
    /// would be true here — except the question is not *what the model says*, it is **how many
    /// times it is asked**, and for that a stub is the better instrument as well as the one
    /// that runs on every pull request.
    struct StubModel {
        base: String,
        calls: Arc<AtomicUsize>,
    }

    impl StubModel {
        async fn start(content: &'static str) -> Self {
            let calls = Arc::new(AtomicUsize::new(0));
            let counter = Arc::clone(&calls);
            let app = axum::Router::new().route(
                "/completion",
                axum::routing::post(move || {
                    let counter = Arc::clone(&counter);
                    async move {
                        counter.fetch_add(1, Ordering::Relaxed);
                        axum::Json(serde_json::json!({ "content": content }))
                    }
                }),
            );
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
                .await
                .expect("a port");
            let base = format!("http://{}", listener.local_addr().expect("an address"));
            tokio::spawn(async move {
                let _ = axum::serve(listener, app).await;
            });
            Self { base, calls }
        }

        fn calls(&self) -> usize {
            self.calls.load(Ordering::Relaxed)
        }
    }

    #[tokio::test]
    async fn a_trust_page_reaches_the_trust_extractor() {
        // **The dispatch, not the extractor.** `trust-posture.json` pins what
        // `assurance::every_assurance` finds and what the judge accepts; nothing asserted that
        // `Answers::Trust` still arrives *there*. The mutation that returns an empty outcome
        // for a trust page survived the whole suite, which is a page fetched, read, and thrown
        // away in silence.
        let page = landscape_golden::pages::load()
            .expect("the page set loads")
            .into_iter()
            .find(|e| e.page == "linear-security.md")
            .expect("the security page")
            .markdown()
            .expect("it reads");
        let named = landscape_extract::assurance::every_assurance(&page)
            .named
            .len();
        assert!(named > 0, "the fixture should name a standard");

        let stub = StubModel::start(A_CLAIM).await;
        let llm = landscape_llm::LlamaClient::new(&stub.base);
        let mut carry_on = |_: &[crate::Finding]| crate::Wanted::Yes;
        let outcome = extract(
            &llm,
            Answers::Trust,
            "https://linear.app/security",
            &page,
            NaiveDate::from_ymd_opt(2026, 8, 5).unwrap(),
            &mut carry_on,
        )
        .await;

        assert_eq!(stub.calls(), named, "the trust extractor was not asked");
        assert!(
            !outcome.claims.is_empty(),
            "a trust page was read and produced nothing"
        );
        assert!(outcome.window_words > 0, "no window was read");
    }

    /// The real Basecamp features page: twelve capability windows, one model call each. The
    /// shape that made a reader wait four minutes in `BENCHMARKS.md` Run 16, and therefore the
    /// shape most worth abandoning when nobody is waiting any more.
    fn twelve_windows() -> String {
        landscape_golden::pages::load()
            .expect("the page set loads")
            .into_iter()
            .find(|e| e.page == "basecamp-features.md")
            .expect("the twelve-capability page")
            .markdown()
            .expect("it reads")
    }

    const A_CAPABILITY: &str =
        r#"{"name":"The Project page keeps it all together","evidence_quote":"The Project page"}"#;

    /// What the stub says about one standard: a claim, and no name — because the generated
    /// type has no name field to fill.
    const A_CLAIM: &str =
        r#"{"status":"holds","evidence_quote":"undergoes regular SOC 2 Type II audits"}"#;

    /// A section naming two standards, which is the case the old check could not survive.
    const TWO_STANDARDS: &str = "# Security\n\nLinear undergoes regular SOC 2 Type II audits.\n\nISO 27001 is on our roadmap for next year.\n\nWe encrypt everything in transit and at rest, and publish our posture here.";

    #[tokio::test]
    async fn the_standard_reported_is_the_one_the_scanner_found() {
        // **Review found that nothing tested this wiring.** Replacing `claim.about(&named
        // .standard)` with a hard-coded standard left all 572 tests green — so the class of
        // defect the previous round removed could walk straight back in.
        //
        // I had said the stage was untestable without a model. There is a `StubModel` forty
        // lines below the code I was editing, used by nine tests. The claim was wrong and I
        // did not check it.
        //
        // The page names two standards. The stub answers with a claim and no name at all, so
        // whatever reaches the report can only have come from the scanner.
        let stub = StubModel::start(A_CLAIM).await;
        let llm = landscape_llm::LlamaClient::new(&stub.base);

        let outcome = trust(
            &llm,
            "https://linear.app/security",
            TWO_STANDARDS,
            &mut |_| crate::Wanted::Yes,
        )
        .await;

        let said: Vec<&str> = outcome.claims.iter().map(|c| c.text.as_str()).collect();
        assert!(
            said.iter().any(|t| t.contains("SOC 2 Type II")),
            "the precise standard the scanner found is not in the report: {said:?}"
        );
        assert!(
            said.iter().any(|t| t.contains("ISO 27001")),
            "the second standard on the page is missing: {said:?}"
        );
        // Two standards named, two windows, two calls - one question each.
        assert_eq!(stub.calls(), 2, "one call per standard the scanner found");

        // **Review's assertion, and the one the first version of this test was missing.** The
        // stub answers every window with the same SOC 2 sentence. Checking only that both
        // names appear accepted `states it holds ISO 27001` carrying evidence about SOC 2 -
        // a fabricated certification claim that the test was written to rule out.
        let iso = outcome
            .claims
            .iter()
            .find(|c| c.text.contains("ISO 27001"))
            .expect("the ISO finding");
        assert!(
            !iso.quote.contains("SOC 2"),
            "the ISO claim was paired with evidence about another standard: {}",
            iso.quote
        );
        assert!(
            iso.text.contains("without saying"),
            "an answer about a different standard became a claim about this one: {}",
            iso.text
        );
    }

    #[tokio::test]
    async fn a_claim_whose_quote_is_not_on_the_page_never_reaches_the_report() {
        // **Review's reproduction, and my test used to assert the opposite.** The page asks a
        // question about ISO 27001 and claims nothing; the model answers `holds` with a
        // sentence that is not there. The old arm counted the quote and published the status,
        // so the report said "states ISO 27001" about a company that had said no such thing.
        //
        // `ARCHITECTURE.md`: a claim whose evidence quote is absent from its cited source is
        // deleted. The mention is the scanner's and survives; the claim is the model's and
        // does not.
        const FABRICATED: &str =
            r#"{"status":"holds","evidence_quote":"We are ISO 27001 certified"}"#;
        let stub = StubModel::start(FABRICATED).await;
        let llm = landscape_llm::LlamaClient::new(&stub.base);

        let outcome = trust(
            &llm,
            "https://example.test/security",
            "# Security\n\nQuestions about ISO 27001? Contact us.",
            &mut |_| crate::Wanted::Yes,
        )
        .await;

        assert_eq!(outcome.claims.len(), 1, "the mention was thrown away too");
        let said = &outcome.claims[0];
        assert!(
            said.text.contains("without saying"),
            "an unsupported claim reached the report: {}",
            said.text
        );
        assert!(
            said.quote.is_empty(),
            "a quote that is not on the page was published as evidence: {}",
            said.quote
        );
        assert!(
            outcome.details.iter().any(|d| d.contains("dropped")),
            "the reader is not told a claim was set aside: {:?}",
            outcome.details
        );
    }

    #[tokio::test]
    async fn a_page_that_names_no_standard_never_asks_the_model() {
        // The scanner runs first, so a security page that is reassurance and nothing else
        // costs nothing at all. This is the arithmetic `landscape cost` prints, asserted
        // against the stage that does it rather than against the counter that predicts it.
        let stub = StubModel::start(A_CLAIM).await;
        let llm = landscape_llm::LlamaClient::new(&stub.base);

        let outcome = trust(
            &llm,
            "https://basecamp.com/security",
            "# Security\n\nWe take security seriously and encrypt everything in transit.",
            &mut |_| crate::Wanted::Yes,
        )
        .await;

        assert!(outcome.claims.is_empty());
        assert_eq!(stub.calls(), 0, "a page with nothing named was still read");
        assert!(
            outcome.summary.contains("no named standard"),
            "{}",
            outcome.summary
        );
    }

    #[tokio::test]
    async fn a_trust_run_told_to_stop_asks_no_further_questions() {
        // Every other extractor is checked for this; the newest one has to be too, or the
        // saving that `Wanted::No` exists for quietly stops applying to a fifth of the run.
        let stub = StubModel::start(A_CLAIM).await;
        let llm = landscape_llm::LlamaClient::new(&stub.base);

        let _ = trust(
            &llm,
            "https://linear.app/security",
            TWO_STANDARDS,
            &mut |_| crate::Wanted::No,
        )
        .await;

        assert_eq!(stub.calls(), 1, "it asked again after being told to stop");
    }

    #[tokio::test]
    async fn a_run_told_to_stop_does_not_call_the_model_again() {
        let stub = StubModel::start(A_CAPABILITY).await;
        let llm = landscape_llm::LlamaClient::new(&stub.base);
        let page = twelve_windows();

        let mut asked = 0usize;
        let mut stop_at_once = |_: &[crate::Finding]| {
            asked += 1;
            crate::Wanted::No
        };
        extract(
            &llm,
            Answers::Features,
            "https://basecamp.com/features",
            &page,
            NaiveDate::from_ymd_opt(2026, 8, 5).unwrap(),
            &mut stop_at_once,
        )
        .await;

        assert_eq!(asked, 1, "the caller should be asked once, then not again");
        assert_eq!(
            stub.calls(),
            1,
            "a twelve-window page kept calling the model after being told to stop - which is the entire cost this exists to avoid"
        );
    }

    /// The Notion pricing page: five plan windows, one model call each.
    fn five_plan_windows() -> String {
        landscape_golden::pages::load()
            .expect("the page set loads")
            .into_iter()
            .find(|e| e.page == "notion-pricing.md")
            .expect("the five-window pricing page")
            .markdown()
            .expect("it reads")
    }

    const A_PLAN: &str = r#"{"plan_name":"Plus","price_usd":10.0,"billing_period":"monthly","evidence_quote":"$10 per member / month"}"#;

    #[tokio::test]
    async fn the_pricing_loop_stops_too() {
        // The same break in a different stage. Three loops call the model window by window,
        // and a fix applied to one of them looks identical in a diff to a fix applied to all
        // three — so more than one is asserted.
        let stub = StubModel::start(A_PLAN).await;
        let llm = landscape_llm::LlamaClient::new(&stub.base);
        let page = five_plan_windows();
        let windows = landscape_extract::span::every_plan(&page).len();
        assert!(windows > 1, "the fixture should offer several windows");

        let mut stop_at_once = |_: &[crate::Finding]| crate::Wanted::No;
        extract(
            &llm,
            Answers::Pricing,
            "https://www.notion.com/pricing",
            &page,
            NaiveDate::from_ymd_opt(2026, 8, 5).unwrap(),
            &mut stop_at_once,
        )
        .await;
        assert_eq!(
            stub.calls(),
            1,
            "pricing kept reading after being told to stop"
        );

        let stub = StubModel::start(A_PLAN).await;
        let llm = landscape_llm::LlamaClient::new(&stub.base);
        let mut carry_on = |_: &[crate::Finding]| crate::Wanted::Yes;
        extract(
            &llm,
            Answers::Pricing,
            "https://www.notion.com/pricing",
            &page,
            NaiveDate::from_ymd_opt(2026, 8, 5).unwrap(),
            &mut carry_on,
        )
        .await;
        assert_eq!(
            stub.calls(),
            windows,
            "and a run nobody stopped should read every plan on the page"
        );
    }

    #[tokio::test]
    async fn a_window_the_model_could_not_answer_is_still_a_chance_to_stop() {
        // Review found this. The stop was only asked about on the branch where a window
        // *worked*, so a run whose model calls are failing never learned its claim was gone
        // and read all twelve windows anyway.
        //
        // That is the worst place to miss it: a run erroring or producing ungrounded output is
        // a slow run, and a slow run is the one the staleness sweep takes away.
        let stub = StubModel::start("not json at all").await;
        let llm = landscape_llm::LlamaClient::new(&stub.base);
        let page = twelve_windows();

        let mut asked = 0usize;
        let mut stop_at_once = |_: &[crate::Finding]| {
            asked += 1;
            crate::Wanted::No
        };
        extract(
            &llm,
            Answers::Features,
            "https://basecamp.com/features",
            &page,
            NaiveDate::from_ymd_opt(2026, 8, 5).unwrap(),
            &mut stop_at_once,
        )
        .await;

        assert_eq!(asked, 1, "a failed window is still a window; ask after it");
        assert_eq!(
            stub.calls(),
            1,
            "the model kept being called for a run nobody wanted, because every answer failed to parse and nothing on that path ever asked whether to carry on"
        );
    }

    #[tokio::test]
    async fn an_ungrounded_answer_is_still_a_chance_to_stop() {
        // The other path that skipped the question: a features answer whose name is not in the
        // window is dropped, and the drop used to `continue` past the check.
        let stub = StubModel::start(
            r#"{"capability":"Invented Feature","qualifier":null,"evidence_quote":"nowhere"}"#,
        )
        .await;
        let llm = landscape_llm::LlamaClient::new(&stub.base);
        let page = twelve_windows();

        let mut stop_at_once = |_: &[crate::Finding]| crate::Wanted::No;
        extract(
            &llm,
            Answers::Features,
            "https://basecamp.com/features",
            &page,
            NaiveDate::from_ymd_opt(2026, 8, 5).unwrap(),
            &mut stop_at_once,
        )
        .await;

        assert_eq!(
            stub.calls(),
            1,
            "an ungrounded answer skipped the question and the run carried on"
        );
    }

    #[tokio::test]
    async fn the_identity_loop_stops_too() {
        // The third of the three model-backed loops, and the one that had no test until a
        // mutation showed the check could be deleted from it in silence. Fewest windows of the
        // three, so the cheapest to get wrong and the least noticed when it is.
        let page = landscape_golden::pages::load()
            .expect("the page set loads")
            .into_iter()
            .find(|e| e.page == "plausible-about.md")
            .expect("the about page that states all three facts")
            .markdown()
            .expect("it reads");
        let windows = landscape_extract::identity::every_fact(&page).len();
        assert!(windows > 1, "the fixture should offer several windows");

        let stub = StubModel::start("not json at all").await;
        let llm = landscape_llm::LlamaClient::new(&stub.base);
        let mut stop_at_once = |_: &[crate::Finding]| crate::Wanted::No;
        extract(
            &llm,
            Answers::Identity,
            "https://plausible.io/about",
            &page,
            NaiveDate::from_ymd_opt(2026, 8, 5).unwrap(),
            &mut stop_at_once,
        )
        .await;
        assert_eq!(
            stub.calls(),
            1,
            "identity kept reading after being told to stop"
        );

        let stub = StubModel::start("not json at all").await;
        let llm = landscape_llm::LlamaClient::new(&stub.base);
        let mut carry_on = |_: &[crate::Finding]| crate::Wanted::Yes;
        extract(
            &llm,
            Answers::Identity,
            "https://plausible.io/about",
            &page,
            NaiveDate::from_ymd_opt(2026, 8, 5).unwrap(),
            &mut carry_on,
        )
        .await;
        assert_eq!(
            stub.calls(),
            windows,
            "and a run nobody stopped should ask about every fact the page states"
        );
    }

    #[tokio::test]
    async fn a_run_nobody_stops_reads_every_window() {
        // The control. Without it a bug that abandoned *every* run would pass the test above
        // and look like a saving rather than like a report with one capability in it.
        let stub = StubModel::start(A_CAPABILITY).await;
        let llm = landscape_llm::LlamaClient::new(&stub.base);
        let page = twelve_windows();
        let windows = landscape_extract::capability::every_capability(&page)
            .windows
            .len();
        assert!(windows > 1, "the fixture should offer several windows");

        let mut carry_on = |_: &[crate::Finding]| crate::Wanted::Yes;
        extract(
            &llm,
            Answers::Features,
            "https://basecamp.com/features",
            &page,
            NaiveDate::from_ymd_opt(2026, 8, 5).unwrap(),
            &mut carry_on,
        )
        .await;

        assert_eq!(
            stub.calls(),
            windows,
            "a run nobody interrupted should read every window the page offered"
        );
    }

    #[test]
    fn a_changelog_is_read_with_no_model() {
        // The whole reason this stage takes no client. ARCHITECTURE §5.4 puts dates on the
        // deterministic side, and this asserts that the code follows.
        let today = NaiveDate::from_ymd_opt(2026, 8, 4).unwrap();
        let page = "- Jul 14, 2026\n## Shipped annotations\nYou can now annotate.";
        let outcome = changes(page, today);
        assert_eq!(outcome.claims.len(), 1);
        assert!(outcome.summary.contains("1 change(s) in 90 days"));
    }

    #[test]
    fn a_page_with_no_dates_says_so_rather_than_nothing() {
        let today = NaiveDate::from_ymd_opt(2026, 8, 4).unwrap();
        let outcome = changes("# Releases\nA feature called Releases.", today);
        assert!(outcome.claims.is_empty());
        assert_eq!(outcome.summary, "no dated entries on the page");
    }

    #[test]
    fn what_falls_outside_the_window_is_counted_in_the_summary() {
        let today = NaiveDate::from_ymd_opt(2026, 8, 4).unwrap();
        let page = "2026-07-14 - Recent\n2024-01-01 - Ancient";
        let outcome = changes(page, today);
        assert!(outcome.summary.contains("1 older"), "{}", outcome.summary);
    }

    #[test]
    fn a_careers_page_is_read_with_no_model() {
        // The second stage that takes no client, and the reason `is_deterministic` is now a
        // two-armed match rather than a single question.
        let page =
            "## Open roles\nSenior / Staff Product Engineer\nEurope\nSenior Counsel\nNorth America";
        let outcome = direction(page);
        assert_eq!(outcome.claims.len(), 2);
        assert_eq!(outcome.summary, "2 open roles listed");
        assert_eq!(
            outcome.claims[0].text,
            "lists an open role: Senior / Staff Product Engineer"
        );
        // The evidence is the line, so a reader can find it on the page.
        assert_eq!(outcome.claims[0].quote, "Senior / Staff Product Engineer");
    }

    #[test]
    fn a_hiring_freeze_says_so_rather_than_nothing() {
        // A careers page with nothing on it is a finding about the company, and it has to be
        // distinguishable from a page we could not read - which is the rule the coverage note
        // exists for and the one an empty section quietly breaks.
        let outcome = direction("## Open roles\nWe have no open roles right now.");
        assert!(outcome.claims.is_empty());
        assert_eq!(outcome.summary, "no open roles listed on the page");
    }

    #[test]
    fn what_the_cap_left_out_is_said_on_the_run_log() {
        let mut page = String::from("## Open positions\n");
        for i in 0..landscape_extract::hiring::MAX_ROLES + 3 {
            page.push_str(&format!("Senior Software Engineer, Team {i}\n"));
        }
        let outcome = direction(&page);
        assert_eq!(
            outcome.claims.len(),
            landscape_extract::hiring::MAX_ROLES,
            "the cap applies"
        );
        assert!(
            outcome.details.iter().any(|d| d.contains("read 20 of 23")),
            "a short list with nothing beside it is a wrong list: {:?}",
            outcome.details
        );
    }

    #[test]
    fn the_two_silences_do_not_read_alike() {
        // **The distinction the report needs, and the one this stage used to bury in a run
        // log.** A careers page advertising nothing is news about the company; a page whose
        // list we could not locate is our gap, and calling that one "no open roles" would
        // state something this pipeline does not know.
        let freeze = direction("## Open roles\nWe have no open roles right now.");
        assert_eq!(freeze.summary, "no open roles listed on the page");

        let unreadable = direction("Why we love working here\nKelsey Weber , Engineering Manager");
        assert!(unreadable.claims.is_empty());
        assert_eq!(
            unreadable.summary,
            "the page does not say where its open roles are listed, so none were read"
        );
    }

    #[test]
    fn a_testimonial_on_an_unannounced_page_never_becomes_a_claim() {
        // Review's reproduction, held at the surface that publishes rather than at the scanner:
        // the run log saying the page was unscoped never reached a report, so the only thing
        // between a named employee and `lists an open role: ...` was a line nobody reads.
        let outcome = direction("Kelsey Weber , Engineering Manager\nStaff Data Engineer");
        let texts: Vec<&str> = outcome.claims.iter().map(|c| c.text.as_str()).collect();
        assert!(texts.is_empty(), "a person became a vacancy: {texts:?}");
    }

    #[test]
    fn the_prompts_never_name_a_real_company_as_an_example() {
        // BENCHMARKS Run 8: a worked example naming Message Boards put a Basecamp feature
        // into a report about Linear. A prompt example is a source with no URL and nothing
        // to cite, and this is the test that keeps one from creeping back.
        let span = Span {
            text: "## Pro\n$15".to_owned(),
            starts_at_line: 0,
            heading: Some("## Pro".to_owned()),
            score: 20,
        };
        let prompts = [
            pricing_prompt("https://e.com", &span),
            capability_prompt("https://e.com", &span),
            identity_prompt(
                "https://e.com",
                landscape_extract::identity::Fact::Founded,
                &span,
            ),
        ];
        for prompt in prompts {
            for name in [
                "Message Boards",
                "Basecamp",
                "Linear",
                "Notion",
                "Plausible",
            ] {
                assert!(!prompt.contains(name), "prompt names {name}");
            }
        }
    }

    #[test]
    fn the_prompt_version_is_stated_so_two_runs_can_be_compared() {
        assert_eq!(crate::PROMPT_VERSION, 1);
    }

    #[test]
    fn a_page_with_nothing_to_ask_about_is_still_a_whole_answer() {
        // "Nothing found" is an answer, and an expensive one to reach: it means every scanner
        // ran. Marking it partial would re-scan the same page for every later reader.
        for outcome in [
            direction("Why we love working here"),
            changes("# Home\nNothing dated here", "2026-08-09".parse().unwrap()),
        ] {
            assert_eq!(outcome.settled, crate::Settled::Complete);
        }
    }

    #[test]
    fn a_parsed_answer_is_always_whole() {
        // Neither of these asks a model, so neither has anything that can half-happen — and
        // that is why they are the two questions a reader still gets when the model is down.
        let roles = direction("## Open roles\n- Staff Data Engineer");
        assert_eq!(roles.settled, crate::Settled::Complete);
        assert!(!roles.claims.is_empty(), "this test needs a real answer");

        let dated = changes(
            "# Changelog\n## 2026-08-01\nShipped the thing",
            "2026-08-09".parse().unwrap(),
        );
        assert_eq!(dated.settled, crate::Settled::Complete);
    }

    #[tokio::test]
    async fn a_window_the_model_never_answered_is_not_a_whole_answer() {
        // **The rule `memo::Extractions` rests on, at the place that knows.** Nothing is
        // listening on port 1, so every window fails — and an outcome that reports a model
        // error must never be remembered, or the outage is replayed to every later reader.
        let down = landscape_llm::LlamaClient::new("http://127.0.0.1:1".to_owned());
        let page = "# Pricing\n\n## Pro\n\n$15 per user per month, billed annually.\n";
        let outcome = pricing(&down, "https://e.com/pricing", page, &mut |_| {
            crate::Wanted::Yes
        })
        .await;

        assert!(
            outcome.details.iter().any(|d| d.starts_with("model error")),
            "this test needs the model to have failed: {:?}",
            outcome.details
        );
        assert_eq!(
            outcome.settled,
            crate::Settled::Partial,
            "an outage was marked as a reusable answer"
        );
    }

    #[tokio::test]
    async fn a_run_nobody_is_waiting_for_is_not_a_whole_answer() {
        // **The other way an answer comes back short, and the model answering is the point.**
        // A stopped run gathers real facts from the windows it reached — nothing failed — so
        // this needs a model that succeeds, or the failure path would be doing the work and the
        // abandonment itself would stay untested. What was gathered is worth showing the reader
        // who caused it and worth nothing to the next one, who wants the whole page.
        let page = five_plan_windows();
        let windows = landscape_extract::span::every_plan(&page).len();
        assert!(windows > 1, "the fixture should offer several windows");

        let stub = StubModel::start(A_PLAN).await;
        let llm = landscape_llm::LlamaClient::new(&stub.base);
        let stopped = pricing(&llm, "https://www.notion.com/pricing", &page, &mut |_| {
            crate::Wanted::No
        })
        .await;
        assert!(
            !stopped.claims.is_empty(),
            "this test needs the model to have answered before the stop"
        );
        assert!(
            !stopped.details.iter().any(|d| d.starts_with("model error")),
            "nothing should have failed here: {:?}",
            stopped.details
        );
        assert_eq!(
            stopped.settled,
            crate::Settled::Partial,
            "a page read as far as its first window was remembered as the whole page"
        );

        // And the same page read to the end is the answer the next reader may have.
        let stub = StubModel::start(A_PLAN).await;
        let llm = landscape_llm::LlamaClient::new(&stub.base);
        let whole = pricing(&llm, "https://www.notion.com/pricing", &page, &mut |_| {
            crate::Wanted::Yes
        })
        .await;
        assert_eq!(whole.settled, crate::Settled::Complete);
        assert_eq!(stub.calls(), windows, "every window should have been read");
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod judging_assurances {
    use super::{judge_assurance, Judged};
    use landscape_core::{Assurance, AssuranceClaim};

    fn claim(quote: &str) -> AssuranceClaim {
        AssuranceClaim {
            status: Some(Assurance::Holds),
            evidence_quote: Some(quote.to_owned()),
        }
    }

    const SECTION: &str = "Linear undergoes regular SOC 2 Type II audits.
ISO 27001 is on our roadmap.";

    #[test]
    fn a_claim_quoted_from_the_section_is_kept() {
        assert_eq!(
            judge_assurance(
                &claim("regular SOC 2 Type II audits"),
                SECTION,
                "SOC 2 Type II"
            ),
            Judged::Keep
        );
    }

    #[test]
    fn evidence_about_a_neighboring_standard_is_not_a_claim_about_this_one() {
        // **The failure review found in the regression written to prove the class was closed.**
        // The window holds both standards, so the same three lines are handed over twice. An
        // answer about SOC 2, relabeled ISO 27001, published a certification claim on evidence
        // about a different certification - and the quote was verbatim, and the name was the
        // scanner's, so every check passed.
        assert_eq!(
            judge_assurance(
                &claim("undergoes regular SOC 2 Type II audits"),
                SECTION,
                "ISO 27001"
            ),
            Judged::EvidenceIsAboutAnother
        );
    }

    #[test]
    fn evidence_that_names_no_standard_is_the_ordinary_case_and_is_kept() {
        // *"We are certified and audited annually."* beside the name. Refusing this would
        // throw away most true claims to catch a rare wrong one.
        const PLAIN: &str = "ISO 27001\nWe are certified and audited annually.";
        assert_eq!(
            judge_assurance(&claim("certified and audited annually"), PLAIN, "ISO 27001"),
            Judged::Keep
        );
    }

    #[test]
    fn a_quote_naming_the_standard_asked_about_is_kept_at_either_precision() {
        // `SOC 2` in the evidence for a scanned `SOC 2 Type II` is the same certification, not
        // a different one - the check must not turn a precision difference into a rejection.
        assert_eq!(
            judge_assurance(&claim("our SOC 2 report"), SECTION, "SOC 2 Type II"),
            Judged::QuoteNotInTheSection
        );
    }

    #[test]
    fn a_quote_that_is_not_in_the_section_takes_the_claim_with_it() {
        // `ARCHITECTURE.md`: *"a claim whose evidence quote is absent from its cited source is
        // deleted"*. This used to keep the status and only count the quote, which published an
        // unsupported compliance claim with a run-log line standing in for a check.
        assert_eq!(
            judge_assurance(&claim("we are fully certified"), SECTION, "SOC 2 Type II"),
            Judged::QuoteNotInTheSection
        );
    }

    #[test]
    fn a_status_with_no_quote_at_all_is_not_a_claim() {
        // "There is no quote to check" reads as "the quote is fine", which is the shape of
        // every check-that-cannot-fail in the register.
        let bare = AssuranceClaim {
            status: Some(Assurance::Holds),
            evidence_quote: None,
        };
        assert_eq!(
            judge_assurance(&bare, SECTION, "SOC 2 Type II"),
            Judged::QuoteNotInTheSection
        );

        let blank = AssuranceClaim {
            status: Some(Assurance::Holds),
            evidence_quote: Some("   ".to_owned()),
        };
        assert_eq!(
            judge_assurance(&blank, SECTION, "SOC 2 Type II"),
            Judged::QuoteNotInTheSection
        );
    }

    #[test]
    fn claiming_nothing_and_quoting_nothing_is_not_a_failure() {
        // The page named the standard and said nothing about it. There is no claim to support,
        // so there is nothing to drop.
        assert_eq!(
            judge_assurance(&AssuranceClaim::empty(), SECTION, "SOC 2 Type II"),
            Judged::Keep
        );
    }

    #[test]
    fn evidence_written_with_the_other_numeral_is_the_same_standard() {
        // `SOC 2 Type 2` and `SOC 2 Type II` are one report, and an auditor writes it either
        // way. Treating them as different threw away a correct quote.
        const EITHER: &str = "Our SOC 2 Type 2 report is available under NDA.";
        assert_eq!(
            judge_assurance(
                &claim("SOC 2 Type 2 report is available"),
                EITHER,
                "SOC 2 Type II"
            ),
            Judged::Keep
        );
    }

    #[test]
    fn the_standard_reported_is_the_one_the_scanner_asked_about() {
        // **The failure review found, now unrepresentable.** This section names two standards.
        // The model used to be asked for the name as well, so an answer about `ISO 27001` -
        // carrying a verbatim SOC 2 quote - passed both checks and published a certification
        // claim about the wrong standard.
        //
        // The name is attached from the scanner after generation. Whatever the model says, the
        // extraction is about the standard this iteration asked about.
        let got = claim("regular SOC 2 Type II audits").about("SOC 2 Type II");
        assert_eq!(got.standard.as_deref(), Some("SOC 2 Type II"));
    }

    #[test]
    fn a_shortened_spelling_cannot_come_back_from_the_model() {
        // The other half: returning `SOC 2` for a scanned `SOC 2 Type II` passed a containment
        // check and quietly undid the precise-spelling rule. There is nothing to return now.
        let got = AssuranceClaim::empty().about("SOC 2 Type II");
        assert_eq!(got.standard.as_deref(), Some("SOC 2 Type II"));
        assert!(got.status.is_none());
    }
}
