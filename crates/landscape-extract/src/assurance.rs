//! What a trust page says it holds — and what it says it is only working towards.
//!
//! The fifth question kind, and the first whose vocabulary is **closed**. A capability can be
//! called anything; a price can be any number. But a company does not invent a compliance
//! standard — it names one of a few dozen that already exist, and it spells them the way the
//! auditors do.
//!
//! That changes the shape of the extractor, and for the better:
//!
//! ```text
//! SOC 2 Type II                          found by the scanner, not by a model
//! …report available on request           read by a model: do they have it?
//! ```
//!
//! **Finding the mention is arithmetic. Reading the claim is not.** A page saying *"SOC 2 Type
//! II report available on request"* and one saying *"SOC 2 is on our roadmap for 2027"* contain
//! the same words, and a report that treated them alike would be the wrong answer this project
//! keeps finding: correct-looking, fully cited, and about a different fact.
//!
//! So the scanner locates every standard the page names — deterministically, from the list
//! below — and the model is asked one small question about each: *is this something they have,
//! or something they are working towards?*
//!
//! # Why a closed list rather than "any capitalised acronym"
//!
//! Because the alternative fabricates. A model asked to *"list the certifications on this
//! page"* will happily return SOC 2 for a page that never mentions it, and the grounding check
//! would then be the only thing between that and a report. Here the name comes from the page by
//! construction: nothing can be reported that the scanner did not first find written down.
//!
//! Adding a standard is one line, and it is a decision about what this product claims to know
//! rather than a tuning parameter — which is why the list is here and named rather than a
//! regular expression somewhere.

use crate::span::{Span, WINDOW_CHARS};

/// Bumped whenever these rules change.
pub const ASSURANCE_VERSION: u32 = 1;

/// How many assurances one page is worth reading.
///
/// A security page can list a dozen frameworks and the first few are the ones a buyer is
/// choosing on. The number is stated wherever it bites — a short list with nothing beside it is
/// a wrong list, which is the same rule the capability cap follows.
pub const MAX_ASSURANCES: usize = 8;

/// The standards this product recognises, longest spelling first.
///
/// **Order matters.** `ISO 27001` must be tried before `ISO 27` would be, and `SOC 2 Type II`
/// before `SOC 2`, or the shorter name wins and the report loses the part a reader cares about.
/// Every entry is a name an organisation publishes about itself; none is a marketing word.
const STANDARDS: [&str; 18] = [
    "SOC 2 Type II",
    "SOC 2 Type 2",
    "SOC 2 Type I",
    "SOC 2",
    "SOC 3",
    "ISO 27017",
    "ISO 27018",
    "ISO 27701",
    "ISO 27001",
    "PCI DSS",
    "FedRAMP",
    "HIPAA",
    "GDPR",
    "CCPA",
    "Cyber Essentials",
    "TISAX",
    "StateRAMP",
    "HITRUST",
];

/// One standard the page names, and the words around it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Named {
    /// Exactly as the page spells it, taken from the page rather than from the list — so
    /// `iso 27001` in running text is reported the way it was written.
    pub standard: String,
    /// The sentence it appears in, with its neighbours.
    pub span: Span,
}

/// What the page offered before [`MAX_ASSURANCES`] was applied.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Found {
    pub named: Vec<Named>,
    /// How many distinct standards the page named in total.
    pub considered: usize,
}

