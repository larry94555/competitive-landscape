//! What a company is hiring for — the cheapest public signal of where it is investing.
//!
//! The sixth and last question kind, and **the second that runs no model at all**. A careers
//! page is a list of job titles somebody wrote down on purpose. Reading them is transcription,
//! and `ARCHITECTURE.md` §5.4's rule applies exactly as it does to a changelog: where a
//! deterministic parser can produce the fact, nothing a language model could add is worth what
//! it would risk.
//!
//! ```text
//! ## Open roles                          the page says where its list starts
//! Senior / Staff Product Engineer        a title
//! Europe, North America Learn more →     where, and how to apply
//! ```
//!
//! # Why the roles are not sorted into functions
//!
//! The obvious version of this extractor counts *engineering: 7, sales: 3* by matching title
//! words. Three real pages say not to:
//!
//! * Linear files **Product Marketing Manager** under *Product Management* and **Developer
//!   Relations** under *Marketing*. A keyword would put both in the other bucket.
//! * Front's **AI Engineer - GTM / Operations** is filed under *G&A*, not engineering.
//! * Linear's own group labels include **Magic**, which no table of functions would contain.
//!
//! So the page's titles are reported as the page writes them, and the reader does the sorting
//! a keyword would have done worse. On linear.app that reads as *five titles carrying
//! "Engineer", four carrying "Designer", and one "Senior Counsel"* — the investment signal,
//! stated rather than inferred.
//!
//! # Where the list starts, and where it stops
//!
//! **Scoped to the page's own announcement.** All three pages have one — `## Open roles`,
//! `## Open Roles`, `## Open positions` — and everything dangerous is outside it:
//!
//! ```text
//! The Pragmatic Engineer →                              linear.app/careers, a podcast
//! Staff Java Engineer in Brno, Czechia, started in 2015  helpscout.com, an employee
//! Kelsey Weber , Engineering Manager                     front.com, a testimonial byline
//! ```
//!
//! Every one of those is a short line containing a job title, and every one of them would have
//! become an open role on somebody's report. The scan runs from the announcing heading to the
//! next heading at the same level or higher, which is what stops Help Scout's list at
//! *"Our Hiring Process"*.
//!
//! **A page that announces nothing yields nothing.** That is a refusal rather than a fallback,
//! and review is the reason: the first version read such a page whole and let the shape rules
//! below decide, which published *"lists an open role: Kelsey Weber , Engineering Manager"* —
//! a person who already works there. The shape rules clean up *inside* a list somebody has
//! pointed at; they were never strong enough to find one. [`Roles::announced`] carries which
//! silence it was, so a report can say *our gap* rather than *they are hiring nobody*.
//!
//! # What still reaches the scan, and what stops it
//!
//! **Front's list is the last heading on the page**, so the scan runs through its footer, and
//! the shape rules are the only thing between a navigation label and a vacancy. That is not
//! theoretical: the first run over that page reported *"Become a Partner"* — a reseller
//! program — as an open role, and Linear's footer offers `Developers` for the same reason. A
//! footer has no heading to stop at and no marker that says *footer*, so the rules that hold
//! here are the ones about the shape of a job title: [`MIN_TITLE_WORDS`], the full stop, the
//! word cap, and what [`TITLE_WORDS`] does **not** contain. Each is stated where it is enforced,
//! with the line that justifies it, rather than assumed to be enough.

use crate::doc;

/// Bumped whenever these rules change.
pub const HIRING_VERSION: u32 = 1;

/// The most roles taken from one careers page.
///
/// Front lists more than thirty. A report shows the shape of a hiring plan, not a job board,
/// and the number passed over is reported beside the ones kept — `PRODUCT_SPEC.md` §4's
/// coverage note exists so a short list is never mistaken for a small company.
pub const MAX_ROLES: usize = 20;

