//! Who they are, where, and how big — the windows that might say so.
//!
//! The fourth question kind, and the first one whose answers are *entities* rather than names
//! or numbers a page is designed to publish. A pricing page exists to state a price. **An
//! about page exists to tell a story**, and the facts are in it by accident:
//!
//! ```text
//! Today Plausible is a team of 10.                        plausible.io/about
//! …an independent, open source analytics company based in the EU.
//! We're here for them, 23 years and running.              basecamp.com/about
//! ```
//!
//! The last one is the shape of the problem. Basecamp states its age and never its founding
//! year, and *2003* is arithmetic rather than reading. Nothing here computes it: a fact this
//! module reports has to be **written on the page**, and a company that only implies its
//! founding year is a company whose founding year we do not have.
//!
//! # One window per fact, not one per page
//!
//! The three facts sit in different sentences, often paragraphs apart, and asking one small
//! question three times is what has worked since `BENCHMARKS.md` Run 5. So each fact gets its
//! own window, scored for its own vocabulary, and a page that states two of the three produces
//! two windows.

use crate::span::{Span, WINDOW_CHARS};

/// Bumped whenever these rules change.
pub const IDENTITY_VERSION: u32 = 1;

/// The score a line must reach to be worth a model call.
///
/// Ten is the vocabulary of the fact plus something to attach it to — `founded` beside a year,
/// `based in` beside a place. A line with only one half of that is a sentence about something
/// else.
const FLOOR: u32 = 10;

/// One of the three facts an about page might state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fact {
    /// The year it started.
    Founded,
    /// Where it is.
    Headquarters,
    /// How many people work there.
    Employees,
}

impl Fact {
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Founded => "founded",
            Self::Headquarters => "headquarters",
            Self::Employees => "employees",
        }
    }

    /// Every fact, in the order a report presents them.
    #[must_use]
    pub const fn all() -> [Self; 3] {
        [Self::Founded, Self::Headquarters, Self::Employees]
    }
}

/// A window for each fact the page looks like it states.
///
/// At most three, and often none — most about pages are entirely story.
#[must_use]
pub fn every_fact(markdown: &str) -> Vec<(Fact, Span)> {
    let lines: Vec<&str> = markdown.lines().collect();
    if lines.is_empty() {
        return Vec::new();
    }

    let mut out = Vec::new();
    for fact in Fact::all() {
        let scores: Vec<u32> = lines.iter().map(|l| score_for(fact, l)).collect();
        let Some(peak) = (0..lines.len()).max_by_key(|i| scores[*i]) else {
            continue;
        };
        if scores[peak] < FLOOR {
            continue;
        }
        out.push((fact, window(&lines, peak, scores[peak])));
    }
    out
}

/// The sentence that scored, with the one before and after it.
///
/// Wider than the scoring line because a place name often follows the phrase that introduces
/// it, and narrower than a section because an about page's paragraphs are unrelated to each
/// other.
fn window(lines: &[&str], peak: usize, score: u32) -> Span {
    let start = peak.saturating_sub(1);
    let end = (peak + 1).min(lines.len().saturating_sub(1));
    let mut text = lines[start..=end].join("\n");
    if text.chars().count() > WINDOW_CHARS {
        text = text.chars().take(WINDOW_CHARS).collect();
    }
    Span {
        text,
        starts_at_line: start,
        heading: None,
        score,
    }
}

/// How much this line looks like it states one particular fact.
fn score_for(fact: Fact, line: &str) -> u32 {
    let lower = line.to_lowercase();
    let (phrases, needs_number) = match fact {
        // Broader than it first was, and `plausible.io/about` is why: it says *"Uku Taht
        // started Plausible in December 2018"*, which `started in` does not match — a name
        // sits between the verb and the preposition. The window went elsewhere, the model
        // answered 2018 from somewhere else, and the grounding check dropped a year the page
        // states plainly.
        Fact::Founded => (
            [
                "founded", "started", "launched", "began", "since", "est.", "created",
            ]
            .as_slice(),
            true,
        ),
        Fact::Headquarters => (
            [
                "headquarter",
                "based in",
                "our office",
                "offices in",
                "we are located",
                "company based",
            ]
            .as_slice(),
            false,
        ),
        Fact::Employees => (
            [
                "employees",
                "team of",
                "people work",
                "we are a team",
                "staff of",
                "headcount",
            ]
            .as_slice(),
            true,
        ),
    };

    if !phrases.iter().any(|p| lower.contains(p)) {
        return 0;
    }
    // A phrase with nothing to attach it to is a sentence about something else. `founded` is
    // in "founded on the belief that"; `team of` is in "team of people who care".
    let has_number = match fact {
        Fact::Founded => year_in(&lower).is_some(),
        _ => lower.chars().any(|c| c.is_ascii_digit()),
    };
    if needs_number && !has_number {
        return 0;
    }
    10 + u32::from(has_number) * 2
}