/// Every standard this page names, in the order it names them.
///
/// One entry per standard, not per mention: a page listing SOC 2 in a banner and again in a
/// table has said one thing twice, and reading it twice would cost a model call and produce a
/// duplicate for the assembler to drop.
#[must_use]
pub fn every_assurance(markdown: &str) -> Found {
    let lines: Vec<&str> = markdown.lines().collect();
    if lines.is_empty() {
        return Found::default();
    }

    let mut named: Vec<Named> = Vec::new();
    let mut considered = 0usize;

    for (at, line) in lines.iter().enumerate() {
        for found in standards_in(line) {
            // **The same standard, at two precisions, is one standard.** A real security page
            // says `SOC 2` in a banner and `SOC 2 Type II` in the section that explains it -
            // freezing linear.app/security is what showed this - and reporting both would tell
            // a reader the same certification twice while spending two model calls on it.
            //
            // The more precise spelling wins wherever it appears, because `Type II` is the
            // part a buyer is comparing on. Order of appearance does not decide it.
            if let Some(kept) = named
                .iter_mut()
                .find(|kept| subsumes(&kept.standard, &found))
            {
                if found.len() > kept.standard.len() {
                    kept.standard = found;
                    kept.span = window(&lines, at);
                }
                continue;
            }
            considered += 1;
            if named.len() < MAX_ASSURANCES {
                named.push(Named {
                    standard: found,
                    span: window(&lines, at),
                });
            }
        }
    }

    Found { named, considered }
}

/// Whether two spellings name the same standard.
///
/// `SOC 2` and `SOC 2 Type II` do; `SOC 2` and `SOC 3` do not. Compared case-insensitively,
/// because a page may write `GDPR` in a heading and `gdpr` in a link.
fn subsumes(a: &str, b: &str) -> bool {
    let (a, b) = (a.to_lowercase(), b.to_lowercase());
    a == b || a.starts_with(&format!("{b} ")) || b.starts_with(&format!("{a} "))
}

/// Every standard named on one line, longest spelling first.
///
/// Overlaps are removed: `SOC 2 Type II` and `SOC 2` both match the same words, and reporting
/// both would double-count one claim and spend a model call proving it.
fn standards_in(line: &str) -> Vec<String> {
    let lower = line.to_lowercase();
    let mut found: Vec<String> = Vec::new();
    let mut claimed: Vec<(usize, usize)> = Vec::new();

    for standard in STANDARDS {
        let needle = standard.to_lowercase();
        let mut from = 0usize;
        while let Some(offset) = lower[from..].find(&needle) {
            let start = from + offset;
            let end = start + needle.len();
            from = end;
            // Inside a longer name already taken — `SOC 2` within `SOC 2 Type II`.
            if claimed.iter().any(|(s, e)| start >= *s && end <= *e) {
                continue;
            }
            // A name that runs into a word is not that name: `GDPR` in `gdprxyz`, and more
            // usefully `SOC 2` in `SOC 20`.
            if !bounded(line, start, end) {
                continue;
            }
            claimed.push((start, end));
            found.push(line[start..end].to_owned());
        }
    }
    found
}

/// Whether a match sits on word boundaries.
fn bounded(line: &str, start: usize, end: usize) -> bool {
    let before = line[..start].chars().next_back();
    let after = line[end..].chars().next();
    let open = |c: Option<char>| c.is_none_or(|c| !c.is_alphanumeric());
    open(before) && open(after)
}