/// The longest a job title gets before the line is prose.
///
/// Front's filter bar is one line naming six cities and five departments; Help Scout's process
/// section explains what a hiring manager does. Both contain title words, and both are
/// sentences. Real titles on the three frozen pages run to nine words.
const MAX_TITLE_WORDS: usize = 12;

/// The shortest, and it is not one.
///
/// **A line reading `Engineer` names a discipline, not a position.** A report saying *"lists an
/// open role: Engineer"* has told a reader nothing they could act on, and a one-word line
/// carrying a job word is a group label or a navigation link far more often than it is a
/// vacancy. No title on any of the three frozen pages is shorter than two words.
///
/// It used to be justified by `Developers` in Linear's footer. That is no longer the reason —
/// plurals stopped matching, for the reason [`names_a_job`] gives — and the rule was re-argued
/// on its own terms rather than kept because it had once been useful.
const MIN_TITLE_WORDS: usize = 2;

/// The words a job title is built from, matched whole.
///
/// **Not a taxonomy — a filter.** These decide whether a line is a job title at all; what the
/// job *is* stays in the page's own words. Singular only, because a job advertisement names one
/// job — see [`names_a_job`] for what matching plurals cost and bought.
///
/// **Three words are missing on purpose.** `partner`, `associate` and `lead` are job titles in
/// some industries and ordinary marketing nouns in this one: running the scanner over
/// front.com/jobs turned *"Become a Partner"* — a footer link to a reseller program — into an
/// open vacancy. None of the three is needed by any role on the three frozen pages, because a
/// real title carrying them carries another word too: *Lead/Principal Product Manager*.
const TITLE_WORDS: [&str; 31] = [
    "engineer",
    "developer",
    "designer",
    "architect",
    "scientist",
    "analyst",
    "researcher",
    "manager",
    "director",
    "executive",
    "specialist",
    "head",
    "president",
    "officer",
    "counsel",
    "controller",
    "accountant",
    "recruiter",
    "recruiting",
    "marketer",
    "strategist",
    "consultant",
    "representative",
    "advocate",
    "administrator",
    "coordinator",
    "technician",
    "writer",
    "editor",
    "intern",
    "apprentice",
];

/// The headings a careers page uses to announce its list.
///
/// Compared against the heading's whole text, not searched for inside it: Help Scout has a
/// *"View open roles"* button halfway up the page, and jumping the scan there would take in
/// every employee profile between it and the real list.
const ANNOUNCEMENTS: [&str; 10] = [
    "open roles",
    "open positions",
    "open jobs",
    "open opportunities",
    "current openings",
    "job openings",
    "available positions",
    "available roles",
    "our openings",
    "roles",
];

/// One open role, exactly as the page lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Role {
    /// The title, with list markers and heading hashes removed and nothing else changed.
    pub title: String,
    /// The line it was read from, copied verbatim.
    pub quote: String,
    pub at_line: usize,
}

/// What a careers page yielded.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Roles {
    pub roles: Vec<Role>,
    /// How many distinct roles the page listed before [`MAX_ROLES`] was applied.
    pub considered: usize,
    /// Whether the page said where its list of roles starts.
    ///
    /// **`false` means nothing was read, and it is the only reason [`roles`] can be empty
    /// without the page being empty.** It used to mean *"the whole page was read instead"*,
    /// which is what review found publishing a testimonial byline as a vacancy.
    ///
    /// Kept, rather than folded into an empty list, because the two silences are different
    /// facts about a company and `Coverage` exists to tell them apart: *this careers page
    /// advertises nothing* is news, and *we could not tell which part of this page is the
    /// list* is our gap.
    ///
    /// [`roles`]: Self::roles
    pub announced: bool,
}

