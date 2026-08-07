//! Which page is read first, and how many are read at all.
//!
//! `ROADMAP.md` §2·D's risk section names latency as the thing no other item fixes, and
//! `PRODUCT_SPEC.md` §2.1A puts numbers on it: **content in twenty to forty seconds**, a whole
//! report inside ninety to a hundred and eighty. Neither was being met, and the reason is not
//! the model's speed — it is what the pipeline chooses to spend the model on, and when.
//!
//! Two decisions live here, both about time and neither about correctness.
//!
//! # 1. The page that needs no model goes first
//!
//! A changelog is **parsed**, not generated (`ARCHITECTURE.md` §5.4): dated entries are on the
//! deterministic side of the line. Reading it costs one fetch. Every other question costs a
//! fetch *and* a model call per window — a real capability page is a dozen of them.
//!
//! Discovery hands its pages back ordered by question, pricing first, so the first thing a
//! reader waited for used to be a chain of model calls. Moving the deterministic page to the
//! front changes first content from *"after the model has finished a page"* to *"after one
//! fetch"*, and costs the pricing section about a second.
//!
//! **It is not a promise that a changelog exists.** Basecamp publishes none, and then this
//! reorders nothing and the run proceeds exactly as before.
//!
//! # 2. One page a question, and the rest are named
//!
//! Discovery admits eight pages, round-robin across six questions — so a company gets a first
//! page for each question and then a *second* page for some of them. The second is where the
//! wait doubles: it is another dozen model calls for the question that already has an answer,
//! while a reader watches.
//!
//! At rung 0 each question is worth one page that needs a model. **The pages that are not read
//! are named in the report**, because a shorter list with nothing beside it is not a shorter
//! list — it is a wrong one, which is the same rule the capability cap and the subject cap
//! already follow.
//!
//! Raising [`PAGES_PER_QUESTION`] is a decision about the wait rather than a change to the
//! design, which is why it is a named constant here and not a number inside a loop.

use landscape_discover::probes::Answers;
use landscape_discover::rank::Candidate;

/// How many pages needing a model each question is worth, at rung 0.
///
/// One. A second page for a question the run has already answered is the single largest
/// avoidable cost in a run: it is a whole page of windows, one model call each, spent on the
/// question least likely to change its answer.
pub const PAGES_PER_QUESTION: usize = 1;

/// Whether a question is answered without a model.
///
/// Two of the six: [`Answers::Changes`] and [`Answers::Direction`]. A dated entry and a job
/// title are both things somebody wrote on a page on purpose, and reading either is
/// transcription. It is a `match` rather than a list so that adding a question kind has to
/// decide this, rather than defaulting to "needs a model" by omission.
#[must_use]
pub const fn is_deterministic(question: Answers) -> bool {
    matches!(question, Answers::Changes | Answers::Direction)
}

/// What one run will read, and what it will not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    /// In the order they are read.
    pub read: Vec<Candidate>,
    /// Admitted by discovery, and left for the wait. Named in the report.
    pub skipped: Vec<Candidate>,
}

impl Plan {
    /// The sentence a reader sees when pages were left unread. `None` when none were.
    ///
    /// **A note on the report rather than a line in a log.** The reader is the person paying
    /// for the shorter wait, so they are the person who has to be told what it cost.
    #[must_use]
    pub fn note(&self) -> Option<String> {
        if self.skipped.is_empty() {
            return None;
        }
        let paths: Vec<&str> = self.skipped.iter().map(|c| c.url.as_str()).collect();
        Some(format!(
            "Read the first page for each question. {} further page(s) were found and not \
             read, because a second page for a question roughly doubles the wait: {}.",
            self.skipped.len(),
            paths.join(", ")
        ))
    }
}

/// Decide the order and the budget together.
///
/// Order is stable within each half: whatever discovery decided about *which* pricing page is
/// best is not second-guessed here. This chooses when, and how many.
#[must_use]
pub fn plan(sources: &[Candidate]) -> Plan {
    let mut read: Vec<Candidate> = Vec::with_capacity(sources.len());
    let mut skipped: Vec<Candidate> = Vec::new();

    // The budget is per question and counts only pages that need a model. A deterministic page
    // is a fetch, so capping it would buy nothing and lose a section.
    let mut spent: Vec<(Answers, usize)> = Vec::new();
    let mut affordable: Vec<Candidate> = Vec::with_capacity(sources.len());
    for candidate in sources {
        if is_deterministic(candidate.answers) {
            affordable.push(candidate.clone());
            continue;
        }
        let used = spent
            .iter_mut()
            .find(|(q, _)| *q == candidate.answers)
            .map(|(_, n)| n);
        match used {
            Some(n) if *n >= PAGES_PER_QUESTION => skipped.push(candidate.clone()),
            Some(n) => {
                *n += 1;
                affordable.push(candidate.clone());
            }
            None => {
                spent.push((candidate.answers, 1));
                affordable.push(candidate.clone());
            }
        }
    }

    // Deterministic first, everything else behind it, each half in discovery's order.
    read.extend(
        affordable
            .iter()
            .filter(|c| is_deterministic(c.answers))
            .cloned(),
    );
    read.extend(
        affordable
            .iter()
            .filter(|c| !is_deterministic(c.answers))
            .cloned(),
    );
    Plan { read, skipped }
}