/// The line that named it, with the one before and after.
///
/// Wider than the line because the claim is usually in the next sentence — *"SOC 2 Type II. Our
/// report is available under NDA."* — and narrower than the section because a security page's
/// paragraphs are about different standards.
fn window(lines: &[&str], at: usize) -> Span {
    let start = at.saturating_sub(1);
    let end = (at + 1).min(lines.len().saturating_sub(1));
    let mut text = lines[start..=end].join("\n");
    if text.chars().count() > WINDOW_CHARS {
        text = text.chars().take(WINDOW_CHARS).collect();
    }
    Span {
        text,
        starts_at_line: start,
        heading: None,
        score: 0,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn names(markdown: &str) -> Vec<String> {
        every_assurance(markdown)
            .named
            .into_iter()
            .map(|n| n.standard)
            .collect()
    }

    #[test]
    fn a_standard_the_page_names_is_found() {
        assert_eq!(names("We are SOC 2 Type II certified."), ["SOC 2 Type II"]);
    }

    #[test]
    fn the_longer_spelling_wins_over_the_shorter_one() {
        // **The whole point of ordering the list.** `SOC 2` is inside `SOC 2 Type II`, and
        // reporting the short one throws away the part a buyer is comparing on.
        assert_eq!(names("SOC 2 Type II report available"), ["SOC 2 Type II"]);
    }

    #[test]
    fn the_same_standard_twice_is_one_fact() {
        // A banner and a table saying the same thing. Reading it twice costs a model call and
        // produces a duplicate for the assembler to drop.
        let page = "# GDPR\n\nWe are GDPR compliant.\n\n| GDPR | yes |";
        assert_eq!(names(page), ["GDPR"]);
    }

    #[test]
    fn the_same_standard_at_two_precisions_is_one_standard() {
        // Freezing linear.app/security showed this: a banner says `SOC 2` and the section
        // below says `SOC 2 Type II`. Two entries would tell a reader the same certification
        // twice and spend two model calls proving it.
        let page = "We are SOC 2 compliant.

Our SOC 2 Type II report is available.";
        let found = every_assurance(page);
        assert_eq!(
            names(page),
            ["SOC 2 Type II"],
            "the precise spelling should win"
        );
        assert_eq!(found.considered, 1, "it is one standard, named twice");
    }

    #[test]
    fn a_different_standard_with_a_shared_prefix_is_not_folded_in() {
        // `SOC 2` and `SOC 3` are different reports. Subsuming on a shared prefix rather than
        // on a whole word would lose one of them.
        assert_eq!(names("We hold SOC 2 and SOC 3."), ["SOC 2", "SOC 3"]);
    }

    #[test]
    fn a_name_that_runs_into_a_word_is_not_that_name() {
        // `SOC 20` is not `SOC 2`, and a report that said so would be inventing a
        // certification from a substring.
        assert!(names("Our SOC 20 initiative").is_empty());
        assert!(names("see gdprxyz for more").is_empty());
    }

    #[test]
    fn a_page_with_no_standards_offers_nothing_to_read() {
        // The common case on a marketing security page: reassuring words, nothing named. It
        // must produce **no windows at all** rather than one the model is asked to fill.
        let page = "# Security\n\nWe take security seriously and use bank-grade encryption.";
        assert!(every_assurance(page).named.is_empty());
        assert_eq!(every_assurance(page).considered, 0);
    }

    #[test]
    fn the_page_is_read_in_the_order_it_presents_them() {
        let page = "We hold ISO 27001.\n\nAnd SOC 2 Type II.\n\nAnd we follow HIPAA.";
        assert_eq!(names(page), ["ISO 27001", "SOC 2 Type II", "HIPAA"]);
    }

    #[test]
    fn the_window_carries_the_sentence_after_the_name() {
        // Where the claim usually is: the name is a heading, and whether they *have* it is the
        // line below. A window of one line would ask the model to judge a title.
        let page = "SOC 2\nOur Type II report is available under NDA.";
        let found = every_assurance(page);
        assert!(
            found.named[0].span.text.contains("available under NDA"),
            "{:?}",
            found.named[0].span.text
        );
    }

    #[test]
    fn a_long_list_is_capped_and_the_number_is_kept() {
        // A short list with nothing beside it is a wrong list. `considered` is what the
        // report says out loud.
        let page = STANDARDS
            .iter()
            .map(|s| format!("We hold {s}."))
            .collect::<Vec<_>>()
            .join("\n\n");
        let found = every_assurance(&page);
        assert_eq!(found.named.len(), MAX_ASSURANCES);
        assert!(
            found.considered > MAX_ASSURANCES,
            "considered {} is not more than the cap",
            found.considered
        );
    }

    #[test]
    fn the_spelling_reported_is_the_page_s_own() {
        // Taken from the page rather than from the list, so a report quotes the company.
        assert_eq!(names("we are iso 27001 certified"), ["iso 27001"]);
    }

    #[test]
    fn an_empty_page_is_not_a_panic() {
        assert!(every_assurance("").named.is_empty());
    }
}
