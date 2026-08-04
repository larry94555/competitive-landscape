//! Running one page's extractor, and the prompts that go with it.
//!
//! Moved here from the `read` command whole. The prompts are worth reading beside each other:
//! three of them ask a model for a small thing about a small window, and the fourth asks
//! nothing of a model at all, because [`ARCHITECTURE.md`] §5.4 puts dates on the deterministic
//! side and dates are what models most often invent.
//!
//! Every answer is checked before it becomes a claim. The checks differ because the facts do:
//!
//! | | checked by |
//! |---|---|
//! | a price | the quote is verbatim |
//! | a capability name | every word of it is in the section — it is a paraphrase by design |
//! | a date | nothing. It was parsed, not generated |
//! | a founding year | the digits are in the section, as a whole number |
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
pub(crate) type Progress<'a> = &'a mut dyn FnMut(&[crate::Finding]);

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
        _ => Outcome {
            claims: Vec::new(),
            summary: format!("no extractor yet for {} pages", question.name()),
            details: Vec::new(),
            window_words: 0,
        },
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
        };
    }

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
                // One plan is enough to put the section on somebody's screen.
                so_far(&crate::claims_from_pricing(&PagePricing::assembled(
                    extracted.clone(),
                )));
            }
            Err(e) => details.push(format!("model error: {e}")),
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
        };
    }

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
                if !got.name_is_from(&section) {
                    ungrounded += 1;
                    continue;
                }
                if !got.quote_is_verbatim(&section) {
                    unsupported += 1;
                }
                extracted.push(got);
                so_far(&crate::claims_from_features(&PageFeatures::assembled(
                    extracted.clone(),
                    found.considered,
                )));
            }
            Err(e) => details.push(format!("model error: {e}")),
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
    }
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
        };
    }

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
                so_far(&crate::claims_from_identity(&PageIdentity::assembled(
                    extracted.clone(),
                )));
            }
            Err(e) => details.push(format!("model error: {e}")),
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

/// The capability prompt. Normalisation, which is all §5.4 asks a model for here.
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
}
