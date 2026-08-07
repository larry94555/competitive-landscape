//! Real pages, frozen, and what the parsers must make of them **with no model running**.
//!
//! # Why this exists beside the model-facing set
//!
//! `BENCHMARKS.md` Runs 5 to 16 found sixteen defects. Every one of them was found the same
//! way: point the pipeline at a real company, read the output, notice something wrong. Not one
//! was found by a test.
//!
//! That method works and does not scale — it needs a `llama-server`, several minutes, and
//! somebody willing to read nine sections of output carefully. **The half of it that needs
//! none of those things is this file.** Span selection, capability windows, date parsing and
//! identity windows are deterministic: given the same page they produce the same answer for
//! ever, so the answer can be written down and checked on every pull request.
//!
//! The subjects in `subjects/` measure a *model*. These measure *us*.
//!
//! # Why the pages are Markdown rather than HTML
//!
//! What every extractor consumes is the converted Markdown, and freezing that is a tenth of
//! the bytes and legible in a diff — when an expectation changes, the reason is visible beside
//! it. The conversion itself is covered by unit tests in `landscape-extract::markdown`,
//! including the one that mattered: a page that was already Markdown arriving as a single
//! 2,167-word line.
//!
//! # An expectation may be wrong on purpose
//!
//! [`PageExpectation::known_wrong`] records where the frozen answer is **what we currently do
//! rather than what we should do** — `notion-pricing` counts an add-on as a plan, and Run 7
//! says so. Freezing it silently would turn a known defect into a specification; freezing it
//! with the reason attached means the day somebody fixes it, the test tells them which line to
//! delete.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Which question the page answers, and therefore which extractor is asserted.
///
/// A pricing page run through the capability extractor produces windows for its FAQ headings.
/// Nothing runs that combination — discovery labels each page and `landscape-analyze` reads the
/// label — so asserting it would freeze behaviour nobody depends on and break tests for
/// improvements nobody asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Answers {
    Pricing,
    Features,
    Changes,
    Identity,
    Trust,
    Direction,
}

/// What the deterministic passes must produce for one frozen page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageExpectation {
    /// The file in `pages/`, without a path.
    pub page: String,
    /// Where it came from and when. Frozen: never re-fetched, because a page that changes
    /// under a test turns a red build into a shrug.
    pub from: String,
    /// Which finding this page carries, in prose. Required, for the same reason
    /// [`crate::Subject::why`] is: a page nobody can justify is a page that gets deleted the
    /// first time it fails.
    pub why: String,
    pub answers: Answers,
    /// Where the frozen answer is not the right answer, and why we keep it anyway.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub known_wrong: Option<String>,

    /// Every plan window, in page order. `Answers::Pricing`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan_windows: Option<Vec<Window>>,
    /// The capability windows, and how many the page offered. `Answers::Features`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<Capabilities>,
    /// Dated entries. `Answers::Changes`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub changes: Option<Changes>,
    /// The window read for each fact the page states. `Answers::Identity`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub facts: Option<Vec<FactWindow>>,
    /// Each standard the page names, and how many it named. `Answers::Trust`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assurances: Option<Assurances>,
    /// Every open role the page lists, and how many it listed. `Answers::Direction`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub openings: Option<Openings>,
}

/// One window, heading and body.
///
/// # Why the body is here and not just the heading
///
/// The first version of this file froze only headings, and review found the hole in one
/// try: changing Linear Business from `$16 per user/month` to `$999` in the frozen page left
/// all five tests green. **The window is the product** — §5.4's whole argument is that a bad
/// window and a bad model are indistinguishable at the output — so freezing which heading won
/// and not what came with it asserts the cheaper half of the thing that matters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Window {
    /// The heading the window sits under. Empty when it has none.
    pub heading: String,
    /// The body, one entry per line, so a change shows up as a changed line in review rather
    /// than as one altered character inside a long escaped string.
    pub text: Vec<String>,
}

/// What a features page yields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Capabilities {
    /// Each window, in page order.
    pub named: Vec<Window>,
    /// How many sections looked like capabilities before the cap. Asserted because a cap
    /// nobody counts is a cap nobody notices growing.
    pub considered: usize,
}

/// What a changelog yields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Changes {
    /// Every entry read, after the cap, in page order.
    pub entries: Vec<Dated>,
    /// Dated entries on the page, before the cap.
    pub considered: usize,
}

