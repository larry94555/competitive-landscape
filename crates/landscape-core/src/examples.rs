//! Ideas somebody can click, over companies that really answer.
//!
//! `ROADMAP.md` §2·D item D4. A demo you send someone fails at the first screen if the first
//! screen is an empty box: a reader who has never used this does not know what a good prompt
//! looks like, and the prompts they invent are the ones this pipeline cannot resolve —
//! *"a tool for small farms"* names no site, so the run fails with a reason and the demo is
//! over before anything was read.
//!
//! # What is curated, and what is not
//!
//! **Only the choice of companies.** Everything a report says is fetched, quoted and cited at
//! the moment somebody clicks — there are no stored answers here, no cached reports, and no
//! text that ends up in a claim. What this file contains is a list of domains that were checked
//! by hand, once, for a property the pipeline cannot promise for an arbitrary site: that
//! discovery finds pages worth reading.
//!
//! That distinction is the whole honesty of the feature, so [`Example::prompt`] puts the
//! companies **into the text the reader sees and can edit**. Nothing is expanded behind their
//! back: clicking a chip types a sentence they could have typed themselves, and deleting a
//! company from it analyses one company.
//!
//! # Why this is data in `landscape-core` rather than a list in the web app
//!
//! Two copies of a curated list drift, and the copy that drifts is the one nobody tests. Here
//! it is reachable by the API that serves it, by the command that checks it, and by the test in
//! `landscape-analyze` that runs each prompt through **the real prompt parser** and asserts the
//! companies come back out.
//!
//! # The wait these imply
//!
//! Two companies each, not three. Each company is its own discovery, fetches and model calls —
//! about two minutes a company on the laptop this was measured on, so an example is about four.
//! Three companies would be six, which is past what `PRODUCT_SPEC.md` §2.1A asks for and well
//! past what somebody clicking a link will sit through. Three is what the analyser *allows*;
//! two is what a demo can spend.

use serde::{Deserialize, Serialize};

/// One idea a reader can click, and the companies it compares.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Example {
    /// Stable across changes to the wording, because it ends up in logs and in the command
    /// that checks these.
    pub id: String,
    /// The idea in the words somebody would use, without the companies.
    pub idea: String,
    /// The companies, as a person would type them — bare domains, no scheme.
    ///
    /// Discovery follows a redirect to `www.`, which was checked for each of these rather
    /// than assumed: `helpscout.com` and `simpleanalytics.com` both serve from `www.` and both
    /// resolve from the bare name.
    pub companies: Vec<String>,
    /// One line saying why these two are a comparison rather than two arbitrary sites.
    pub why: String,
}

impl Example {
    /// The text this puts in the box.
    ///
    /// **The companies are in it.** A chip that filled the box with the idea alone and passed
    /// the domains separately would be hiding the curated part in the place a reader is least
    /// able to see it, which is the opposite of what this feature is for.
    #[must_use]
    pub fn prompt(&self) -> String {
        format!("{} - {}", self.idea, self.companies.join(" vs "))
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
    "The companies in these examples were chosen by hand. Everything the report says about them \
     is fetched, quoted and cited when you click - nothing here is stored or written in advance.";

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn every_example_puts_its_companies_in_the_text_a_reader_can_see() {
        // The honesty of the feature, as an assertion. A chip that expanded to companies the
        // reader never saw would be curating in the one place they cannot check.
        for example in examples() {
            let prompt = example.prompt();
            for company in &example.companies {
                assert!(
                    prompt.contains(company.as_str()),
                    "{} is not in the prompt {prompt:?}",
                    example.id
                );
            }
        }
    }

    #[test]
    fn every_example_is_a_comparison() {
        // One company is a profile. The whole reason these exist is to show the thing the
        // product is for, and a single-company example would demonstrate the old defect.
        for example in examples() {
            assert!(
                example.companies.len() >= 2,
                "{} names {} company - that is a profile, not a comparison",
                example.id,
                example.companies.len()
            );
        }
    }

    #[test]
    fn no_example_asks_for_a_wait_nobody_will_sit_through() {
        // Two minutes a company, measured. Three companies is six minutes, and a demo that
        // takes six minutes is not a demo - `ROADMAP.md` §2·D says so in the risk section.
        for example in examples() {
            assert!(
                example.companies.len() <= 2,
                "{} names {} companies, which is about {} minutes",
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
        // Not a correctness rule - a demo rule. Clicking the second chip and recognising the
        // first one's companies makes three examples look like one.
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
    }
}
