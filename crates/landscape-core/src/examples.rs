//! Ideas somebody can click, over companies that really answer.
//!
//! `ROADMAP.md` §2·D item D4. A demo you send someone fails at the first screen if the first
//! screen is an empty box: a reader who has never used this does not know what a good prompt
//! looks like, and the prompts they invent are the ones this pipeline cannot resolve —
//! *"a tool for small farms"* names no site, so the run fails with a reason and the demo is
//! over before anything was read.
//!
//! # An example is an idea, and nothing else
//!
//! **[`Example::prompt`] is the idea verbatim.** It used to be `"{idea} - {a.com vs b.com}"`,
//! which made every example a *named set*: the reader typed nothing, the pipeline discovered
//! nothing, and the first screen of the product demonstrated the one path where discovery does
//! not run. A reader clicking *"privacy-friendly website analytics"* wants to see this find the
//! companies, not to be handed two we picked.
//!
//! # What is curated, and what is not
//!
//! **Only the ideas.** Everything a report says is fetched, quoted and cited at the moment
//! somebody clicks — there are no stored answers here, no cached reports, and no text that ends
//! up in a claim or in the box.
//!
//! [`Example::companies`] survives that change as a **fixture, not an input**: the domains that
//! were checked by hand for a property the pipeline cannot promise of an arbitrary site — that
//! discovery finds pages worth reading. `landscape examples --check` still runs them, and they
//! are what a reader of that output compares live discovery against. **Nothing puts them in
//! front of somebody using the product**, and the API does not send them.
//!
//! # Why this is data in `landscape-core` rather than a list in the web app
//!
//! Two copies of a curated list drift, and the copy that drifts is the one nobody tests. Here
//! it is reachable by the API that serves it, by the command that checks it, and by the test in
//! `landscape-analyze` that runs each prompt through **the real prompt parser** and asserts the
//! companies come back out.
//!
//! # The wait these imply, and what changed about it
//!
//! **An idea is a description, so clicking one runs discovery.** That is the point, and it has
//! two consequences worth stating rather than discovering.
//!
//! *It needs a search engine.* A description cannot be resolved without one, so on a laptop
//! with no `SEARX_URL` these now fail where they used to run — the refusal says which variable
//! and why, but a demo that dies at the first screen is what this module exists to prevent.
//! That is a real cost of the change and it is named here rather than left to be found.
//!
//! *And the wait is no longer exactly two companies.* It was four minutes because the set was
//! fixed at two; now discovery decides, and the fixture below is what it was checked against.

use serde::{Deserialize, Serialize};

/// One idea a reader can click.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Example {
    /// Stable across changes to the wording, because it ends up in logs and in the command
    /// that checks these.
    pub id: String,
    /// The idea in the words somebody would use. **This is the whole of the prompt.**
    pub idea: String,
    /// The companies this idea was checked against — a fixture, never an input.
    ///
    /// **Not sent to the browser and not put in the box.** These are what
    /// `landscape examples --check` fetches to prove discovery finds pages worth reading, and
    /// what a reader of that output compares live discovery against. Putting them in the prompt
    /// is what made every example a named set.
    ///
    /// Discovery follows a redirect to `www.`, which was checked for each of these rather
    /// than assumed: `helpscout.com` and `simpleanalytics.com` both serve from `www.` and both
    /// resolve from the bare name.
    pub companies: Vec<String>,
    /// One line saying why these two were the pair worth checking against.
    ///
    /// Operator-facing, like `companies`: it explains a curation decision, and the reader of
    /// the product is no longer shown a curated set to explain.
    pub why: String,
}

impl Example {
    /// The text this puts in the box: the idea, and nothing else.
    ///
    /// **A function rather than reading `idea` at each call site**, because what an example
    /// *is* has now changed twice and both times every caller had to agree about it.
    #[must_use]
    pub fn prompt(&self) -> String {
        self.idea.clone()
    }
}

/// The ideas the demo offers.
///
/// **Every domain here was run through `landscape examples --check`** — real discovery against
/// the real sites — and every one admitted a pricing page and pages for most of the other
/// questions. That is the only property being promised, and it is the one that decides whether
/// a demo produces a report or a page of coverage notes.
///
/// Two companies each. See the module documentation for why not three.
#[must_use]
pub fn examples() -> Vec<Example> {
    vec![
        Example {
            id: "project-management".to_owned(),
            idea: "project management for a small design agency".to_owned(),
            companies: vec!["basecamp.com".to_owned(), "linear.app".to_owned()],
            why: "The two ends of the same market: one sells calm and a flat price, the other \
                  sells speed and prices per seat."
                .to_owned(),
        },
        Example {
            id: "website-analytics".to_owned(),
            idea: "privacy-friendly website analytics".to_owned(),
            companies: vec!["usefathom.com".to_owned(), "simpleanalytics.com".to_owned()],
            why: "Both sell the same promise - no cookies, no consent banner - so the \
                  comparison is entirely about price and what each one counts."
                .to_owned(),
        },
        Example {
            id: "shared-inbox".to_owned(),
            idea: "a shared inbox for a small support team".to_owned(),
            companies: vec!["helpscout.com".to_owned(), "front.com".to_owned()],
            why: "One is built for support and one for any team's email, which is exactly the \
                  distinction a buyer is trying to make."
                .to_owned(),
        },
    ]
}