/// One dated entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dated {
    /// `YYYY-MM-DD`.
    pub date: String,
    /// The heading or text beside the date. Empty when the page dated something untitled.
    pub title: String,
    /// The line the date was read from. Frozen because it is what a reader is shown as
    /// evidence, and a title can stay right while the quote behind it drifts.
    pub quote: String,
}

/// One identity fact and the window it was read from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FactWindow {
    /// `founded`, `headquarters` or `employees`.
    pub fact: String,
    /// The window, one entry per line. The fact is only as good as what it was read from —
    /// Run 12's finding was a window, not a parser.
    pub text: Vec<String>,
}

/// What a trust page yields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Assurances {
    /// Each standard, in page order, with the window it was read from.
    pub named: Vec<NamedStandard>,
    /// How many the page named before the cap.
    pub considered: usize,
}

/// One standard and the words around it.
///
/// The window is frozen for the same reason a plan window is: **the standard is only as good as
/// what it was read from.** A scanner that finds `SOC 2` and hands the model a window from the
/// wrong paragraph produces a confident answer about a different sentence, and freezing only
/// the name would assert the cheaper half.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NamedStandard {
    /// As the page spells it.
    pub standard: String,
    /// The window, one entry per line.
    pub text: Vec<String>,
}

impl From<landscape_extract::assurance::Named> for NamedStandard {
    fn from(found: landscape_extract::assurance::Named) -> Self {
        Self {
            standard: found.standard,
            text: found.span.text.lines().map(str::to_owned).collect(),
        }
    }
}

/// What a careers page yields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Openings {
    /// Each role, in page order, after the cap.
    pub roles: Vec<Opening>,
    /// Distinct roles the page listed, before the cap.
    pub considered: usize,
    /// Whether the page announced where its list starts.
    ///
    /// Frozen because it decides how much of the page was read, and a change from `true` to
    /// `false` would quietly widen the scan to a footer without changing a single role — the
    /// half of this extractor that a list of titles cannot show.
    pub announced: bool,
}

/// One open role.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Opening {
    /// The title, as the page writes it.
    pub title: String,
    /// The line it was read from. Frozen for the same reason a change's quote is: a title can
    /// stay right while the evidence under it drifts to another line.
    pub quote: String,
}

impl From<&landscape_extract::hiring::Role> for Opening {
    fn from(role: &landscape_extract::hiring::Role) -> Self {
        Self {
            title: role.title.clone(),
            quote: role.quote.clone(),
        }
    }
}

/// Where the frozen pages live.
#[must_use]
pub fn pages_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("pages")
}

/// Load every expectation, in filename order.
///
/// # Errors
/// If the directory cannot be read, or any file is not a valid [`PageExpectation`], or names a
/// page that is not there.
pub fn load() -> Result<Vec<PageExpectation>, String> {
    let dir = pages_dir();
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&dir)
        .map_err(|e| format!("cannot read {}: {e}", dir.display()))?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|p| p.to_string_lossy().ends_with(".expected.json"))
        .collect();
    paths.sort();

    paths
        .iter()
        .map(|path| {
            let text = std::fs::read_to_string(path)
                .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
            let expectation: PageExpectation = serde_json::from_str(&text)
                .map_err(|e| format!("{} is not a valid expectation: {e}", path.display()))?;
            if !dir.join(&expectation.page).is_file() {
                return Err(format!(
                    "{} expects {}, which is not in pages/",
                    path.display(),
                    expectation.page
                ));
            }
            Ok(expectation)
        })
        .collect()
}

impl PageExpectation {
    /// The frozen page itself.
    ///
    /// # Errors
    /// If it cannot be read.
    pub fn markdown(&self) -> Result<String, String> {
        std::fs::read_to_string(pages_dir().join(&self.page))
            .map_err(|e| format!("cannot read {}: {e}", self.page))
    }

