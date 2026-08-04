//! Reading a changelog: what shipped, and when.
//!
//! The third question kind, and the one [`ARCHITECTURE.md`] §5.4 is most emphatic about:
//!
//! > | Changelog entries, release dates, version numbers | **Code** — heading + `<time>` +
//! > date regex | Dates are the most common LLM fabrication in "recent changes" and are
//! > trivially verifiable. |
//!
//! **So no model runs here at all.** A date either appears on the page or it does not, and a
//! parser that finds one carries an exact line number with it. Nothing a language model could
//! add to that is worth what it would risk.
//!
//! # What a changelog looks like
//!
//! Two real pages, and they agree:
//!
//! ```text
//! - Jul 14, 2026                        plausible.io/changelog
//! ## Add notes to your traffic chart with annotations
//!
//! July 31, 2026                         notion.com/releases
//! ## AI Meeting Notes can now trigger Custom Agents
//! ```
//!
//! A date on a line of its own, then a heading. The other shape — `2026-07-14 — Linear
//! shipped Initiatives`, which is how `PRODUCT_SPEC.md` §4 renders one — puts both on one
//! line, and both are read here.
//!
//! # The line has to *be* a date
//!
//! Notion's own changelog contains *"Starting August 11 2026, Workers will run on Notion
//! credits."* That is a date in a sentence, and a parser that takes any date it finds would
//! file a future promise as a shipped change. **A date is an entry only when the line is the
//! date** — alone, or followed by a separator and a title.
//!
//! [`ARCHITECTURE.md`]: ../../../docs/ARCHITECTURE.md

use crate::doc;

/// Bumped whenever these rules change.
pub const CHANGES_VERSION: u32 = 1;

/// The most entries taken from one changelog.
///
/// A busy changelog has hundreds and a report shows a quarter's worth. This is a bound on the
/// work, not a judgement about the page, and the number passed over is reported alongside —
/// `PRODUCT_SPEC.md` §4's coverage note exists so that a short list is never mistaken for a
/// quiet quarter.
pub const MAX_ENTRIES: usize = 40;

/// One dated entry, exactly as the page states it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub year: i32,
    pub month: u32,
    pub day: u32,
    /// The heading or the text beside the date. Empty when the page dated something without
    /// titling it.
    pub title: String,
    /// The line the date was read from, copied verbatim.
    pub quote: String,
    pub at_line: usize,
}

/// What a changelog page yielded.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Entries {
    pub entries: Vec<Entry>,
    /// How many dated entries the page held before [`MAX_ENTRIES`] was applied.
    pub considered: usize,
}

/// Every dated entry on the page, in the order it presents them.
#[must_use]
pub fn every_change(markdown: &str) -> Entries {
    let lines: Vec<&str> = markdown.lines().collect();
    let mut entries: Vec<Entry> = Vec::new();

    for (i, line) in lines.iter().enumerate() {
        let Some(dated) = dated_line(line) else {
            continue;
        };
        let title = if dated.rest.is_empty() {
            heading_below(&lines, i)
        } else {
            dated.rest
        };
        // The same date twice with the same title is one entry written twice — some pages
        // repeat the date in a heading and again in a byline.
        if entries.last().is_some_and(|e| {
            (e.year, e.month, e.day) == (dated.year, dated.month, dated.day) && e.title == title
        }) {
            continue;
        }
        entries.push(Entry {
            year: dated.year,
            month: dated.month,
            day: dated.day,
            title,
            quote: line.trim().to_owned(),
            at_line: i,
        });
    }

    let considered = entries.len();
    entries.truncate(MAX_ENTRIES);
    Entries {
        entries,
        considered,
    }
}

/// A date, and whatever the line said after it.
struct Dated {
    year: i32,
    month: u32,
    day: u32,
    rest: String,
}

/// Read a line that *is* a date, rather than one that mentions one.
///
/// The date has to come first. That single condition is what separates a changelog entry from
/// Notion's *"Starting August 11 2026, Workers will run on Notion credits"* — a promise about
/// the future, and the worst thing this parser could file as a shipped change.
fn dated_line(line: &str) -> Option<Dated> {
    let stripped = strip_markers(line.trim());
    let (year, month, day, consumed) = parse_date(stripped)?;

    let after = &stripped[consumed.len()..];
    let rest = after
        .trim()
        .trim_start_matches(['-', '—', '–', ':', '|', '·', '•'])
        .trim()
        .to_owned();
    // Either the date stands alone, or a separator introduces a title. A date followed
    // straight into prose is a sentence again.
    if !rest.is_empty() && !starts_with_separator(after) {
        return None;
    }
    Some(Dated {
        year,
        month,
        day,
        rest,
    })
}

/// Whether what follows a date introduces a title rather than continuing a sentence.
fn starts_with_separator(after: &str) -> bool {
    after
        .trim_start()
        .starts_with(['-', '—', '–', ':', '|', '·', '•'])
}

