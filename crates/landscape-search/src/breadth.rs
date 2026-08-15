//! Whether a question has one answer or several, decided from the market's own writing.
//!
//! `IMPROVING_PRODUCT_IDEAS_LOGIC_ROADMAP.md` PR 7, and the rest of cause 1 in
//! `PRODUCT_IDEA_RESULTS_LOGIC.md` §4.
//!
//! # The question this answers
//!
//! A reader typed *"project management software"*. That is a real question with several real
//! answers — for agencies, for software teams, for construction — and a single ranked list of
//! five vendors is a confident answer to a question nobody asked. A general web answer handles
//! this by offering the subcategories; the reader who reported the original failure said so.
//!
//! # Where the categories come from
//!
//! **The headings of the pages already fetched.** [`crate::literature`] reads the buyer's guides
//! a search returned, and a guide to a broad market is organized by category: *Best for creative
//! agencies*, *Best for software teams*. A heading is structure **the page wrote**, in the same
//! sense a link is something the page did — no model, no summarizing, nothing asserted that was
//! not read.
//!
//! **And it costs nothing.** These are the same four pages `literature` already fetched for the
//! same run; this module reads them again from memory rather than asking anybody's server twice.
//!
//! # The two decisions, and both thresholds are hypotheses
//!
//! ```text
//! a category is OFFERED when
//!     it is named on >= NAMED_BY_HOSTS independent hosts
//!     and it has >= 1 candidate company under it
//!
//! the question is TOO GENERAL when
//!     >= 2 categories clear that bar
//!     and no single category holds more than CONCENTRATION of the candidates
//! ```
//!
//! **Labeled as starting values, exactly as [`landscape_core::subject::AMBIGUITY_MARGIN`] is.**
//! The roadmap says this row is a research problem and should be measured hardest; the numbers
//! below are where that measuring starts, not where it ended.

use std::collections::HashMap;

use crate::literature::{Endorsement, Reading};

/// How many independent publishers must name a category before it is offered to a reader.
///
/// **[`crate::candidates::CORROBORATION`], for the same reason [`crate::literature`] uses it for
/// companies.** One guide's table of contents is one editor's opinion about how a market
/// divides; two unrelated guides agreeing is the market dividing that way.
pub const NAMED_BY_HOSTS: usize = crate::candidates::CORROBORATION;

/// How much of the candidate set one category may hold before the question counts as answered.
///
/// **Sixty percent, and it is a hypothesis.** The claim is narrow: if most of the companies
/// found sit under one heading, a reader asking about that market got the market they asked
/// about, and offering to narrow it is interrogation rather than help — `PRODUCT_SPEC.md` §3's
/// *never ask something inferable from the input*. Where exactly "most" sits is a number nobody
/// has measured, and it is labeled here so the next person moves it deliberately.
pub const CONCENTRATION: f32 = 0.60;

/// One way a market divides, as the guides describe it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Category {
    /// The heading, as written. Shown to a reader, so it is the page's words rather than ours.
    pub label: String,
    /// The publishers whose guides use this heading, sorted and without repeats.
    pub named_by: Vec<String>,
    /// The companies linked under it, by registrable host.
    pub companies: Vec<String>,
}

/// A heading, normalized enough that two guides can be seen to agree.
///
/// **Lowercased, and stripped of the words every guide's headings carry.** *"Best for creative
/// agencies"* and *"For creative agencies"* are one category described twice, and a comparison
/// that missed that would need three guides to agree before two of them did.
///
/// It is deliberately not more than this. Stemming, synonyms and *for agencies ≈ for design
/// teams* are judgments about meaning, and a judgment about meaning made by a program with no
/// model behind it is a guess wearing a rule's clothes.
#[must_use]
pub fn as_category(heading: &str) -> String {
    let lowered = heading.trim().trim_start_matches('#').trim().to_lowercase();
    let mut words: Vec<&str> = lowered.split_whitespace().collect();
    while let Some(first) = words.first() {
        if matches!(*first, "best" | "top" | "the" | "our") {
            words.remove(0);
        } else {
            break;
        }
    }
    words.join(" ")
}

/// Split a guide into its sections: each heading, and the links that follow it.
///
/// **The links under a heading, not the links on the page.** A guide's first heading is its
/// title and everything on the page follows it; what makes *Best for creative agencies* a
/// category with companies in it is the three links between that heading and the next one.
///
/// The page's own title — the first `#` — is not a category. It is the market, which is the
/// thing being divided.
#[must_use]
pub fn sections(page: &str, publisher: &str) -> Vec<(String, Vec<Endorsement>)> {
    let mut out: Vec<(String, Vec<Endorsement>)> = Vec::new();
    let mut heading: Option<String> = None;
    let mut body = String::new();

    let mut flush = |heading: &mut Option<String>, body: &mut String| {
        if let Some(label) = heading.take() {
            out.push((label, crate::literature::linked_from(body, publisher)));
        }
        body.clear();
    };

    for line in page.lines() {
        let hashes = line.len() - line.trim_start_matches('#').len();
        if hashes >= 2 && line.starts_with('#') {
            flush(&mut heading, &mut body);
            heading = Some(line.trim_start_matches('#').trim().to_owned());
        } else if line.starts_with('#') {
            // The page's own title. Everything before the first real heading belongs to nothing.
            flush(&mut heading, &mut body);
        } else {
            body.push_str(line);
            body.push('\n');
        }
    }
    flush(&mut heading, &mut body);
    out
}

