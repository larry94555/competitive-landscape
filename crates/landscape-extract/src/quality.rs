//! Is this Markdown worth sending to a model?
//!
//! `ROADMAP.md` Phase 1 asks for an `extraction_quality` score alongside the conversion.
//! It earns its place by saving the most expensive thing we have: a page that converted to
//! forty words of navigation costs ~12 seconds of extraction on the target hardware and
//! returns nothing.
//!
//! # What it is not
//!
//! **Not a measure of whether the page is useful** — that is the model's job, and the golden
//! set already scores how well it does it. This asks the narrower question: *did the
//! conversion produce something with content in it?*
//!
//! The distinction matters because the two failures need different responses. A page that
//! converted badly should be re-fetched or reported as unread; a page that converted fine
//! and simply has no price is a **finding**, and `FACT_CHECKING.md` §5.4 wants it displayed
//! rather than retried.
//!
//! # Deliberately crude
//!
//! Four signals, no weights fitted to anything. A score that looks precise invites being
//! trusted to two decimal places, and nothing here has earned that. What it can do reliably
//! is separate "a page" from "a menu", which is the decision in front of it.

/// How usable a converted page looks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Quality {
    /// Structure and prose. Worth a model pass.
    Good,
    /// Thin, or nearly all links and headings. Worth trying, and worth doubting.
    Thin,
    /// Not enough to extract from. **A finding, not an error** — the page was read, and
    /// there was nothing on it. That is what a report says.
    Empty,
}

impl Quality {
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Good => "good",
            Self::Thin => "thin",
            Self::Empty => "empty",
        }
    }

    /// Whether to spend a model pass on this.
    ///
    /// `Thin` is included: a short pricing page is still a pricing page, and refusing to
    /// read it would turn a real answer into a gap. Only `Empty` is skipped.
    #[must_use]
    pub const fn worth_extracting(self) -> bool {
        matches!(self, Self::Good | Self::Thin)
    }
}

/// A page converted to Markdown, assessed.
#[derive(Debug, Clone)]
pub struct Assessment {
    pub quality: Quality,
    pub words: usize,
    /// Headings, list items and table rows. Structure is what survives conversion when the
    /// page had any, and its absence is the clearest signal that the page was a shell.
    pub structured_lines: usize,
    /// True when a table survived. Worth naming separately because a pricing table is the
    /// most valuable structure on the page this product cares about.
    pub has_table: bool,
}

/// The fewest words that could possibly carry a fact.
///
/// Deliberately low. The first draft set this at 25 and scored a real pricing page —
/// *"# Plans / - Free — $0 a month / - Pro — $12 a month / Cancel whenever you like."* — as
/// empty at 18 words. **Structure is content**, and a floor made of words alone cannot see
/// that.
pub const MIN_WORDS: usize = 12;

/// Words per structural line below which a list is a menu rather than a page.
///
/// This is the signal that separates a navigation bar from a terse pricing table, and it
/// does it better than any word count: `- Home` and `- About` carry one word each, while
/// `- Pro — $12 a month` carries five. Many items with nothing in them is a menu.
pub const MIN_WORDS_PER_LINE: usize = 3;

/// At or above this many words, prose alone is enough without any structure.
///
/// Plenty of real pricing pages are written as paragraphs.
pub const PROSE_IS_ENOUGH: usize = 120;

