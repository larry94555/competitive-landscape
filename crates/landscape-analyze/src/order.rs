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
/// Only [`Answers::Changes`]. It is a `match` rather than a list so that adding a question kind
/// has to decide this, rather than defaulting to "needs a model" by omission.
#[must_use]
pub const fn is_deterministic(question: Answers) -> bool {
    matches!(question, Answers::Changes)
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
#[must_use]
pub fn model_calls_for(question: Answers, markdown: &str) -> usize {
    match question {
        Answers::Pricing => landscape_extract::span::every_plan(markdown).len(),
        Answers::Features => landscape_extract::capability::every_capability(markdown)
            .windows
            .len(),
        Answers::Identity => landscape_extract::identity::every_fact(markdown).len(),
        // Parsed, not generated. The reason it is read first.
        Answers::Changes => 0,
        // No extractor yet, so the page is not opened and nothing is spent.
        _ => 0,
    }
}

/// Whether a page would put anything on screen, counted the same way.
///
/// Used to say *when* a reader first sees something rather than only what a run costs: a page
/// with no windows is free and also produces nothing, and calling that "first content" would
/// make the number this exists to report meaningless.
#[must_use]
pub fn yields_content(question: Answers, markdown: &str) -> bool {
    match question {
        Answers::Changes => !landscape_extract::changes::every_change(markdown)
            .entries
            .is_empty(),
        _ => model_calls_for(question, markdown) > 0,
    }
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