/// The categories the guides agree on, best supported first.
///
/// A category needs [`NAMED_BY_HOSTS`] independent publishers **and** at least one company under
/// it. A heading two guides share with nothing beneath it is a section of prose, not a market.
#[must_use]
pub fn categories(read: &HashMap<String, String>) -> Vec<Category> {
    // label -> (publishers, companies)
    let mut by_label: HashMap<String, (Vec<String>, Vec<String>)> = HashMap::new();
    // Sorted, so the categories a reader is offered do not depend on how a map iterated.
    let mut pages: Vec<(&String, &String)> = read.iter().collect();
    pages.sort();

    for (publisher, page) in pages {
        for (heading, linked) in sections(page, publisher) {
            let label = as_category(&heading);
            if label.is_empty() {
                continue;
            }
            let entry = by_label.entry(label).or_default();
            if !entry.0.contains(publisher) {
                entry.0.push(publisher.clone());
            }
            for one in linked {
                if !entry.1.contains(&one.host) {
                    entry.1.push(one.host);
                }
            }
        }
    }

    let mut out: Vec<Category> = by_label
        .into_iter()
        .filter(|(_, (by, companies))| by.len() >= NAMED_BY_HOSTS && !companies.is_empty())
        .map(|(label, (named_by, companies))| Category {
            label,
            named_by,
            companies,
        })
        .collect();
    out.sort_by(|a, b| {
        b.named_by
            .len()
            .cmp(&a.named_by.len())
            .then_with(|| b.companies.len().cmp(&a.companies.len()))
            .then_with(|| a.label.cmp(&b.label))
    });
    out
}

/// The categories in a run's literature, read from the pages it already fetched.
#[must_use]
pub fn of(reading: &Reading) -> Vec<Category> {
    categories(&reading.pages)
}

/// Whether the question covers several markets rather than one.
///
/// **Two conditions, and the second is the one doing the work.** Any broad market has
/// subheadings; what makes a question *too general* is that the companies found are spread
/// across them rather than sitting under one. A reader who asked about project management for
/// agencies and got mostly agency tools was answered.
///
/// `candidates` is how many companies the run found in total, which is the denominator
/// [`CONCENTRATION`] is a fraction of.
#[must_use]
pub fn too_general(categories: &[Category], candidates: usize) -> bool {
    if categories.len() < 2 || candidates == 0 {
        return false;
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "counts here are single digits; NAMED caps the candidates at five"
    )]
    let share = |n: usize| n as f32 / candidates as f32;
    !categories
        .iter()
        .any(|c| share(c.companies.len()) > CONCENTRATION)
}

