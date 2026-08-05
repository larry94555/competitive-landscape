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

    /// The heading of each plan window, in page order. `Answers::Pricing`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan_windows: Option<Vec<String>>,
    /// The capabilities named, and how many the page offered. `Answers::Features`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<Capabilities>,
    /// Dated entries. `Answers::Changes`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub changes: Option<Changes>,
    /// Which of the three facts the page has a window for. `Answers::Identity`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub facts: Option<Vec<String>>,
}

/// What a features page yields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Capabilities {
    /// The name of each window, in page order.
    pub named: Vec<String>,
    /// How many sections looked like capabilities before the cap. Asserted because a cap
    /// nobody counts is a cap nobody notices growing.
    pub considered: usize,
}

/// What a changelog yields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Changes {
    /// Entries read, after the cap.
    pub read: usize,
    /// Dated entries on the page.
    pub considered: usize,
    /// The newest, as `YYYY-MM-DD`. `None` for a page with no dates — which is a finding, not
    /// an absence of one.
    pub newest: Option<String>,
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
            let got: Vec<String> = landscape_extract::span::every_plan(&markdown)
                .into_iter()
                .map(|s| s.heading.unwrap_or_default())
                .collect();
            if &got != expected {
                out.push(format!(
                    "plan windows\n  expected {expected:?}\n  got      {got:?}"
                ));
            }
        }

        if let Some(expected) = &self.capabilities {
            let found = landscape_extract::capability::every_capability(&markdown);
            let named: Vec<String> = found
                .windows
                .iter()
                .map(|w| w.heading.clone().unwrap_or_default())
                .collect();
            if named != expected.named {
                out.push(format!(
                    "capabilities\n  expected {:?}\n  got      {named:?}",
                    expected.named
                ));
            }
            if found.considered != expected.considered {
                out.push(format!(
                    "capabilities considered: expected {}, got {}",
                    expected.considered, found.considered
                ));
            }
        }

        if let Some(expected) = &self.changes {
            let found = landscape_extract::changes::every_change(&markdown);
            let newest = found
                .entries
                .first()
                .map(|e| format!("{:04}-{:02}-{:02}", e.year, e.month, e.day));
            if found.entries.len() != expected.read
                || found.considered != expected.considered
                || newest != expected.newest
            {
                out.push(format!(
                    "changes\n  expected {} of {}, newest {:?}\n  got      {} of {}, newest {newest:?}",
                    expected.read,
                    expected.considered,
                    expected.newest,
                    found.entries.len(),
                    found.considered
                ));
            }
        }

        if let Some(expected) = &self.facts {
            let got: Vec<String> = landscape_extract::identity::every_fact(&markdown)
                .into_iter()
                .map(|(fact, _)| fact.name().to_owned())
                .collect();
            if &got != expected {
                out.push(format!(
                    "identity facts\n  expected {expected:?}\n  got      {got:?}"
                ));
            }
        }

        Ok(out)
    }
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
