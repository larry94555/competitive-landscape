//! What was checked when nothing was found.
//!
//! `PRODUCT_SPEC.md` §4 shows the output this exists to produce:
//!
//! > Coverage note: no public changelog found for Shortcut in this window — /changelog (404),
//! > /releases (404), blog (90d). **Not "no changes."**
//!
//! and `FACT_CHECKING.md` §5.4 gives the rule behind it: *a negative nobody can check is not a
//! finding*. An empty section and a section nobody tried to fill look identical in a report
//! that only prints what it found, and the difference between them is the whole product.
//!
//! # Why this is a type rather than a sentence
//!
//! Because the same words have to appear in three places — the CLI, the report, and an alert
//! that fires on a change — and a note assembled separately in each will drift. The
//! [`Coverage`] is built once from what discovery recorded, and every surface renders it.

use serde::{Deserialize, Serialize};

use crate::report::{Section, SectionStatus};

/// One path we tried and what came back, as a coverage note repeats it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attempt {
    /// The path, not the whole URL — `/changelog (404)` is what a reader can retype, and on a
    /// report about one company the heading above it already says whose it is.
    pub path: String,
    /// `404`, `robots`, `200`. Short enough to sit in brackets.
    pub outcome: String,
    /// Which company was tried, as the origin.
    ///
    /// **The heading stopped naming the company** once one section could hold several of them:
    /// two companies each contribute `/pricing (404)`, and merged they are indistinguishable.
    /// Shown only when a report covers more than one, for the same reason a claim's subject is.
    #[serde(default)]
    pub subject: String,
}

/// An origin without its scheme, which is how a person names a company.
fn bare(origin: &str) -> String {
    let host = origin
        .trim_start_matches("https://")
        .trim_start_matches("http://");
    format!("{host} ")
}

/// What one question of the six is backed by.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Coverage {
    /// `pricing`, `changes`, and the rest.
    pub question: String,
    /// Pages admitted for this question.
    pub sources: Vec<String>,
    /// How many of those pages were actually read.
    ///
    /// Separate from `sources.len()` because they come apart, and the gap between them is a
    /// finding of its own: four of the six questions have no extractor yet, so their pages are
    /// admitted and never opened. A note saying *"read one page, it stated nothing"* about a
    /// page nobody opened would be **this feature committing the error it exists to prevent**.
    pub pages_read: usize,
    /// Facts extracted from them — plans, capabilities, dated changes.
    pub facts: usize,
    /// Every path tried whose shape suggested this question.
    pub attempts: Vec<Attempt>,
}