/// The categories as something a reader can click.
///
/// **The same shape the ambiguous-company question already uses**, and for the reason
/// [`crate::vocabulary::choices_from`] gives: the interface has one way of asking *which did you
/// mean* rather than three. `domain` is empty because a category has no website, and what stands
/// in its place is the evidence there is for it — how many guides named it, and how many
/// companies they put under it.
///
/// **Each chip carries a whole prompt**, so a click is a new run with its own URL rather than a
/// word to be typed back into a sentence. The reader's own words come first and the category
/// follows, unless the category already contains them.
#[must_use]
pub fn choices_from(between: &[Category], asked: &str) -> Vec<landscape_core::Choice> {
    between
        .iter()
        .map(|c| landscape_core::Choice {
            name: c.label.clone(),
            domain: String::new(),
            what_it_is: format!(
                "{} guides list {} {} under this",
                c.named_by.len(),
                c.companies.len(),
                if c.companies.len() == 1 {
                    "company"
                } else {
                    "companies"
                }
            ),
            prompt: if c.label.contains(&asked.to_lowercase()) {
                c.label.clone()
            } else {
                format!("{asked} {}", c.label)
            },
        })
        .collect()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    const G2: &str = "# Best Project Management Software\n\n\
        Our picks, by the kind of team you run.\n\n\
        ## Best for creative agencies\n\n\
        [Workamajig](https://www.workamajig.com/) and [Asana](https://asana.com/).\n\n\
        ## Best for software teams\n\n\
        [Linear](https://linear.app/) and [Jira](https://www.atlassian.com/software/jira).\n";

    const CAPTERRA: &str = "# Project Management Software\n\n\
        ## For creative agencies\n\n\
        [Workamajig](https://www.workamajig.com/).\n\n\
        ## For software teams\n\n\
        [Linear](https://linear.app/).\n\n\
        ## Methodology\n\n\
        How we rank.\n";

    fn read(of: &[(&str, &str)]) -> HashMap<String, String> {
        of.iter()
            .map(|(host, page)| ((*host).to_owned(), (*page).to_owned()))
            .collect()
    }

    #[test]
    fn a_heading_is_the_pages_own_word_for_a_category() {
        // Two guides describing one category two ways. Stripping the words every guide's
        // headings carry is the whole of the normalizing: anything more is a judgment about
        // meaning, made by a program with nothing behind it.
        assert_eq!(
            as_category("## Best for creative agencies"),
            "for creative agencies"
        );
        assert_eq!(
            as_category("For creative agencies"),
            "for creative agencies"
        );
        assert_eq!(
            as_category("# Best Project Management Software"),
            "project management software"
        );
    }

    #[test]
    fn the_companies_under_a_heading_are_the_ones_between_it_and_the_next() {
        let found = sections(G2, "g2.com");
        let labels: Vec<&str> = found.iter().map(|(h, _)| h.as_str()).collect();
        assert_eq!(
            labels,
            vec!["Best for creative agencies", "Best for software teams"],
            "the page's own title is the market, not a category"
        );
        let agencies: Vec<&str> = found[0].1.iter().map(|e| e.host.as_str()).collect();
        assert_eq!(agencies, vec!["workamajig.com", "asana.com"]);
        let software: Vec<&str> = found[1].1.iter().map(|e| e.host.as_str()).collect();
        assert_eq!(software, vec!["linear.app", "atlassian.com"]);
    }

    #[test]
    fn a_category_needs_two_publishers_and_somebody_under_it() {
        let found = categories(&read(&[("g2.com", G2), ("capterra.com", CAPTERRA)]));
        let labels: Vec<&str> = found.iter().map(|c| c.label.as_str()).collect();
        assert_eq!(
            labels,
            vec!["for creative agencies", "for software teams"],
            "and *methodology* is one publisher's section, not a market: {found:#?}"
        );
        assert_eq!(found[0].named_by, vec!["capterra.com", "g2.com"]);
        assert_eq!(found[0].companies, vec!["workamajig.com", "asana.com"]);
    }

    #[test]
    fn one_guide_dividing_a_market_is_one_editors_opinion() {
        let found = categories(&read(&[("g2.com", G2)]));
        assert!(found.is_empty(), "{found:#?}");
    }

    #[test]
    fn a_heading_with_nobody_under_it_is_a_section_of_prose() {
        let empty = "# A market\n\n## For agencies\n\nWe could not find any.\n";
        let found = categories(&read(&[("g2.com", empty), ("capterra.com", empty)]));
        assert!(found.is_empty(), "{found:#?}");
    }

    #[test]
    fn a_question_whose_answers_sit_under_one_heading_was_answered() {
        // **The condition doing the work.** Any broad market has subheadings; what makes a
        // question too general is the companies being spread across them.
        let concentrated = vec![
            Category {
                label: "for creative agencies".to_owned(),
                named_by: vec!["capterra.com".to_owned(), "g2.com".to_owned()],
                companies: vec![
                    "a.example".to_owned(),
                    "b.example".to_owned(),
                    "c.example".to_owned(),
                ],
            },
            Category {
                label: "for software teams".to_owned(),
                named_by: vec!["capterra.com".to_owned(), "g2.com".to_owned()],
                companies: vec!["d.example".to_owned()],
            },
        ];
        assert!(
            !too_general(&concentrated, 4),
            "three of four under one heading is an answer, not a question"
        );

        let spread = vec![
            Category {
                label: "for creative agencies".to_owned(),
                named_by: vec!["capterra.com".to_owned(), "g2.com".to_owned()],
                companies: vec!["a.example".to_owned(), "b.example".to_owned()],
            },
            Category {
                label: "for software teams".to_owned(),
                named_by: vec!["capterra.com".to_owned(), "g2.com".to_owned()],
                companies: vec!["c.example".to_owned(), "d.example".to_owned()],
            },
        ];
        assert!(too_general(&spread, 4), "half and half is two markets");
    }

    #[test]
    fn one_category_is_never_a_question() {
        let only = vec![Category {
            label: "for creative agencies".to_owned(),
            named_by: vec!["capterra.com".to_owned(), "g2.com".to_owned()],
            companies: vec!["a.example".to_owned()],
        }];
        assert!(!too_general(&only, 4), "there is nothing to choose between");
        assert!(!too_general(&[], 4));
        // And a run that found nobody has no denominator, so there is no share to compare.
        assert!(!too_general(&only, 0));
    }
}