/// List markers and heading hashes, removed so both shapes read the same.
fn strip_markers(line: &str) -> &str {
    let no_heading = doc::heading_text(line);
    let trimmed = no_heading.trim_start();
    let no_bullet = trimmed
        .strip_prefix("- ")
        .or_else(|| trimmed.strip_prefix("* "))
        .unwrap_or(trimmed);
    no_bullet.trim()
}

/// The heading that titles an entry whose date is on its own line.
fn heading_below(lines: &[&str], from: usize) -> String {
    /// A date and its heading are adjacent; more than this and they belong to different
    /// entries.
    const REACH: usize = 3;

    lines
        .iter()
        .skip(from + 1)
        .take(REACH)
        .find(|l| doc::heading_level(l).is_some() && !doc::heading_text(l).is_empty())
        .map(|l| doc::heading_text(l).to_owned())
        .unwrap_or_default()
}

/// Parse a date at the start-ish of `text`, returning it and the exact text matched.
///
/// Three shapes, because three shapes is what real changelogs use: ISO, month-first, and
/// day-first. A bare year is not a date — `© 2026 Notion Labs` is a footer.
fn parse_date(text: &str) -> Option<(i32, u32, u32, &str)> {
    iso(text)
        .or_else(|| month_first(text))
        .or_else(|| day_first(text))
}

/// `2026-07-14` or `2026/07/14`.
fn iso(text: &str) -> Option<(i32, u32, u32, &str)> {
    let bytes = text.as_bytes();
    if bytes.len() < 10 {
        return None;
    }
    let sep = bytes[4];
    if sep != b'-' && sep != b'/' || bytes[7] != sep {
        return None;
    }
    let year: i32 = text.get(..4)?.parse().ok()?;
    let month: u32 = text.get(5..7)?.parse().ok()?;
    let day: u32 = text.get(8..10)?.parse().ok()?;
    valid(year, month, day).then(|| (year, month, day, text.get(..10).unwrap_or_default()))
}

/// `Jul 14, 2026`, `July 14 2026`.
fn month_first(text: &str) -> Option<(i32, u32, u32, &str)> {
    let mut words = text.split_whitespace();
    let month = month_number(words.next()?)?;
    let day_word = words.next()?;
    let day: u32 = day_word.trim_end_matches(',').parse().ok()?;
    let year: u32 = words.next()?.trim_end_matches(',').parse().ok()?;
    let year = i32::try_from(year).ok()?;
    valid(year, month, day).then(|| (year, month, day, matched(text, 3)))
}

/// `14 July 2026`, `14 Jul 2026`.
fn day_first(text: &str) -> Option<(i32, u32, u32, &str)> {
    let mut words = text.split_whitespace();
    let day: u32 = words.next()?.trim_end_matches(['.', ',']).parse().ok()?;
    let month = month_number(words.next()?)?;
    let year: u32 = words.next()?.trim_end_matches(',').parse().ok()?;
    let year = i32::try_from(year).ok()?;
    valid(year, month, day).then(|| (year, month, day, matched(text, 3)))
}

/// The prefix of `text` covering its first `words` whitespace-separated words.
fn matched(text: &str, words: usize) -> &str {
    let mut seen = 0usize;
    let mut end = text.len();
    let mut in_word = false;
    for (i, c) in text.char_indices() {
        if c.is_whitespace() {
            if in_word {
                seen += 1;
                in_word = false;
                if seen == words {
                    end = i;
                    break;
                }
            }
        } else {
            in_word = true;
        }
    }
    &text[..end]
}

/// A month name or its three-letter abbreviation.
fn month_number(word: &str) -> Option<u32> {
    const MONTHS: [&str; 12] = [
        "january",
        "february",
        "march",
        "april",
        "may",
        "june",
        "july",
        "august",
        "september",
        "october",
        "november",
        "december",
    ];
    let lower = word.trim_end_matches(['.', ',']).to_lowercase();
    if lower.len() < 3 {
        return None;
    }
    MONTHS
        .iter()
        .position(|m| *m == lower || m.starts_with(&lower) && lower.len() == 3)
        .map(|i| u32::try_from(i + 1).unwrap_or(1))
}