    /// Everything about this page that does not match, in the words a reader needs.
    ///
    /// Returns the differences rather than asserting them, so one run reports every page
    /// rather than stopping at the first — a change to a shared rule usually moves several,
    /// and seeing which ones is the diagnosis.
    ///
    /// # Errors
    /// If the page cannot be read.
    pub fn differences(&self) -> Result<Vec<String>, String> {
        let markdown = self.markdown()?;
        let mut out = Vec::new();

        if let Some(expected) = &self.plan_windows {
            let got: Vec<Window> = landscape_extract::span::every_plan(&markdown)
                .into_iter()
                .map(Window::from)
                .collect();
            compare_windows("plan windows", expected, &got, &mut out);
        }

        if let Some(expected) = &self.capabilities {
            let found = landscape_extract::capability::every_capability(&markdown);
            let named: Vec<Window> = found.windows.into_iter().map(Window::from).collect();
            compare_windows("capabilities", &expected.named, &named, &mut out);
            if found.considered != expected.considered {
                out.push(format!(
                    "capabilities considered: expected {}, got {}",
                    expected.considered, found.considered
                ));
            }
        }

        if let Some(expected) = &self.changes {
            let found = landscape_extract::changes::every_change(&markdown);
            let got: Vec<Dated> = found.entries.iter().map(Dated::from).collect();
            if got.len() != expected.entries.len() {
                out.push(format!(
                    "changes read: expected {}, got {}",
                    expected.entries.len(),
                    got.len()
                ));
            }
            for (i, (want, have)) in expected.entries.iter().zip(&got).enumerate() {
                if want != have {
                    out.push(format!(
                        "change {i}\n  expected {} | {} | {}\n  got      {} | {} | {}",
                        want.date, want.title, want.quote, have.date, have.title, have.quote
                    ));
                }
            }
            if found.considered != expected.considered {
                out.push(format!(
                    "changes considered: expected {}, got {}",
                    expected.considered, found.considered
                ));
            }
        }

        if let Some(expected) = &self.assurances {
            let found = landscape_extract::assurance::every_assurance(&markdown);
            let got: Vec<NamedStandard> =
                found.named.into_iter().map(NamedStandard::from).collect();
            if got.len() != expected.named.len() {
                out.push(format!(
                    "standards named: expected {}, got {}",
                    expected.named.len(),
                    got.len()
                ));
            }
            for (i, (want, have)) in expected.named.iter().zip(&got).enumerate() {
                if want.standard != have.standard {
                    out.push(format!(
                        "standard {i}: expected {}, got {}",
                        want.standard, have.standard
                    ));
                }
                // The window too, for the same reason the plan windows are frozen: a name read
                // from the wrong paragraph is a confident answer about a different sentence.
                if want.text != have.text {
                    out.push(format!(
                        "standard {i} ({}) window\n  expected {:?}\n  got      {:?}",
                        want.standard, want.text, have.text
                    ));
                }
            }
            if found.considered != expected.considered {
                out.push(format!(
                    "standards considered: expected {}, got {}",
                    expected.considered, found.considered
                ));
            }
        }

        if let Some(expected) = &self.openings {
            let found = landscape_extract::hiring::every_role(&markdown);
            let got: Vec<Opening> = found.roles.iter().map(Opening::from).collect();
            if got.len() != expected.roles.len() {
                out.push(format!(
                    "roles read: expected {}, got {}",
                    expected.roles.len(),
                    got.len()
                ));
            }
            for (i, (want, have)) in expected.roles.iter().zip(&got).enumerate() {
                if want != have {
                    out.push(format!(
                        "role {i}\n  expected {} | {}\n  got      {} | {}",
                        want.title, want.quote, have.title, have.quote
                    ));
                }
            }
            if found.considered != expected.considered {
                out.push(format!(
                    "roles considered: expected {}, got {}",
                    expected.considered, found.considered
                ));
            }
            if found.announced != expected.announced {
                out.push(format!(
                    "the page announcing its list: expected {}, got {}",
                    expected.announced, found.announced
                ));
            }
        }

        if let Some(expected) = &self.facts {
            let got: Vec<FactWindow> = landscape_extract::identity::every_fact(&markdown)
                .into_iter()
                .map(|(fact, span)| FactWindow {
                    fact: fact.name().to_owned(),
                    text: lines_of(&span.text),
                })
                .collect();
            let names = |ws: &[FactWindow]| ws.iter().map(|w| w.fact.clone()).collect::<Vec<_>>();
            if names(expected) != names(&got) {
                out.push(format!(
                    "identity facts\n  expected {:?}\n  got      {:?}",
                    names(expected),
                    names(&got)
                ));
            }
            for (want, have) in expected.iter().zip(&got) {
                if want.fact == have.fact && want.text != have.text {
                    out.push(format!(
                        "the window read for `{}`\n{}",
                        want.fact,
                        first_difference(&want.text, &have.text)
                    ));
                }
            }
        }

        Ok(out)
    }
}

