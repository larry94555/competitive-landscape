//! Choosing the parts of a features page that name something the product does.
//!
//! The second question kind, after pricing. [`ARCHITECTURE.md`] §5.4 assigns it a shape:
//!
//! > | Feature lists on structured pages | **Code first**, model for normalization only |
//!
//! # What a features page actually looks like
//!
//! The guess was bullet lists. Four real pages say otherwise:
//!
//! ```text
//! ## Message Boards for announcements and discussions      basecamp.com/features
//! Message Boards essentially replace email. All project…
//!
//! ### Customer Requests                                    linear.app/features
//! Build what customers actually want
//!
//! ### Capture                                              notion.com/product/docs
//! Bring everything into one system of record.
//! ```
//!
//! **A capability is a heading with a description under it.** The bullet lists on those pages
//! are navigation — `- Pricing`, `- Log in`, `- Customer stories` — and treating bullets as
//! capabilities would have reported Basecamp's footer as fourteen features.
//!
//! So this module finds *named things with descriptions*, and the model turns each into a
//! capability name. That division is §5.4's: the page's structure is already an answer, and
//! only the wording needs a model. `Message Boards for announcements and discussions` is a
//! heading a parser can find and a name only a reader can shorten.
//!
//! [`ARCHITECTURE.md`]: ../../../docs/ARCHITECTURE.md

use crate::doc;
use crate::span::{Span, WINDOW_CHARS};

/// Bumped whenever these rules change, for the same reason spans are versioned: two pages cut
/// by different rules produce counts that are not comparable.
pub const CAPABILITY_VERSION: u32 = 1;

/// How many capabilities one page may contribute.
///
/// Each is a model call. Twelve is more than a feature-comparison matrix has rows, and a page
/// producing more is better served by reporting the first twelve and **saying how many it
/// passed over** than by spending forty calls on a navigation menu.
pub const MAX_CAPABILITIES: usize = 12;

/// The shortest description that distinguishes a capability from a link.
///
/// `### Capture` / *"Bring everything into one system of record."* is a capability.
/// `### Product` with a list of links under it is a menu. Four words is what separates them on
/// every page measured — Notion's shortest real description is five.
const MIN_DESCRIPTION_WORDS: usize = 4;

/// A name longer than this is a sentence, and a section titled with a sentence is a pitch.
const MAX_NAME_WORDS: usize = 14;

/// What a features page offered, and what was left out.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Candidates {
    /// One window per capability, in the order the page presents them.
    pub windows: Vec<Span>,
    /// How many sections looked like capabilities before [`MAX_CAPABILITIES`] was applied.
    ///
    /// Carried so a caller can say *"twelve of seventeen"* out loud. A cap that is not
    /// reported reads as completeness, which is the failure this whole product is about.
    pub considered: usize,
}

/// Every capability the page names, one window each.
#[must_use]
pub fn every_capability(markdown: &str) -> Candidates {
    let lines: Vec<&str> = markdown.lines().collect();
    if lines.is_empty() {
        return Candidates::default();
    }

    let mut windows: Vec<Span> = Vec::new();
    for (start, end) in doc::sections(&lines) {
        // A section with no heading of its own is the text above the page's first one — the
        // browser title, a cookie banner, a menu. It names nothing.
        //
        // And a level-1 heading is positioning rather than a capability: `# Where there's
        // work, there's Basecamp.` is how a features page opens, and it names nothing you can
        // buy.
        match doc::heading_level(lines[start]) {
            None | Some(1) => continue,
            Some(_) => {}
        }
        let Some((name, named_at)) = names_something(&lines, start, end) else {
            continue;
        };
        let described = describing_words(&lines, named_at, end);
        if described < MIN_DESCRIPTION_WORDS || is_instructions(&lines, start, end) {
            continue;
        }

        let mut text = lines[start..=end].join("\n");
        if text.chars().count() > WINDOW_CHARS {
            text = text.chars().take(WINDOW_CHARS).collect();
        }
        windows.push(Span {
            text,
            starts_at_line: start,
            heading: Some(name),
            // How much the page says about it. Not a confidence — a section with one line of
            // description and one with ten are both capabilities — but it is what tells a
            // reader which window to look at first when an answer comes back wrong.
            score: u32::try_from(described).unwrap_or(u32::MAX),
        });
    }

    let considered = windows.len();
    windows.truncate(MAX_CAPABILITIES);
    Candidates {
        windows,
        considered,
    }
}