/// How many model calls one page would cost, counted without asking a model.
///
/// **Every extractor works a window at a time and makes one call per window**, so the windows
/// a page holds are its price. This is the same span-finding code the run uses, called for its
/// count rather than its content — not an estimate of the pipeline but the pipeline's own
/// arithmetic, which is the only kind of prediction worth printing.
///
/// **The quality gate is applied here too**, and review is the reason: `analyse_with` skips a
/// page that is not [`worth_extracting`] before any extractor sees it, and a counter that did
/// not would report a call for a two-line page whose only words are a plan name and a price —
/// below `MIN_WORDS`, so the real run never opens it, while `every_plan` happily finds a window
/// on it. A prediction of the run has to include everything the run decides, or it is a
/// prediction of a different program.
///
/// [`worth_extracting`]: landscape_extract::quality::Quality::worth_extracting
#[must_use]
pub fn model_calls_for(question: Answers, markdown: &str) -> usize {
    if !landscape_extract::quality::assess(markdown)
        .quality
        .worth_extracting()
    {
        return 0;
    }
    match question {
        Answers::Pricing => landscape_extract::span::every_plan(markdown).len(),
        Answers::Features => landscape_extract::capability::every_capability(markdown)
            .windows
            .len(),
        Answers::Identity => landscape_extract::identity::every_fact(markdown).len(),
        // One call per standard the page names. The scanner finds them without a model, so a
        // page full of reassurance and no named standard costs nothing at all.
        Answers::Trust => landscape_extract::assurance::every_assurance(markdown)
            .named
            .len(),
        // Parsed, not generated. The reason both are read first.
        Answers::Changes | Answers::Direction => 0,
    }
}

/// Whether a page is a *chance* of content — not a promise of it.
///
/// **The distinction matters and review made it.** For a deterministic page this is exact: the
/// dated entries are already parsed, so either they are there or they are not. For a
/// model-backed page it means only that a window exists to ask about — whether the model
/// answers, and whether the answer survives grounding, is unknowable without running it.
///
/// So everything counted from this is named an *opportunity*. The measured figure is the one
/// `landscape read` prints after a real run.
#[must_use]
pub fn may_yield_content(question: Answers, markdown: &str) -> bool {
    // Both deterministic questions are exact here, and both need the quality gate first: an
    // unreadable page yields nothing whatever its parser would have found in the wreckage.
    if is_deterministic(question) {
        if !landscape_extract::quality::assess(markdown)
            .quality
            .worth_extracting()
        {
            return false;
        }
        return match question {
            Answers::Changes => !landscape_extract::changes::every_change(markdown)
                .entries
                .is_empty(),
            _ => !landscape_extract::hiring::every_role(markdown)
                .roles
                .is_empty(),
        };
    }
    model_calls_for(question, markdown) > 0
}