impl Coverage {
    /// Whether this question has nothing behind it.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.facts == 0
    }

    /// The note a reader sees where facts would otherwise be.
    ///
    /// Four different silences, and no two of them are the same finding:
    ///
    /// | | |
    /// |---|---|
    /// | Nothing tried | our gap — say so plainly |
    /// | Tried, nothing answered | **the company publishes none of this** |
    /// | Found but never opened | our gap again, and a different one |
    /// | Read, nothing extracted | the page exists and did not say |
    #[must_use]
    pub fn note(&self) -> String {
        if !self.is_empty() {
            return format!(
                "{} fact(s) from {} source(s)",
                self.facts,
                self.sources.len()
            );
        }
        if self.attempts.is_empty() && self.sources.is_empty() {
            return "nothing was checked - our gap, not theirs".to_owned();
        }
        if self.sources.is_empty() {
            return format!("no page found. Checked: {}", self.render_attempts());
        }
        if self.pages_read == 0 {
            return format!(
                "{} page(s) found and not read - our gap, not theirs",
                self.sources.len()
            );
        }
        format!(
            "read {} page(s), none stated anything. Checked: {}",
            self.pages_read,
            self.render_attempts()
        )
    }

    /// `/changelog (404), /releases (404), /blog (200)`.
    /// Whether these attempts span more than one company.
    ///
    /// Derived once and read in both places that render an attempt — two expressions computing
    /// it from different inputs is entry 4 of the mistakes register.
    fn covers_several(&self) -> bool {
        let mut seen: Vec<&str> = self
            .attempts
            .iter()
            .map(|a| a.subject.as_str())
            .filter(|s| !s.is_empty())
            .collect();
        seen.sort_unstable();
        seen.dedup();
        seen.len() > 1
    }

    fn render_attempts(&self) -> String {
        /// Enough to be convincing; short enough to read. The rest are counted.
        const SHOWN: usize = 6;

        let several = self.covers_several();
        let mut parts: Vec<String> = self
            .attempts
            .iter()
            .take(SHOWN)
            .map(|a| {
                if several && !a.subject.is_empty() {
                    format!("{}{} ({})", bare(&a.subject), a.path, a.outcome)
                } else {
                    format!("{} ({})", a.path, a.outcome)
                }
            })
            .collect();
        if self.attempts.len() > SHOWN {
            parts.push(format!("and {} more", self.attempts.len() - SHOWN));
        }
        if parts.is_empty() {
            return "nothing".to_owned();
        }
        parts.join(", ")
    }

    /// The report section this coverage produces when the question came back empty.
    ///
    /// Uses [`Section::not_found`], so the CLI's words and the report's words cannot drift
    /// apart — which is the reason this lives in `landscape-core` rather than beside the
    /// command that prints it.
    #[must_use]
    pub fn to_section(&self, title: impl Into<String>) -> Section {
        // Two companies each tried `/pricing`, and a list of four identical-looking paths tells
        // a reader nothing about which company found nothing. The name goes on only when there
        // is more than one, because on a single-company report it is on the heading already.
        let several = self.covers_several();
        let checked = self
            .attempts
            .iter()
            .map(|a| {
                if several && !a.subject.is_empty() {
                    format!("{}{} ({})", bare(&a.subject), a.path, a.outcome)
                } else {
                    format!("{} ({})", a.path, a.outcome)
                }
            })
            .collect();
        let mut section = Section::not_found(self.question.clone(), title, checked);
        if !self.is_empty() {
            section.status = SectionStatus::Populated;
        }
        section
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn attempt(path: &str, outcome: &str) -> Attempt {
        Attempt {
            subject: String::new(),
            path: path.to_owned(),
            outcome: outcome.to_owned(),
        }
    }

    fn empty(question: &str, attempts: Vec<Attempt>, sources: Vec<String>) -> Coverage {
        let pages_read = sources.len();
        Coverage {
            question: question.to_owned(),
            sources,
            pages_read,
            facts: 0,
            attempts,
        }
    }

    #[test]
    fn a_question_nobody_tried_is_our_gap_not_theirs() {
        // The distinction that makes an empty section honest. A report saying "no changelog
        // found" when nothing was requested is a lie told by omission.
        let note = empty("changes", vec![], vec![]).note();
        assert!(note.contains("nothing was checked"), "{note}");
    }

    #[test]
    fn a_question_that_was_tried_names_the_paths_and_the_answers() {
        // PRODUCT_SPEC §4's coverage note, near enough word for word.
        let note = empty(
            "changes",
            vec![attempt("/changelog", "404"), attempt("/releases", "404")],
            vec![],
        )
        .note();
        assert!(note.contains("/changelog (404)"), "{note}");
        assert!(note.contains("/releases (404)"), "{note}");
    }

    #[test]
    fn a_page_that_was_read_and_said_nothing_is_a_third_thing() {
        // Not "no page", and not "not checked". The page exists and does not state the fact,
        // which is the strongest of the three findings and the easiest to misreport.
        let note = empty(
            "changes",
            vec![attempt("/releases", "200")],
            vec!["https://e.com/releases".to_owned()],
        )
        .note();
        assert!(
            note.contains("read 1 page(s), none stated anything"),
            "{note}"
        );
    }

    #[test]
    fn a_page_found_and_never_opened_is_our_gap_too() {
        // Four of the six questions have no extractor yet. Saying "we read it and it said
        // nothing" about a page nobody opened would be this feature committing the exact
        // error it exists to prevent.
        let coverage = Coverage {
            question: "trust".to_owned(),
            sources: vec!["https://e.com/security".to_owned()],
            pages_read: 0,
            facts: 0,
            attempts: vec![attempt("/security", "200")],
        };
        let note = coverage.note();
        assert!(note.contains("found and not read"), "{note}");
        assert!(note.contains("our gap"), "{note}");
    }

    #[test]
    fn a_question_with_facts_reports_them_instead() {
        let covered = Coverage {
            question: "pricing".to_owned(),
            sources: vec!["https://e.com/pricing".to_owned()],
            pages_read: 1,
            facts: 3,
            attempts: vec![attempt("/pricing", "200")],
        };
        assert!(!covered.is_empty());
        assert_eq!(covered.note(), "3 fact(s) from 1 source(s)");
    }

    #[test]
    fn a_long_list_of_attempts_is_summarised_rather_than_dumped() {
        let attempts: Vec<Attempt> = (0..12).map(|n| attempt(&format!("/p{n}"), "404")).collect();
        let note = empty("trust", attempts, vec![]).note();
        assert!(note.contains("and 6 more"), "{note}");
    }

    #[test]
    fn the_note_and_the_report_section_say_the_same_thing() {
        // The reason this is a type. Two surfaces, one set of words.
        let coverage = empty("changes", vec![attempt("/changelog", "404")], vec![]);
        let section = coverage.to_section("Recent public changes");
        assert_eq!(section.status, SectionStatus::NotFoundInPublicSources);
        assert_eq!(section.checked, ["/changelog (404)"]);
        assert!(coverage.note().contains("/changelog (404)"));
    }

    #[test]
    fn a_section_with_facts_is_not_marked_not_found() {
        let coverage = Coverage {
            question: "pricing".to_owned(),
            sources: vec!["https://e.com/pricing".to_owned()],
            pages_read: 1,
            facts: 2,
            attempts: vec![],
        };
        assert_eq!(
            coverage.to_section("Pricing & packaging").status,
            SectionStatus::Populated
        );
    }
}