/// A four-digit year a company could have been founded in.
#[must_use]
pub fn year_in(text: &str) -> Option<u16> {
    let bytes: Vec<char> = text.chars().collect();
    for start in 0..bytes.len().saturating_sub(3) {
        if !bytes[start].is_ascii_digit() {
            continue;
        }
        let before_is_digit = start > 0 && bytes[start - 1].is_ascii_digit();
        let after = bytes.get(start + 4);
        if before_is_digit || after.is_some_and(char::is_ascii_digit) {
            continue;
        }
        let digits: String = bytes[start..start + 4].iter().collect();
        if !digits.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        if let Ok(year) = digits.parse::<u16>() {
            if (1800..=2100).contains(&year) {
                return Some(year);
            }
        }
    }
    None
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn facts(page: &str) -> Vec<Fact> {
        every_fact(page).into_iter().map(|(f, _)| f).collect()
    }

    #[test]
    fn a_page_that_states_all_three_produces_three_windows() {
        let page = "# About us
We were founded in 2019 by two people who were tired of it.
Plausible Analytics is an independent company based in the EU.
Today Plausible is a team of 10. You can meet everyone on our team page.";
        assert_eq!(
            facts(page),
            [Fact::Founded, Fact::Headquarters, Fact::Employees]
        );
    }

    #[test]
    fn a_page_that_only_tells_a_story_produces_nothing() {
        // Most about pages. Not a failure — the coverage note is what says so.
        let page = "# Our story\nPeople often ask why we built this. They have never seen \
                    anything like it, so they are curious where the idea came from.";
        assert!(every_fact(page).is_empty());
    }

    #[test]
    fn an_age_is_not_a_founding_year() {
        // basecamp.com/about: "We're here for them, 23 years and running." The year is
        // arithmetic, not reading, and a module that computed it would be inventing a fact
        // the page does not state.
        let page = "It has been an incredible ride. We're here for them, 23 years and running.";
        assert!(!facts(page).contains(&Fact::Founded), "{:?}", facts(page));
    }

    #[test]
    fn a_founding_sentence_with_a_name_in_the_middle_is_still_one() {
        // plausible.io/about, verbatim. `started in` does not match it; `started` does.
        let page = "Uku Taht started Plausible in December 2018, building it alone.";
        let found = every_fact(page);
        assert!(found.iter().any(|(f, _)| *f == Fact::Founded), "{found:#?}");
    }

    #[test]
    fn a_founding_phrase_with_no_year_is_not_a_founding_year() {
        // "founded on the belief that" is the commonest sentence on an about page.
        let page = "This company was founded on the belief that work should be calmer.";
        assert!(!facts(page).contains(&Fact::Founded));
    }

    #[test]
    fn a_team_with_no_number_is_not_a_headcount() {
        let page = "We are a team of people who care about the craft more than the growth.";
        assert!(!facts(page).contains(&Fact::Employees));
    }

    #[test]
    fn the_window_carries_the_sentence_either_side() {
        // A place name often lands on the line after the phrase that introduces it.
        let page = "# About\nOur headquarters are in\nChicago, Illinois, and have been since 2003.";
        let found = every_fact(page);
        let (_, span) = found
            .iter()
            .find(|(f, _)| *f == Fact::Headquarters)
            .expect("a headquarters line");
        assert!(span.text.contains("Chicago"), "{}", span.text);
    }

    #[test]
    fn every_window_stays_inside_the_budget() {
        let long = "We are based in a very long sentence about our offices. ".repeat(200);
        for (_, span) in every_fact(&long) {
            assert!(span.text.chars().count() <= WINDOW_CHARS);
        }
    }

    #[test]
    fn a_year_is_read_only_when_it_is_a_year() {
        assert_eq!(year_in("founded in 2019 by two people"), Some(2019));
        assert_eq!(year_in("we have 12000 customers"), None);
        assert_eq!(year_in("suite 4100, chicago"), None);
        assert_eq!(year_in("since 1899"), Some(1899));
        assert_eq!(year_in("no digits here"), None);
    }

    #[test]
    fn an_empty_page_does_not_panic() {
        assert!(every_fact("").is_empty());
        assert!(every_fact("\n\n").is_empty());
    }
}