/// Assess converted Markdown.
#[must_use]
pub fn assess(markdown: &str) -> Assessment {
    let words = markdown.split_whitespace().count();
    let mut structured_lines = 0usize;
    let mut has_table = false;

    for line in markdown.lines() {
        let t = line.trim_start();
        if t.starts_with('#') || t.starts_with("- ") {
            structured_lines += 1;
        } else if t.starts_with('|') {
            structured_lines += 1;
            // The separator row is the thing that makes it a table rather than a line of
            // pipes, so it is what gets checked.
            if t.contains("---") {
                has_table = true;
            }
        }
    }

    // A menu: several items, none of them containing a sentence. Checked before the word
    // floor because a long enough navigation bar clears any word count while carrying
    // nothing at all.
    let is_menu = structured_lines >= 3 && words / structured_lines.max(1) < MIN_WORDS_PER_LINE;

    let quality = if words < MIN_WORDS || is_menu {
        Quality::Empty
    } else if has_table || structured_lines >= 4 || words >= PROSE_IS_ENOUGH {
        Quality::Good
    } else {
        Quality::Thin
    };

    Assessment {
        quality,
        words,
        structured_lines,
        has_table,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn a_real_pricing_page_is_good() {
        let md =
            "# Pricing\n\n| Plan | Price |\n|---|---|\n| Starter | $19 |\n| Grower | $49 |\n\n\
                  All plans are billed monthly in US dollars and can be cancelled at any time. \
                  There is no setup fee on any plan, and annual billing is available on request.";
        let a = assess(md);
        assert_eq!(a.quality, Quality::Good, "{a:?}");
        assert!(a.has_table);
    }

    #[test]
    fn a_navigation_menu_is_empty() {
        // The case this exists to catch: ~12 seconds of extraction on the target hardware,
        // returning nothing.
        let a = assess("- Home\n- About\n- Login");
        assert_eq!(a.quality, Quality::Empty, "{a:?}");
        assert!(!a.quality.worth_extracting());
    }

    #[test]
    fn empty_means_a_finding_rather_than_a_retry() {
        // The distinction the module docs turn on: a page that converted badly should be
        // re-fetched; a page that was read and had nothing on it is what a report reports.
        // Only the second is `Empty`, and it is deliberately not worth extracting.
        assert!(!Quality::Empty.worth_extracting());
        assert!(Quality::Thin.worth_extracting());
        assert!(Quality::Good.worth_extracting());
    }

    #[test]
    fn a_terse_pricing_page_is_still_read() {
        // A short page is not an empty one, and refusing to read it would turn a real
        // answer into a gap. Thin is a doubt, not a refusal.
        let md = "# Plans\n\n- Free — $0 a month\n- Pro — $12 a month\n\nCancel whenever you like.";
        let a = assess(md);
        assert!(a.quality.worth_extracting(), "{a:?}");
    }

    #[test]
    fn a_wall_of_prose_with_no_structure_is_still_good_if_there_is_enough_of_it() {
        // Plenty of real pricing pages are written as paragraphs. Structure is a signal,
        // not a requirement.
        let md = "We charge thirty five dollars a month for it, with no commission on what \
                  you sell, because taking a cut of a grower's margin felt like the wrong way \
                  to build this. "
            .repeat(4);
        assert_eq!(assess(&md).quality, Quality::Good);
    }

    #[test]
    fn a_cookie_notice_that_clears_the_word_floor_is_only_thin() {
        let md = "We use cookies to improve your experience on this website and to show you \
                  content we think you will find relevant. Accept or manage preferences.";
        assert_eq!(assess(md).quality, Quality::Thin);
    }

    #[test]
    fn a_long_navigation_bar_is_empty_however_many_words_it_has() {
        // The reason the menu test is words-per-line rather than a word count: a big enough
        // navigation bar clears any floor while carrying nothing.
        let md = "- Home
- About
- Pricing
- Docs
- Blog
- Careers
- Status
                  - Security
- Login
- Sign up
- Contact
- Privacy";
        let a = assess(md);
        assert_eq!(a.quality, Quality::Empty, "{a:?}");
    }

    #[test]
    fn structure_rescues_a_page_a_word_count_would_reject() {
        // The first draft of this module set the floor at 25 words and scored this real
        // pricing page as empty at 18. Structure is content.
        let md = "# Plans

- Free — $0 a month
- Pro — $12 a month

Cancel whenever.";
        let a = assess(md);
        assert!(a.words < 25, "the test has drifted: {a:?}");
        assert!(a.quality.worth_extracting(), "{a:?}");
    }

    #[test]
    fn a_line_of_pipes_is_not_a_table() {
        // Only the separator row makes it a table. Without this, any page using pipes as
        // a visual divider would report `has_table`.
        let a = assess("| not | really | a | table |\nand some more words to clear the floor here");
        assert!(!a.has_table, "{a:?}");
    }

    #[test]
    fn nothing_in_is_empty_rather_than_a_panic() {
        for md in ["", "   ", "\n\n\n"] {
            let a = assess(md);
            assert_eq!(a.quality, Quality::Empty);
            assert_eq!(a.words, 0);
        }
    }
}
