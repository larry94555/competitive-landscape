//! The shape of a converted page: where its sections are, and what names them.
//!
//! This is the part of span selection that is not about pricing. A pricing page and a features
//! page are cut the same way — headings delimit sections, and a section is named by its own
//! heading — and only the question of *which* sections are worth keeping differs. Sharing the
//! cut is what keeps the two extractors from drifting into disagreeing about what a section is.
//!
//! Every rule here was written against pages fetched from real companies and read with
//! `cargo run -p landscape-extract --example plans`. The wrong answers they produced are in
//! `BENCHMARKS.md`, Runs 6 and 7.

/// A section is a heading and everything under it, as inclusive line indices.
pub type Section = (usize, usize);

/// Cut the page into sections.
///
/// A section starts at every heading. The one refinement is that **a heading immediately
/// followed by a deeper heading is a name followed by its subtitle**, not two sections.
#[must_use]
pub fn sections(lines: &[&str]) -> Vec<Section> {
    /// A name and its subtitle sit within a line or two. More than this and the shallower
    /// heading owns real content of its own.
    const SUBTITLE_LINES: usize = 4;

    if lines.is_empty() {
        return Vec::new();
    }

    let mut starts: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, l)| l.trim_start().starts_with('#'))
        .map(|(i, _)| i)
        .collect();
    if starts.first() != Some(&0) {
        starts.insert(0, 0);
    }

    let raw: Vec<Section> = starts
        .iter()
        .enumerate()
        .map(|(n, &s)| (s, starts.get(n + 1).map_or(lines.len(), |&e| e) - 1))
        .collect();

    let mut out: Vec<Section> = Vec::new();
    let mut carried: Option<usize> = None;
    for (n, &(s, e)) in raw.iter().enumerate() {
        let start = carried.take().unwrap_or(s);
        // A level-1 heading is the page's title, never the first half of a name-and-subtitle
        // pair. Merging it into what follows swallowed `linear.app/features`' first capability
        // whole: `# The system for modern product development` sits directly above a bare
        // `###` and the section that came out was the page rather than the feature.
        let is_lead = heading_level(lines[s]).is_some_and(|l| l > 1)
            && e.saturating_sub(s) < SUBTITLE_LINES
            && raw
                .get(n + 1)
                .and_then(|&(next, _)| heading_level(lines[next]))
                .zip(heading_level(lines[s]))
                .is_some_and(|(next, here)| next > here);
        if is_lead {
            carried = Some(start);
            continue;
        }
        out.push((start, e));
    }
    if let Some(start) = carried {
        out.push((start, lines.len().saturating_sub(1)));
    }
    out
}

/// `## Pro` is level 2. `None` for a line that is not a heading.
#[must_use]
pub fn heading_level(line: &str) -> Option<usize> {
    let t = line.trim_start();
    let level = t.chars().take_while(|c| *c == '#').count();
    (level > 0).then_some(level)
}

/// The heading that names a section.
///
/// A section opens with one heading or two, and when there are two, **the shorter one is the
/// name**. Which of the pair it is cannot be read off the levels, because both orders occur:
///
/// ```text
/// ## Pro Unlimited                                 basecamp.com
/// ### Top-of-the-line, all-inclusive pricing…
///
/// ## Essentials for staying organized.             notion.com
/// ### Free
/// ```
///
/// Length separates them in both, and not by coincidence: a name is a noun and a subtitle is a
/// sentence.
#[must_use]
pub fn section_heading(lines: &[&str], start: usize, end: usize) -> Option<String> {
    /// The same reach [`sections`] uses to recognize the pair.
    const PAIR: usize = 4;

    lines[start..=end.min(start.saturating_add(PAIR))]
        .iter()
        .filter(|l| heading_level(l).is_some())
        // A heading with no text names nothing. Real pages emit them: `todoist.com/pricing`
        // has three bare `###` above its comparison tables, and the shortest-wins rule would
        // hand the model `###` as the thing it is asking about.
        .filter(|l| !heading_text(l).is_empty())
        .min_by_key(|l| heading_text(l).chars().count())
        .map(|l| l.trim().to_owned())
}

/// Whether a line is a bullet in a list.
///
/// **What makes a hosted board readable.** A board puts its roles in bullets and everything
/// else — a search box, a job count, a department name — in the prose and headings between
/// them, so *"read the list the page marked"* is the difference between reading vacancies and
/// publishing *Create a Job Alert* as one. See `hiring::every_role`.
#[must_use]
pub fn is_list_item(line: &str) -> bool {
    let trimmed = line.trim_start();
    ["- ", "* ", "+ "]
        .iter()
        .any(|marker| trimmed.starts_with(marker))
}

/// A heading with its `#`s and surrounding space removed.
#[must_use]
pub fn heading_text(line: &str) -> &str {
    line.trim().trim_start_matches('#').trim()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn every_heading_starts_a_section() {
        let lines = ["# Title", "prose", "## One", "a", "## Two", "b"];
        assert_eq!(sections(&lines), [(0, 1), (2, 3), (4, 5)]);
    }

    #[test]
    fn text_above_the_first_heading_is_its_own_section() {
        let lines = ["prose", "more prose", "## One", "a"];
        assert_eq!(sections(&lines), [(0, 1), (2, 3)]);
    }

    #[test]
    fn a_heading_followed_by_a_deeper_one_is_a_name_and_a_subtitle() {
        // Not two sections: the pair belongs to one thing, and splitting them would hand a
        // model a subtitle with no name above it.
        let lines = [
            "## Pro Unlimited",
            "### All-inclusive pricing",
            "$299",
            "- A",
        ];
        assert_eq!(sections(&lines), [(0, 3)]);
    }

    #[test]
    fn a_page_title_is_never_a_lead() {
        let lines = [
            "# The system for modern product development",
            "###",
            "Planning",
            "Set it",
        ];
        assert_eq!(sections(&lines), [(0, 0), (1, 3)]);
    }

    #[test]
    fn a_heading_with_content_of_its_own_is_not_a_lead() {
        let lines = ["## Intro", "a", "b", "c", "d", "### Deeper", "e"];
        assert_eq!(sections(&lines), [(0, 4), (5, 6)]);
    }

    #[test]
    fn the_shorter_heading_of_a_pair_is_the_name() {
        let lines = ["## Essentials for staying organized.", "### Free", "$0"];
        assert_eq!(section_heading(&lines, 0, 2).as_deref(), Some("### Free"));
    }

    #[test]
    fn a_bare_heading_names_nothing() {
        let lines = ["###", "## Pro", "$15"];
        assert_eq!(section_heading(&lines, 0, 2).as_deref(), Some("## Pro"));
    }

    #[test]
    fn a_section_with_no_heading_has_no_name() {
        let lines = ["prose", "more prose"];
        assert_eq!(section_heading(&lines, 0, 1), None);
    }

    #[test]
    fn a_page_with_no_lines_has_no_sections() {
        assert!(sections(&[]).is_empty());
    }
}