/// Every open role the page lists, in the order it lists them.
///
/// One entry per title, not per listing: Front advertises *Security Operations Engineer
/// (Contractor)* three times, once per location, and a report that said so three times would be
/// describing a job board rather than a company.
///
/// **A page that never says where its list is yields nothing at all.** See [`Roles::announced`].
#[must_use]
pub fn every_role(markdown: &str) -> Roles {
    let lines: Vec<&str> = markdown.lines().collect();
    let Some((from, to)) = listing(&lines) else {
        // **Not a cautious version of reading the page — a refusal to read it.** Review found
        // what the page-wide scan published: `Kelsey Weber , Engineering Manager` is a
        // testimonial byline on front.com, and it is five words, has no full stop, and carries
        // a job word, so every shape rule below says yes. Off a page that announces its list,
        // that line is scoped away. Off a page that does not, nothing stands between a current
        // employee and a sentence saying the company is hiring them.
        //
        // The shape rules were never strong enough to work unscoped. They were built to clean
        // up *inside* a list somebody had already pointed at, and using them as the only
        // defense was reading a run-log line — *"the page named no list"* — as if it were a
        // safeguard. It is not: nothing carried it into the report.
        return Roles::default();
    };

    // **Read the form the page wrote its list in.** A hosted board puts its roles in bullets
    // and its furniture — *Create a Job Alert*, a job count, a department name — in the prose
    // and headings between them; a company's own careers page usually does the opposite, with
    // its roles as plain lines and a bulleted footer underneath. Neither *"prefer bullets"* nor
    // *"ignore bullets"* is right, and each publishes the other page's furniture as a vacancy.
    //
    // Whichever form carries more of the titles is the list. A tie keeps the plain lines, which
    // is what this did before a board was ever read.
    let bulleted = form_of_the_list(&lines[from..to]);

    let mut roles: Vec<Role> = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    for (at, line) in lines.iter().enumerate().take(to).skip(from) {
        if doc::is_list_item(line) != bulleted {
            continue;
        }
        // **A heading is structure, not an entry** — the same rule the form is counted by, and
        // review found it applied only there: with the roles as plain lines, `### Account
        // Executive` above them survived the filter above and `title_on` strips the hashes, so a
        // group label was reported as a vacancy. It votes for nothing and it is nothing.
        if !bulleted && doc::heading_level(line).is_some() {
            continue;
        }
        let Some(title) = title_on(line) else {
            continue;
        };
        let key = title.to_lowercase();
        if seen.contains(&key) {
            continue;
        }
        seen.push(key);
        roles.push(Role {
            title,
            quote: (*line).trim().to_owned(),
            at_line: at,
        });
    }

    let considered = roles.len();
    roles.truncate(MAX_ROLES);
    Roles {
        roles,
        considered,
        announced: true,
    }
}

/// Whether the list is written as bullets, decided by counting rather than by assuming.
///
/// See the note in [`every_role`]. Only lines that would be titles are counted, so a footer of
/// one-word links weighs nothing against a page of vacancies.
///
/// **A heading is structure, not an entry**, and review found what counting one costs: a board
/// advertising a single vacancy is `### Account Executive` above `- Staff Product Engineer`, so
/// the count ties at one apiece, the tie keeps the plain lines, and the group label is published
/// as the opening while the job is discarded. Neither page shape puts its roles in headings —
/// a board's are bullets and a company's own are plain lines under one — so a heading votes for
/// nothing.
///
/// A page with forty bulleted vacancies and forty bulleted footer links is still not one this
/// can tell apart; a page with forty of one and two of the other is.
fn form_of_the_list(range: &[&str]) -> bool {
    let (mut bulleted, mut plain) = (0usize, 0usize);
    for line in range {
        if title_on(line).is_none() {
            continue;
        }
        if doc::is_list_item(line) {
            bulleted += 1;
        } else if doc::heading_level(line).is_none() {
            plain += 1;
        }
    }
    bulleted > plain
}

/// The half-open line range holding the list, or `None` when the page never says.
fn listing(lines: &[&str]) -> Option<(usize, usize)> {
    let (at, level) = lines.iter().enumerate().find_map(|(i, line)| {
        let level = doc::heading_level(line)?;
        announces(doc::heading_text(line)).then_some((i, level))
    })?;

    // To the next heading that is not *inside* this section. Help Scout's list ends at
    // `## Our Hiring Process`, whose next paragraphs describe what a hiring manager does.
    let end = lines
        .iter()
        .enumerate()
        .skip(at + 1)
        .find(|(_, line)| doc::heading_level(line).is_some_and(|found| found <= level))
        .map_or(lines.len(), |(i, _)| i);
    Some((at + 1, end))
}