/// Calls in total, and calls before the **first chance** of something on screen.
///
/// **One call on the page that first offers content, not all of its windows.** Review found the
/// arithmetic: extractors report progress after *every* window, so a twelve-window first page
/// gives a reader their first chance after one call rather than twelve. Counting the page as
/// twelve made this pipeline look worse than it is, and made a number that is already an
/// approximation look precise.
#[must_use]
pub fn tally(pages: impl Iterator<Item = (usize, bool)>) -> (usize, usize) {
    let (mut total, mut before, mut seen) = (0usize, 0usize, false);
    for (calls, may_yield) in pages {
        total += calls;
        if !seen {
            if may_yield {
                // The first opportunity on this page: one call, or none at all if the page
                // needs no model.
                before += usize::from(calls > 0);
                seen = true;
            } else {
                // Nothing here to see, so everything it spends is spent before content.
                before += calls;
            }
        }
    }
    (total, before)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use landscape_discover::rank::Via;

    fn c(url: &str, answers: Answers) -> Candidate {
        Candidate {
            url: url.to_owned(),
            answers,
            via: Via::Probe,
        }
    }

    /// What discovery really hands back for a site with a changelog: round-robin by question,
    /// pricing first, second pages behind the first ones.
    fn as_discovery_returns_them() -> Vec<Candidate> {
        vec![
            c("https://e.com/pricing", Answers::Pricing),
            c("https://e.com/features", Answers::Features),
            c("https://e.com/changelog", Answers::Changes),
            c("https://e.com/about", Answers::Identity),
            c("https://e.com/plans", Answers::Pricing),
            c("https://e.com/product", Answers::Features),
            c("https://e.com/blog", Answers::Changes),
            c("https://e.com/company", Answers::Identity),
        ]
    }

    #[test]
    fn the_page_that_needs_no_model_is_read_first() {
        // The whole point. First content used to wait on a chain of model calls; it now waits
        // on one fetch.
        let plan = plan(&as_discovery_returns_them());
        assert_eq!(
            plan.read.first().map(|c| c.url.as_str()),
            Some("https://e.com/changelog"),
            "{:?}",
            plan.read.iter().map(|c| &c.url).collect::<Vec<_>>()
        );
    }

    #[test]
    fn a_company_with_no_changelog_reads_exactly_what_it_used_to() {
        // Basecamp publishes none. Reordering must not invent a different run for the
        // companies this does nothing for.
        let sources: Vec<Candidate> = as_discovery_returns_them()
            .into_iter()
            .filter(|c| !is_deterministic(c.answers))
            .collect();
        let plan = plan(&sources);
        let expected: Vec<&str> = vec![
            "https://e.com/pricing",
            "https://e.com/features",
            "https://e.com/about",
        ];
        assert_eq!(
            plan.read.iter().map(|c| c.url.as_str()).collect::<Vec<_>>(),
            expected
        );
    }

    #[test]
    fn every_question_keeps_its_first_page() {
        // The budget must never cost a section. Dropping the only pricing page to save a
        // minute would be the wait fixed by removing the report.
        let plan = plan(&as_discovery_returns_them());
        for question in [
            Answers::Pricing,
            Answers::Features,
            Answers::Changes,
            Answers::Identity,
        ] {
            assert!(
                plan.read.iter().any(|c| c.answers == question),
                "{question:?} lost its only page: {:?}",
                plan.read.iter().map(|c| &c.url).collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn a_second_page_for_a_question_is_skipped_and_named() {
        let plan = plan(&as_discovery_returns_them());
        let skipped: Vec<&str> = plan.skipped.iter().map(|c| c.url.as_str()).collect();
        assert_eq!(
            skipped,
            [
                "https://e.com/plans",
                "https://e.com/product",
                "https://e.com/company"
            ]
        );
        let note = plan.note().expect("pages were skipped, so there is a note");
        assert!(note.contains("/plans"), "{note}");
        assert!(note.contains("3 further page(s)"), "{note}");
    }

    #[test]
    fn a_deterministic_page_is_never_the_thing_that_is_skipped() {
        // A changelog costs a fetch. Capping it saves nothing and loses the one section that
        // arrives quickly.
        let plan = plan(&as_discovery_returns_them());
        assert_eq!(
            plan.read
                .iter()
                .filter(|c| c.answers == Answers::Changes)
                .count(),
            2,
            "both deterministic pages should be read"
        );
        assert!(plan.skipped.iter().all(|c| !is_deterministic(c.answers)));
    }

    #[test]
    fn nothing_skipped_means_nothing_said() {
        // A note about a cap that did not bite is noise, and noise is what teaches a reader to
        // skip the notes that matter.
        let plan = plan(&[
            c("https://e.com/pricing", Answers::Pricing),
            c("https://e.com/changelog", Answers::Changes),
        ]);
        assert_eq!(plan.note(), None);
    }

    #[test]
    fn a_page_below_the_quality_floor_costs_what_the_run_will_spend_on_it() {
        // Review found this. `analyse_with` skips a page that is not worth extracting before
        // any extractor sees it, so the run makes no call — but the span finder, handed the
        // same markdown directly, still finds a priced-plan window. A counter that reports one
        // call there is predicting a different program.
        let too_thin = "# Pro
$10/month";
        assert!(
            !landscape_extract::quality::assess(too_thin)
                .quality
                .worth_extracting(),
            "the fixture is no longer below the floor, so this test proves nothing"
        );
        assert!(
            !landscape_extract::span::every_plan(too_thin).is_empty(),
            "the fixture has no span, so a missing quality gate would not show up"
        );
        assert_eq!(model_calls_for(Answers::Pricing, too_thin), 0);
        assert!(!may_yield_content(Answers::Pricing, too_thin));
    }

    #[test]
    fn a_trust_page_costs_one_call_for_each_standard_it_names() {
        // The prediction has to include the newest extractor, or `landscape cost` reports a
        // run that no longer exists - the same rule the quality gate is here for.
        let page = "# Security

We hold SOC 2 Type II.

We are ISO 27001 certified too.";
        assert_eq!(model_calls_for(Answers::Trust, page), 2);
        assert!(may_yield_content(Answers::Trust, page));
    }

    #[test]
    fn a_security_page_that_names_no_standard_costs_nothing() {
        // The common case, and the reason the scanner runs before the model: a page of
        // reassurance with nothing named spends no calls at all.
        let page = "# Security

We take security seriously and encrypt everything in transit.";
        assert_eq!(model_calls_for(Answers::Trust, page), 0);
        assert!(!may_yield_content(Answers::Trust, page));
    }

    #[test]
    fn a_careers_page_costs_nothing_however_many_roles_it_lists() {
        // The second deterministic question, and the whole reason it is one. A page listing
        // twenty vacancies is twenty facts for the price of a fetch — so a prediction that
        // charged a call per role would report a run this pipeline does not perform.
        let page = "# Careers

## Open roles

Senior / Staff Product Engineer

Senior Counsel

Product Marketing Manager

We are remote-first and hire across Europe and North America.";
        assert!(is_deterministic(Answers::Direction));
        assert_eq!(model_calls_for(Answers::Direction, page), 0);
        assert!(may_yield_content(Answers::Direction, page));
    }

    #[test]
    fn a_careers_page_with_nothing_open_promises_nothing() {
        // Exact, because it is parsed rather than asked: either the roles are on the page or
        // they are not. A hiring freeze is a finding, and it is not a chance of content.
        let page = "# Careers

## Open roles

We have no open roles right now, but we are always glad to hear from people.

Submit your resume and we will keep in touch when something opens up.";
        assert!(!may_yield_content(Answers::Direction, page));
    }

    #[test]
    fn an_unreadable_careers_page_promises_nothing_either() {
        // The gate the other questions get, and review is the reason it is stated for this one
        // too: `analyse_with` never opens a page below `MIN_WORDS`, so a prediction that
        // counted what a parser *would* have found on it is a prediction of a different
        // program.
        //
        // **Announced, deliberately.** A page with no `Open roles` heading now yields nothing
        // whatever its length, so an unannounced fixture here would pass without the quality
        // gate doing anything — a test named after one guard and held up by another, which is
        // entry 32 of the register and was found in this very file.
        let too_thin = "## Open roles\nProduct Engineer";
        assert!(!may_yield_content(Answers::Direction, too_thin));

        // The same page, long enough to be worth opening, does promise content — so the
        // assertion above is about the gate and not about the fixture being unreadable twice.
        let long_enough = format!(
            "## Open roles\nProduct Engineer\n{}",
            "Linear is a purpose-built tool for planning and building products. ".repeat(12)
        );
        assert!(may_yield_content(Answers::Direction, &long_enough));
    }

    #[test]
    fn the_first_chance_of_content_is_one_call_and_not_a_whole_page() {
        // Extractors report after every window, so a twelve-window first page gives a reader
        // their first chance after one call. Counting twelve made the pipeline look worse than
        // it is — and made a number that is already an approximation look precise.
        assert_eq!(tally([(12, true), (8, true)].into_iter()), (20, 1));
    }

    #[test]
    fn a_page_that_can_show_nothing_is_charged_in_full_before_content() {
        // The other half. A page with no windows produces nothing, so everything spent on the
        // pages before the first real chance still counts as waiting.
        assert_eq!(tally([(3, false), (5, true)].into_iter()), (8, 4));
    }

    #[test]
    fn a_deterministic_first_page_costs_nothing_before_content() {
        // The whole point of reading it first, as arithmetic.
        assert_eq!(tally([(0, true), (12, true)].into_iter()), (12, 0));
    }

    #[test]
    fn nothing_admitted_is_not_a_panic() {
        let plan = plan(&[]);
        assert!(plan.read.is_empty() && plan.skipped.is_empty());
        assert_eq!(plan.note(), None);
    }

    #[test]
    fn every_admitted_page_is_either_read_or_named() {
        // The rule that makes the budget honest rather than quiet: a page discovery admitted
        // has to end up somewhere a reader can see.
        let sources = as_discovery_returns_them();
        let plan = plan(&sources);
        assert_eq!(plan.read.len() + plan.skipped.len(), sources.len());
        for source in &sources {
            assert!(
                plan.read.contains(source) || plan.skipped.contains(source),
                "{} vanished",
                source.url
            );
        }
    }
}