/// A date a changelog could plausibly carry.
///
/// The range is a sanity check rather than a calendar: `31/13/2026` is a parse artefact, and
/// a change dated 1970 is a timestamp that lost its formatting.
fn valid(year: i32, month: u32, day: u32) -> bool {
    (2000..=2100).contains(&year) && (1..=12).contains(&month) && (1..=31).contains(&day)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn a_date_line_followed_by_a_heading_is_an_entry() {
        // plausible.io/changelog, exactly.
        let page = "- Jul 14, 2026\n## Add notes to your traffic chart with annotations\nYou can now add annotations.";
        let found = every_change(page);
        assert_eq!(found.entries.len(), 1, "{found:#?}");
        let entry = &found.entries[0];
        assert_eq!((entry.year, entry.month, entry.day), (2026, 7, 14));
        assert_eq!(
            entry.title,
            "Add notes to your traffic chart with annotations"
        );
        assert_eq!(entry.quote, "- Jul 14, 2026");
    }

    #[test]
    fn a_date_and_a_title_on_one_line_is_an_entry() {
        // How PRODUCT_SPEC §4 renders one.
        let page = "2026-07-14 — Linear shipped Initiatives for multi-project planning";
        let found = every_change(page);
        assert_eq!(found.entries.len(), 1, "{found:#?}");
        assert_eq!(
            found.entries[0].title,
            "Linear shipped Initiatives for multi-project planning"
        );
    }

    #[test]
    fn a_date_inside_a_sentence_is_not_an_entry() {
        // From notion.com/releases. A promise about the future, and the single worst thing
        // this parser could file as a shipped change.
        let page = "Starting August 11 2026, Workers will run on Notion credits.";
        assert!(every_change(page).entries.is_empty());
    }

    #[test]
    fn a_date_running_straight_into_prose_is_not_an_entry() {
        let page = "July 31, 2026 was a good month for us and we are pleased to say so.";
        assert!(every_change(page).entries.is_empty());
    }

    #[test]
    fn a_bare_year_is_not_a_date() {
        // `© 2026 Notion Labs, Inc.` is the last line of that page.
        assert!(every_change("© 2026 Notion Labs, Inc.").entries.is_empty());
        assert!(every_change("2026").entries.is_empty());
    }

    #[test]
    fn the_three_shapes_real_changelogs_use_are_all_read() {
        for (line, expected) in [
            ("2026-07-14", (2026, 7, 14)),
            ("2026/07/14", (2026, 7, 14)),
            ("Jul 14, 2026", (2026, 7, 14)),
            ("July 14 2026", (2026, 7, 14)),
            ("14 July 2026", (2026, 7, 14)),
            ("14 Jul 2026", (2026, 7, 14)),
        ] {
            let found = every_change(line);
            assert_eq!(found.entries.len(), 1, "{line}");
            let e = &found.entries[0];
            assert_eq!((e.year, e.month, e.day), expected, "{line}");
        }
    }

    #[test]
    fn a_date_that_is_not_a_date_is_rejected() {
        for line in ["2026-13-40", "Jul 99, 2026", "1970-01-01", "Maybe 14, 2026"] {
            assert!(every_change(line).entries.is_empty(), "{line}");
        }
    }

    #[test]
    fn a_heading_can_carry_the_date() {
        let page = "## 2026-06-24: See full URLs in Page reports\nYou can now break these down.";
        let found = every_change(page);
        assert_eq!(found.entries.len(), 1, "{found:#?}");
        assert_eq!(found.entries[0].title, "See full URLs in Page reports");
    }

    #[test]
    fn a_dated_entry_with_no_title_is_still_an_entry() {
        // The date is the fact. A missing title is a thinner entry, not a reason to drop a
        // shipped change.
        let found = every_change("2026-07-14\n\nSomething happened, untitled.");
        assert_eq!(found.entries.len(), 1);
        assert!(found.entries[0].title.is_empty());
    }

    #[test]
    fn entries_come_back_in_the_order_the_page_lists_them() {
        let page = "- Jul 14, 2026\n## Newest\n- Jun 24, 2026\n## Older\n- May 27, 2026\n## Oldest";
        let found = every_change(page);
        let titles: Vec<&str> = found.entries.iter().map(|e| e.title.as_str()).collect();
        assert_eq!(titles, ["Newest", "Older", "Oldest"]);
    }

    #[test]
    fn the_same_date_and_title_twice_is_one_entry() {
        let page = "## 2026-07-14: Shipped it\n2026-07-14 - Shipped it";
        assert_eq!(every_change(page).entries.len(), 1);
    }

    #[test]
    fn the_cap_is_reported_rather_than_applied_silently() {
        let mut page = String::new();
        let mut written = 0usize;
        for month in 1..=3 {
            for day in 1..=18 {
                page.push_str(&format!("2026-{month:02}-{day:02} — Entry {month}.{day}\n"));
                written += 1;
            }
        }
        let found = every_change(&page);
        assert_eq!(found.entries.len(), MAX_ENTRIES);
        assert_eq!(found.considered, written);
    }

    #[test]
    fn a_page_with_no_dates_yields_nothing() {
        // linear.app/docs/releases.md is documentation about a feature called Releases, and
        // this is what makes that visible rather than inventing a changelog from it.
        let page = "# Releases\n## Overview\nLinear can group issues into releases.";
        assert!(every_change(page).entries.is_empty());
    }

    #[test]
    fn an_empty_page_does_not_panic() {
        assert!(every_change("").entries.is_empty());
        assert!(every_change("\n\n").entries.is_empty());
    }
}