/// Whether a heading is the page saying *the list starts here*.
fn announces(heading: &str) -> bool {
    let text = heading
        .trim()
        // `\u{2060}` is a word joiner, and it is not decoration: Linear ends every group label
        // on its careers page with one. Written as an escape because an invisible character in
        // source is a character nobody can review.
        .trim_end_matches([':', '↓', '→', '\u{2060}'])
        .trim()
        .to_lowercase();
    // Equality, plus the one possessive form a hosted board uses: Greenhouse titles Vercel's
    // page *Current openings at Vercel*. Deliberately not `starts_with(phrase)` on its own —
    // that would let *"open roles are what we are proudest of"* announce a list that is not
    // there, which is the whole reason this is an exact match in the first place.
    ANNOUNCEMENTS
        .iter()
        .any(|phrase| text == *phrase || text.starts_with(&format!("{phrase} at ")))
}

/// The job title a line carries, if it is a listing rather than a sentence about one.
fn title_on(line: &str) -> Option<String> {
    let text = strip_markers(line);
    if text.is_empty() {
        return None;
    }
    // A sentence ends. A title does not. This is what separates Front's *"Browse our open
    // positions and find your dream job."* from the titles directly under it, and it is the
    // same rule that keeps a date inside a sentence out of a changelog.
    if text.ends_with(['.', '?', '!', ',', ';']) {
        return None;
    }
    let words = text.split_whitespace().count();
    if !(MIN_TITLE_WORDS..=MAX_TITLE_WORDS).contains(&words) {
        return None;
    }
    names_a_job(text).then(|| text.to_owned())
}

/// Whether the line names a job, on whole words.
///
/// `Engineering` is a group label and `engineer` is a title word; matching on boundaries is
/// what keeps Linear's own section headings out of its list of vacancies.
///
/// **Plurals are deliberately not matched, and the mutation harness is why.** The first version
/// stepped over a trailing `s`, on the reasoning that a page might write *Engineers*. Putting
/// that rule back as a defect broke no test, and looking for the one to write showed there was
/// none to write: no title on any of the three frozen pages is plural, a job advertisement
/// names one job, and every line the rule *did* reach — `Developers`, `Partners`, `Managers` —
/// was a navigation label. It widened the false-positive surface to buy nothing.
fn names_a_job(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    TITLE_WORDS.iter().any(|word| {
        let mut from = 0usize;
        while let Some(offset) = lower[from..].find(word) {
            let start = from + offset;
            let end = start + word.len();
            from = end;
            if bounded(&lower, start, end) {
                return true;
            }
        }
        false
    })
}

/// Whether a match sits on word boundaries.
///
/// Hyphens and slashes count as boundaries: `Senior / Staff Product Engineer` and
/// `AI Engineer - GTM / Operations` are both real titles.
fn bounded(text: &str, start: usize, end: usize) -> bool {
    let before = text[..start].chars().next_back();
    let after = text[end..].chars().next();
    let open = |c: Option<char>| c.is_none_or(|c| !c.is_alphanumeric());
    open(before) && open(after)
}