impl From<landscape_extract::span::Span> for Window {
    fn from(span: landscape_extract::span::Span) -> Self {
        Self {
            heading: span.heading.unwrap_or_default(),
            text: lines_of(&span.text),
        }
    }
}

impl
    From<(
        landscape_extract::identity::Fact,
        landscape_extract::span::Span,
    )> for FactWindow
{
    fn from(
        (fact, span): (
            landscape_extract::identity::Fact,
            landscape_extract::span::Span,
        ),
    ) -> Self {
        Self {
            fact: fact.name().to_owned(),
            text: lines_of(&span.text),
        }
    }
}

impl From<&landscape_extract::changes::Entry> for Dated {
    fn from(entry: &landscape_extract::changes::Entry) -> Self {
        Self {
            date: format!("{:04}-{:02}-{:02}", entry.year, entry.month, entry.day),
            title: entry.title.clone(),
            quote: entry.quote.clone(),
        }
    }
}

/// A window's body as lines, with trailing blanks dropped.
///
/// Stored a line at a time rather than as one escaped string so that a change to a window shows
/// up in review as a changed *line*.
fn lines_of(text: &str) -> Vec<String> {
    let mut lines: Vec<String> = text.lines().map(str::to_owned).collect();
    while lines.last().is_some_and(|l| l.trim().is_empty()) {
        lines.pop();
    }
    lines
}

/// Which windows differ, and where — headings first, then the body of each.
///
/// Headings are reported as a list because a change to the *selection* moves several at once and
/// the shape of the list is the diagnosis. Bodies are reported one differing line at a time,
/// because a window is 400 tokens and printing two of them per failure buries the answer.
fn compare_windows(what: &str, expected: &[Window], got: &[Window], out: &mut Vec<String>) {
    let headings = |ws: &[Window]| ws.iter().map(|w| w.heading.clone()).collect::<Vec<_>>();
    if headings(expected) != headings(got) {
        out.push(format!(
            "{what}\n  expected {:?}\n  got      {:?}",
            headings(expected),
            headings(got)
        ));
        // The bodies below would all be misaligned by one, and reporting ten shifted windows
        // as ten changed windows hides the single fact that one was added.
        return;
    }
    for (want, have) in expected.iter().zip(got) {
        if want.text != have.text {
            out.push(format!(
                "the window under `{}`\n{}",
                want.heading,
                first_difference(&want.text, &have.text)
            ));
        }
    }
}

/// The first line that differs, with its number, and how the lengths compare.
fn first_difference(expected: &[String], got: &[String]) -> String {
    for (i, (want, have)) in expected.iter().zip(got).enumerate() {
        if want != have {
            return format!("  line {i}\n    expected {want:?}\n    got      {have:?}");
        }
    }
    format!(
        "  the same for {} lines, then: expected {} lines, got {}",
        expected.len().min(got.len()),
        expected.len(),
        got.len()
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn every_expectation_names_a_page_that_exists() {
        // `load` enforces it; this is the test that says so out loud, because the failure it
        // prevents — a file renamed, an expectation quietly checking nothing — is silent.
        let all = load().expect("the page set loads");
        assert!(!all.is_empty(), "there are frozen pages to check");
        for expectation in &all {
            assert!(
                pages_dir().join(&expectation.page).is_file(),
                "{} names a missing page",
                expectation.page
            );
        }
    }

    #[test]
    fn every_page_asserts_the_extractor_for_the_question_it_answers() {
        // A page labelled `pricing` with no `plan_windows` is an expectation that checks
        // nothing at all, which is worse than not having the page: it looks like coverage.
        for e in load().expect("the page set loads") {
            let asserted = match e.answers {
                Answers::Pricing => e.plan_windows.is_some(),
                Answers::Features => e.capabilities.is_some(),
                Answers::Changes => e.changes.is_some(),
                Answers::Identity => e.facts.is_some(),
                Answers::Trust => e.assurances.is_some(),
                Answers::Direction => e.openings.is_some(),
            };
            assert!(asserted, "{} asserts nothing about {:?}", e.page, e.answers);
        }
    }

    #[test]
    fn every_page_says_why_it_is_here_and_where_it_came_from() {
        for e in load().expect("the page set loads") {
            assert!(
                e.why.split_whitespace().count() >= 12,
                "{}: `why` has to be a reason, not a label — got {:?}",
                e.page,
                e.why
            );
            assert!(
                e.from.contains("http"),
                "{}: `from` has to name the page it was frozen from",
                e.page
            );
        }
    }
}