/// The name this section gives the thing it is about, if it gives one.
///
/// Usually the heading. Sometimes the heading is bare — `linear.app/features` emits `###` and
/// puts *Planning* on the line below it — and then the first line of content is the name.
fn names_something(lines: &[&str], start: usize, end: usize) -> Option<(String, usize)> {
    let named = match doc::section_heading(lines, start, end) {
        Some(heading) => lines[start..=end]
            .iter()
            .position(|l| l.trim() == heading)
            .map(|offset| (heading, start + offset))?,
        None => lines[start..=end]
            .iter()
            .enumerate()
            .find(|(_, l)| !l.trim().is_empty() && doc::heading_level(l).is_none())
            .map(|(i, l)| (l.trim().to_owned(), start + i))?,
    };

    let text = doc::heading_text(&named.0);
    let words = text.split_whitespace().count();
    if words == 0 || words > MAX_NAME_WORDS || is_furniture(text) || is_menu_heading(text) {
        return None;
    }
    Some(named)
}

/// Whether a section tells you how to set something up rather than what the product does.
///
/// A code block is the tell, and it is a strong one: `linear.app/docs/mcp.md` is a setup
/// document that discovery labels *features*, and its sections are named `### Zed`,
/// `### Windsurf`, `### Visual Studio Code` — each with a JSON snippet underneath. Read as
/// capabilities they say Linear's product includes Zed, which it does not.
///
/// This does not fix the mislabel, and nothing here can: the page is documentation and the
/// label came from discovery. It stops the most confident version of the wrong answer.
fn is_instructions(lines: &[&str], start: usize, end: usize) -> bool {
    lines[start..=end]
        .iter()
        .any(|l| l.trim_start().starts_with("```"))
}

/// Whether a name is one a site gives to a menu rather than to a capability.
///
/// A short list, and deliberately so — every entry is a footer convention rather than a guess
/// about wording. `### Legal` on `linear.app/features` sits over four links and then repeats
/// them as a line of text, which is enough prose to pass every structural test here.
fn is_menu_heading(name: &str) -> bool {
    const NAMES: [&str; 7] = [
        "legal",
        "resources",
        "company",
        "follow us",
        "social",
        "sitemap",
        "navigation",
    ];
    let lower = name.trim().to_lowercase();
    NAMES.contains(&lower.as_str())
}

/// How many words the section spends describing the thing, rather than naming or linking it.
///
/// List items do not count. On every page measured the lists are menus, and a section whose
/// only content is a list of two-word links is the site's navigation rather than one of its
/// capabilities.
fn describing_words(lines: &[&str], named_at: usize, end: usize) -> usize {
    lines[named_at..=end]
        .iter()
        .map(|l| l.trim())
        .skip(1)
        .filter(|l| doc::heading_level(l).is_none())
        .filter(|l| !l.starts_with('-') && !l.starts_with('*') && !l.starts_with('|'))
        .filter(|l| !is_furniture(l))
        .map(|l| l.split_whitespace().count())
        .sum()
}