/// List markers and heading hashes, removed so every shape reads the same.
fn strip_markers(line: &str) -> &str {
    let no_heading = doc::heading_text(line);
    let trimmed = no_heading.trim_start();
    let no_bullet = trimmed
        .strip_prefix("- ")
        .or_else(|| trimmed.strip_prefix("* "))
        .or_else(|| trimmed.strip_prefix("+ "))
        .unwrap_or(trimmed);
    no_bullet.trim()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn titles(markdown: &str) -> Vec<String> {
        every_role(markdown)
            .roles
            .into_iter()
            .map(|r| r.title)
            .collect()
    }

    #[test]
    fn a_title_under_the_announcement_is_a_role() {
        let page = "## Open roles\nSenior / Staff Product Engineer\nEurope, North America";
        assert_eq!(titles(page), ["Senior / Staff Product Engineer"]);
    }

    #[test]
    fn a_podcast_named_after_a_job_is_not_a_vacancy() {
        // linear.app/careers, verbatim, forty lines above `## Open roles`. A short line, a
        // capitalized job word, no full stop — every shape rule says yes, and it is a podcast.
        let page = "The Pragmatic Engineer\n## Open roles\nProduct Engineer\nNorth America";
        assert_eq!(titles(page), ["Product Engineer"]);
    }

    #[test]
    fn an_employee_is_not_an_opening() {
        // helpscout.com/company/careers, above the list: a profile of somebody already there.
        let page = "Staff Java Engineer in Brno, Czechia, started in 2015\n## Open Roles\nSr. Software Engineer, Platform";
        assert_eq!(titles(page), ["Sr. Software Engineer, Platform"]);
    }

    #[test]
    fn a_testimonial_byline_is_not_an_opening() {
        // front.com/jobs, above the list.
        let page = "Kelsey Weber , Engineering Manager\n## Open positions\nStaff Data Engineer";
        assert_eq!(titles(page), ["Staff Data Engineer"]);
    }

    #[test]
    fn the_list_stops_at_the_next_section() {
        // Help Scout's roles are followed by `## Our Hiring Process`, which explains what a
        // hiring manager does. Without the stop the process description is read as vacancies.
        let page = "## Open Roles\nSr. Product Engineer, Agents\n## Our Hiring Process\nYou will meet the hiring manager and a senior engineer\nAnother recruiter joins later";
        assert_eq!(titles(page), ["Sr. Product Engineer, Agents"]);
    }

    #[test]
    fn a_deeper_heading_inside_the_list_does_not_stop_it() {
        // Front groups its list under `### G&A` and `### EPD` beneath an `## Open positions`.
        let page =
            "## Open positions\n### G&A\nStaff Data Engineer\n### EPD\nSenior Design Engineer";
        assert_eq!(
            titles(page),
            ["Staff Data Engineer", "Senior Design Engineer"]
        );
    }

    #[test]
    fn a_sentence_about_the_list_is_not_in_it() {
        // front.com/jobs, the line directly under the heading. **It is the word list that
        // rejects this one**, not the full stop — `positions` and `job` are not title words —
        // which the mutation harness pointed out by deleting the full-stop rule and watching
        // this pass anyway. The test below is the one that holds that rule.
        let page = "## Open positions\nBrowse our open positions and find your dream job.\nStaff Data Engineer";
        assert_eq!(titles(page), ["Staff Data Engineer"]);
    }

    #[test]
    fn a_short_sentence_carrying_a_job_word_is_still_a_sentence() {
        // Not from a frozen page, and that is the point: all three write their prose long
        // enough that `MAX_TITLE_WORDS` catches it first. This is the shape only the full stop
        // catches — short enough to be a title, and a sentence about somebody who already
        // works there.
        let page = "## Open roles\nMeet our new Head of Design.\nSenior Design Engineer";
        assert_eq!(titles(page), ["Senior Design Engineer"]);
    }

    #[test]
    fn a_bare_job_word_on_its_own_line_is_a_label_and_not_a_vacancy() {
        // What `MIN_TITLE_WORDS` holds now that plurals no longer match: a line reading
        // `Engineer` names a discipline, and a report saying *"lists an open role: Engineer"*
        // has told a reader nothing they could act on.
        let page = "## Open roles\nEngineer\nProduct Engineer\nDesigner";
        assert_eq!(titles(page), ["Product Engineer"]);
    }

    #[test]
    fn a_filter_bar_naming_every_department_is_not_a_role() {
        // front.com/jobs, one line, and it contains no title word — but it is also far longer
        // than any title, which is the rule that has to hold when a page names a department
        // like `Engineering Operations`.
        let page = "## Open positions\nAll locations Chicago, IL Dublin, Ireland Paris, France Remote, Argentina San Francisco, CA All departments EPD Marketing Sales Engineering Manager\nStaff Data Engineer";
        assert_eq!(titles(page), ["Staff Data Engineer"]);
    }

    #[test]
    fn a_hosted_boards_heading_announces_its_list_and_a_sentence_does_not() {
        // Greenhouse titles Vercel's board *Current openings at Vercel*, which an exact match
        // misses. Loosening it to `starts_with` would let the sentence below announce a list
        // that is not there, so the one possessive form is all that was added.
        let board = "# Current openings at Vercel
- Staff Engineer, Runtime";
        assert_eq!(titles(board), ["Staff Engineer, Runtime"]);

        let prose = "## Open roles are what we are proudest of
Meet the team
Staff Engineer";
        assert!(
            titles(prose).is_empty(),
            "a sentence beginning with an announcement announced a list"
        );
    }

    #[test]
    fn the_form_of_the_list_is_counted_rather_than_assumed() {
        // **Two real pages, the opposite way round.** A hosted board puts its roles in bullets
        // and its furniture in prose; a company's own careers page does the reverse. Whichever
        // form carries more of the titles is the list.
        let board = concat!(
            "# Current openings at Acme
",
            "Create a Job Alert
",
            "### Account Executive
",
            "- Senior Product Engineer
",
            "- Staff Product Designer
",
            "- Engineering Manager, Growth
",
        );
        assert_eq!(
            titles(board),
            [
                "Senior Product Engineer",
                "Staff Product Designer",
                "Engineering Manager, Growth"
            ],
            "the board's furniture outvoted its vacancies"
        );

        let own_page = concat!(
            "## Open roles
",
            "Senior Product Engineer
",
            "Staff Product Designer
",
            "Engineering Manager, Growth
",
            "- Support Engineer Handbook
",
        );
        assert_eq!(
            titles(own_page),
            [
                "Senior Product Engineer",
                "Staff Product Designer",
                "Engineering Manager, Growth"
            ],
            "one bulleted footer link outvoted three vacancies"
        );
    }

    #[test]
    fn a_board_advertising_one_job_reports_the_job_and_not_its_group() {
        // **Review found this.** One vacancy under one group label ties the count at one
        // apiece, and a tie keeps the plain lines — so the label was published as the opening
        // and the job discarded. A company with a single vacancy is the most likely board there
        // is, and `Account Executive` is furniture the tests above already establish.
        let one = "# Current openings at Acme
### Account Executive
- Staff Product Engineer";
        assert_eq!(titles(one), ["Staff Product Engineer"]);

        // Still true when the group label is the only other candidate on a longer board.
        let few = concat!(
            "# Current openings at Acme
",
            "### Account Executive
",
            "- Staff Product Engineer
",
            "### Design
",
            "- Senior Product Designer
",
        );
        assert_eq!(
            titles(few),
            ["Staff Product Engineer", "Senior Product Designer"]
        );
    }

    #[test]
    fn a_group_heading_that_looks_like_a_title_is_still_a_heading() {
        // **Review's second pass on the same rule.** Excluding headings from the *vote* was not
        // enough: with the roles as plain lines the heading survives the form filter, and
        // `title_on` strips hashes, so `### Account Executive` was reported as a vacancy beside
        // the real one. `Engineering` is caught by the one-word rule and proves nothing here.
        let titled = "## Open roles
### Account Executive
Staff Product Engineer";
        assert_eq!(titles(titled), ["Staff Product Engineer"]);

        let one_word = "## Open roles
### Engineering
Product Engineer
Mobile Product Designer";
        assert_eq!(
            titles(one_word),
            ["Product Engineer", "Mobile Product Designer"]
        );
    }

    #[test]
    fn only_lines_that_could_be_titles_decide_the_form() {
        // Furniture is what a board has most of, and counting it would hand the decision to
        // whichever form the page happened to write its scaffolding in.
        let page = concat!(
            "## Open roles
",
            "- Read our engineering blog.
",
            "- See the handbook.
",
            "- Browse by team.
",
            "- Sort by location.
",
            "Senior Product Engineer
",
            "Staff Product Designer
",
        );
        assert_eq!(
            titles(page),
            ["Senior Product Engineer", "Staff Product Designer"],
            "four bulleted sentences outvoted two vacancies"
        );
    }

    #[test]
    fn a_list_written_with_asterisks_is_still_a_list() {
        // Markdown has three bullet markers and a converter picks whichever it likes. Knowing
        // only `-` would read an asterisked board as prose and publish its furniture.
        let page = concat!(
            "# Current openings at Acme
",
            "Account Executive
",
            "* Senior Product Engineer
",
            "* Staff Product Designer
",
        );
        assert_eq!(
            titles(page),
            ["Senior Product Engineer", "Staff Product Designer"]
        );
    }

    #[test]
    fn a_group_label_is_not_a_role() {
        // linear.app/careers labels its groups `Engineering`, `Sales`, `Design`. `Engineering`
        // contains `engineer` and is not a vacancy.
        let page = "## Open roles\nEngineering\nProduct Engineer\nDesign\nMobile Product Designer";
        assert_eq!(
            titles(page),
            ["Product Engineer", "Mobile Product Designer"]
        );
    }

    #[test]
    fn a_one_word_navigation_label_is_not_a_role() {
        // `- Developers` sits in linear.app's footer under Resources, inside the scan because
        // that page has no second-level heading after its list.
        let page = "## Open roles\nProduct Engineer\n### Resources\n- Developers\n- Status";
        assert_eq!(titles(page), ["Product Engineer"]);
    }

    #[test]
    fn a_plural_job_word_is_a_group_of_people_and_not_a_vacancy() {
        // The rule the mutation harness deleted for free. Every line here has two words, so
        // `MIN_TITLE_WORDS` does not reach them — matching plurals is the only thing that
        // would have made any of them a job, and none of them is one.
        let page =
            "## Open roles\nProduct Engineer\nMeet our Engineers\n- Our Partners\n- For Developers";
        assert_eq!(titles(page), ["Product Engineer"]);
    }

    #[test]
    fn the_same_title_written_two_ways_is_one_role() {
        // Cheap insurance rather than an observed page: a title in a heading and again in a
        // list is one vacancy however either was capitalized, and reporting it twice would
        // describe a company hiring two people.
        let page = "## Open roles\n### Staff Data Engineer\n- staff data engineer";
        assert_eq!(titles(page).len(), 1);
    }

    #[test]
    fn a_reseller_program_is_not_a_vacancy() {
        // front.com/jobs, in the footer, inside the scan because that page's list is its last
        // heading. The first run over the real page reported this as an open role.
        let page =
            "## Open positions\nStaff Data Engineer\nCompany\n- Become a Partner\n- Partners";
        assert_eq!(titles(page), ["Staff Data Engineer"]);
    }

    #[test]
    fn a_real_title_carrying_a_dropped_word_is_still_read() {
        // `lead`, `partner` and `associate` are not title words here. Help Scout's
        // `Lead/Principal Product Manager` has to survive that, and it does — on `manager`.
        let page =
            "## Open roles\nLead/Principal Product Manager, Growth\nAssociate General Counsel";
        assert_eq!(
            titles(page),
            [
                "Lead/Principal Product Manager, Growth",
                "Associate General Counsel"
            ]
        );
    }

    #[test]
    fn the_same_role_listed_once_per_location_is_one_role() {
        // front.com/jobs advertises this one three times, once per city.
        let page = "## Open positions\nSecurity Operations Engineer (Contractor)\nRemote, Argentina\nSecurity Operations Engineer (Contractor)\nSantiago, Chile\nSecurity Operations Engineer (Contractor)\nParis, France";
        assert_eq!(titles(page).len(), 1);
    }

    #[test]
    fn a_page_that_announces_nothing_is_not_read_at_all() {
        // **Review's reproduction, and it is the whole reason this is a refusal.** Every shape
        // rule says yes to a testimonial byline: five words, no full stop, a job word on a
        // boundary. Off an announced list it is scoped away; off an unannounced page it used to
        // reach the report as `lists an open role: Kelsey Weber , Engineering Manager` at high
        // confidence — a person who already works there, advertised as a vacancy.
        let found = every_role(
            "Why we love working here\nKelsey Weber , Engineering Manager\nWe hire thoughtfully.",
        );
        assert!(found.roles.is_empty(), "{:?}", found.roles);
        assert_eq!(found.considered, 0);
        assert!(!found.announced, "and the report has to know which silence");
    }

    #[test]
    fn a_real_vacancy_on_an_unannounced_page_is_lost_too_and_that_is_the_trade() {
        // Stated rather than hidden: the refusal costs real roles on a page whose heading this
        // list does not recognize. A missed vacancy is a thin section; an invented one is a
        // sentence about a named person that is not true.
        let found = every_role("# Careers\nSenior Software Engineer\nRemote");
        assert!(found.roles.is_empty());
        assert!(!found.announced);
    }

    #[test]
    fn an_announced_page_is_read_and_says_so() {
        let scoped = every_role("## Open roles\nSenior Software Engineer");
        assert_eq!(scoped.roles.len(), 1);
        assert!(scoped.announced);
    }

    #[test]
    fn a_button_is_not_the_announcement() {
        // helpscout.com has `View open roles` halfway up the page. Starting there would take
        // in every employee profile between it and the real list.
        assert!(!announces("View open roles"));
        assert!(announces("Open Roles"));
        assert!(announces("Open roles ↓"));
    }

    #[test]
    fn the_cap_is_reported_rather_than_applied_silently() {
        let mut page = String::from("## Open positions\n");
        for i in 0..MAX_ROLES + 7 {
            page.push_str(&format!("Senior Software Engineer, Team {i}\n"));
        }
        let found = every_role(&page);
        assert_eq!(found.roles.len(), MAX_ROLES);
        assert_eq!(found.considered, MAX_ROLES + 7);
    }

    #[test]
    fn a_careers_page_with_nothing_open_yields_nothing() {
        // The honest case, and the one a company in a hiring freeze produces.
        let page =
            "## Open roles\nWe have no open roles right now.\nSubmit your resume to keep in touch.";
        assert!(every_role(page).roles.is_empty());
    }

    #[test]
    fn future_openings_is_not_a_role() {
        // helpscout.com ends its list with it. Two words, no full stop, and no job in it.
        let page = "## Open Roles\nFuture Openings\nSubmit your resume to keep in touch.";
        assert!(every_role(page).roles.is_empty());
    }

    #[test]
    fn a_title_keeps_the_spelling_the_page_gave_it() {
        let page = "## Open roles\n- Sr. Product Engineer, Agents";
        let found = every_role(page);
        assert_eq!(found.roles[0].title, "Sr. Product Engineer, Agents");
        assert_eq!(found.roles[0].quote, "- Sr. Product Engineer, Agents");
    }

    #[test]
    fn an_empty_page_does_not_panic() {
        assert!(every_role("").roles.is_empty());
        assert!(every_role("\n\n").roles.is_empty());
        assert!(every_role("## Open roles").roles.is_empty());
    }

    #[test]
    fn a_line_whose_lowercase_is_longer_than_itself_is_not_a_crash() {
        // The same shape that crashed the assurance scanner: `İ` lowercases to two characters
        // under Unicode rules, so an offset from the lowered copy points past the end of the
        // original. The fold here is ASCII for the same reason.
        let page = "## Open roles\nİstanbul Senior Engineer";
        let _ = every_role(page);
    }
}