/// What the interface says about them, in one sentence.
///
/// Here rather than in the web app because it is a claim about the product, and a claim about
/// the product that lives only in a component is one nobody reviews.
pub const CURATION_NOTE: &str =
    "These ideas were chosen by hand. The companies are not: choosing one and pressing \
     Analyze searches for them, and everything the report says is fetched, quoted and \
     cited at that moment - nothing here is stored or written in advance.";

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn no_example_puts_a_company_in_the_box() {
        // **The inversion of what this used to assert**, and the whole of the change. A prompt
        // naming domains is a *named set*: nothing is discovered, and the first screen of the
        // product demonstrates the one path where its central feature does not run.
        for example in examples() {
            let prompt = example.prompt();
            for company in &example.companies {
                assert!(
                    !prompt.contains(company.as_str()),
                    "{} puts {company} in the box: {prompt:?}",
                    example.id
                );
            }
            assert!(
                !prompt.contains('.'),
                "{} reads like a domain, so it would be parsed as one: {prompt:?}",
                example.id
            );
        }
    }

    #[test]
    fn every_prompt_is_the_idea_and_nothing_else() {
        // A second reading of the same rule, and not a duplicate: the one above forbids the
        // companies, this one forbids everything else too. An example that grew a suffix
        // would pass the first and fail here.
        for example in examples() {
            assert_eq!(example.prompt(), example.idea, "{}", example.id);
        }
    }

    #[test]
    fn every_example_was_checked_against_a_comparison() {
        // Still two, and still for the same reason — but now about the **fixture** rather than
        // the prompt. An idea checked against one company proves discovery can find one
        // company, which is not the thing the product is for.
        for example in examples() {
            assert!(
                example.companies.len() >= 2,
                "{} was checked against {} company - that is a profile, not a comparison",
                example.id,
                example.companies.len()
            );
        }
    }

    #[test]
    fn no_fixture_asks_for_a_check_nobody_will_sit_through() {
        // Two minutes a company, measured, and `--check` fetches every one of them.
        //
        // **This no longer bounds what a reader waits for**, and saying so is the point: a
        // description is resolved by discovery, so the set is whatever the engine returns and
        // the analyzer's own cap is what holds it. The bound that used to live here moved,
        // and a test still claiming it would be describing a feature that changed.
        for example in examples() {
            assert!(
                example.companies.len() <= 2,
                "{} would make --check fetch {} companies, about {} minutes",
                example.id,
                example.companies.len(),
                example.companies.len() * 2
            );
        }
    }

    #[test]
    fn ids_are_unique_and_url_safe() {
        let mut seen: Vec<String> = examples().into_iter().map(|e| e.id).collect();
        seen.sort();
        let before = seen.len();
        seen.dedup();
        assert_eq!(before, seen.len(), "two examples share an id");
        for example in examples() {
            assert!(
                example
                    .id
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
                "{:?} is not usable in a URL",
                example.id
            );
        }
    }

    #[test]
    fn no_company_appears_in_two_examples() {
        // Not a correctness rule - a check rule. Two ideas verified against the same domain
        // are one measurement reported twice.
        let all: Vec<String> = examples().into_iter().flat_map(|e| e.companies).collect();
        let mut sorted = all.clone();
        sorted.sort();
        let before = sorted.len();
        sorted.dedup();
        assert_eq!(before, sorted.len(), "a company appears twice: {all:?}");
    }

    #[test]
    fn every_prompt_is_long_enough_to_be_accepted() {
        // The box has a minimum, and an example the API would reject is an example that fails
        // in front of the person it was written for.
        for example in examples() {
            crate::NewAnalysis::parse(&example.prompt()).unwrap_or_else(|e| {
                panic!("{} produces a prompt the API refuses: {e}", example.id)
            });
        }
    }

    #[test]
    fn the_curation_note_says_what_is_curated_and_what_is_not() {
        // A sentence that said only "chosen by hand" would leave a reader assuming the answers
        // were too, which is the misreading this feature has to prevent.
        assert!(CURATION_NOTE.contains("chosen by hand"), "{CURATION_NOTE}");
        assert!(CURATION_NOTE.contains("cited"), "{CURATION_NOTE}");
        // **And it must no longer claim the companies are curated**, because they are not.
        // A sentence left over from the previous design would be this product describing a
        // curation it stopped doing.
        assert!(
            CURATION_NOTE.contains("companies are not"),
            "the note still says the companies are chosen by hand: {CURATION_NOTE}"
        );
        // **And it must describe the interaction that actually happens.** A chip fills the box
        // and deliberately does not run — `App.test.tsx` asserts no POST is sent — so a note
        // reading *"clicking one searches for them"* promises four minutes of somebody else's
        // electricity from a click meant to read a label. Review caught it; it had been written
        // in the same change that made the note true about everything else.
        assert!(
            CURATION_NOTE.contains("pressing Analyze"),
            "the note says a click searches, and a click does not: {CURATION_NOTE}"
        );
        assert!(
            !CURATION_NOTE.contains("clicking one searches"),
            "{CURATION_NOTE}"
        );
    }
}