/// Whether a line is part of the site rather than part of the product.
///
/// Calls to action and menu labels sit in headings on real pages —
/// `### Plan the present. Build the future.` above *"Contact sales Get started Download Open
/// app"* is a section of `linear.app/features`, and it is a button, not a feature.
fn is_furniture(line: &str) -> bool {
    const PHRASES: [&str; 14] = [
        "contact sales",
        "get started",
        "sign up",
        "log in",
        "book a demo",
        "start free trial",
        "try it free",
        "download the app",
        "see all features",
        "customer stories",
        "privacy policy",
        "terms of service",
        "cookie",
        "subscribe to our",
    ];
    let lower = line.to_lowercase();
    PHRASES.iter().any(|p| lower.contains(p))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    /// The shape all four measured pages share.
    fn features_page() -> String {
        "# Where there's work, there's Basecamp.
If there are tasks to do and work to deliver, Basecamp helps make it happen.
## Message Boards for announcements and discussions
Message Boards essentially replace email. All project announcements live on the board.
## Hill Charts show you where things really stand
Is the project going to be done on time? Basecamp's Hill Charts let you know.
## And there's more
- Basecamp 5 is here
- Books we've written
- Where we came from"
            .to_owned()
    }

    #[test]
    fn a_capability_is_a_heading_with_a_description_under_it() {
        let found = every_capability(&features_page());
        let names: Vec<_> = found
            .windows
            .iter()
            .filter_map(|w| w.heading.clone())
            .collect();
        assert_eq!(
            names,
            [
                "## Message Boards for announcements and discussions",
                "## Hill Charts show you where things really stand"
            ],
            "{found:#?}"
        );
    }

    #[test]
    fn a_list_of_links_is_not_a_capability() {
        // `## And there's more` sits above a footer menu. Counting bullets as features would
        // have reported Basecamp's footer as fourteen of them.
        let found = every_capability(&features_page());
        assert!(
            !found
                .windows
                .iter()
                .any(|w| w.text.contains("Books we've written")),
            "{found:#?}"
        );
    }

    #[test]
    fn the_page_title_is_positioning_rather_than_a_capability() {
        let found = every_capability(&features_page());
        assert!(!found
            .windows
            .iter()
            .any(|w| w.heading.as_deref() == Some("# Where there's work, there's Basecamp.")));
    }

    #[test]
    fn a_bare_heading_takes_its_name_from_the_line_below() {
        // linear.app/features emits `###` and puts the name on the next line. Requiring a
        // heading with text in it would lose two of its eight capabilities.
        let page = "###\nPlanning\nSet the product direction with projects and initiatives";
        let found = every_capability(page);
        assert_eq!(found.windows.len(), 1, "{found:#?}");
        assert_eq!(found.windows[0].heading.as_deref(), Some("Planning"));
    }

    #[test]
    fn a_call_to_action_is_not_a_capability() {
        // A real section of linear.app/features, and it is a button.
        let page =
            "### Plan the present. Build the future.\nContact sales Get started Download Open app";
        assert!(every_capability(page).windows.is_empty());
    }

    #[test]
    fn a_heading_with_nothing_under_it_is_not_a_capability() {
        let page = "### Security\n### Mobile\nMove product work forward from anywhere";
        let found = every_capability(page);
        assert_eq!(found.windows.len(), 1, "{found:#?}");
        assert_eq!(found.windows[0].heading.as_deref(), Some("### Mobile"));
    }

    #[test]
    fn a_setup_section_is_not_a_capability() {
        // linear.app/docs/mcp.md is a setup document that discovery labels *features*. Its
        // sections are named for other people's editors, and read as capabilities they say
        // Linear's product includes Zed.
        let page = "### Zed
1. Open Zed settings and add the following:
```json
{ \"context_servers\": { \"linear\": { \"source\": \"custom\" } } }
```";
        assert!(every_capability(page).windows.is_empty());
    }

    #[test]
    fn a_footer_menu_is_not_a_capability() {
        // `### Legal` on linear.app/features sits over four links and then repeats them as a
        // line of text, which passes every structural test here.
        let page = "### Legal
- Privacy
- Terms
- DPA
- AUP
Privacy Terms DPA AUP";
        assert!(every_capability(page).windows.is_empty());
    }

    #[test]
    fn the_cap_is_reported_rather_than_applied_silently() {
        // A cap nobody is told about reads as completeness.
        let mut page = String::new();
        for n in 0..20 {
            page.push_str(&format!(
                "## Capability number {n}\nIt does a thing that is useful to you.\n"
            ));
        }
        let found = every_capability(&page);
        assert_eq!(found.windows.len(), MAX_CAPABILITIES);
        assert_eq!(found.considered, 20);
    }

    #[test]
    fn each_window_stays_inside_the_budget() {
        let long = "It does a great many useful things, described at length. ".repeat(80);
        let page = format!("## Reports\n{long}");
        for window in every_capability(&page).windows {
            assert!(window.text.chars().count() <= WINDOW_CHARS);
        }
    }

    #[test]
    fn a_page_with_no_named_sections_offers_nothing() {
        // Not a failure. A features page that converts to a paragraph is a finding, and the
        // honest answer is that we found no capabilities on it.
        let page = "We make software for teams. It is good software and people like it.";
        assert!(every_capability(page).windows.is_empty());
    }

    #[test]
    fn an_empty_page_does_not_panic() {
        assert!(every_capability("").windows.is_empty());
        assert!(every_capability("\n\n").windows.is_empty());
    }
}
