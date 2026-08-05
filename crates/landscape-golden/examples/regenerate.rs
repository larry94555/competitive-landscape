//! Rewrite every `*.expected.json` from what the extractors produce today.
//!
//! ```text
//! cargo run -p landscape-golden --example regenerate
//! ```
//!
//! **This does not validate anything — it records.** Run it only when a change to extraction is
//! believed to be an improvement, then read the diff line by line before committing: that diff
//! is the whole review, and accepting it without reading turns the frozen pages from a test into
//! a transcript. The prose fields (`why`, `from`, `known_wrong`) are carried through untouched.

#![allow(clippy::expect_used)]
// A tool that rewrites the expectations should stop dead on the first thing it cannot read,
// rather than write nine files and leave the tenth as it was.

use landscape_golden::pages::{self, Capabilities, Changes, Dated, FactWindow, Window};

fn main() {
    let mut existing = pages::load().expect("the page set loads");
    for expectation in &mut existing {
        let markdown = expectation.markdown().expect("a frozen page reads");

        if expectation.plan_windows.is_some() {
            expectation.plan_windows = Some(
                landscape_extract::span::every_plan(&markdown)
                    .into_iter()
                    .map(Window::from)
                    .collect(),
            );
        }
        if let Some(caps) = &mut expectation.capabilities {
            let found = landscape_extract::capability::every_capability(&markdown);
            *caps = Capabilities {
                named: found.windows.into_iter().map(Window::from).collect(),
                considered: found.considered,
            };
        }
        if let Some(changes) = &mut expectation.changes {
            let found = landscape_extract::changes::every_change(&markdown);
            *changes = Changes {
                entries: found.entries.iter().map(Dated::from).collect(),
                considered: found.considered,
            };
        }
        if expectation.facts.is_some() {
            expectation.facts = Some(
                landscape_extract::identity::every_fact(&markdown)
                    .into_iter()
                    .map(FactWindow::from)
                    .collect(),
            );
        }

        let path = pages::pages_dir().join(expectation.page.replace(".md", ".expected.json"));
        let mut json = serde_json::to_string_pretty(&expectation).expect("an expectation encodes");
        json.push('\n');
        std::fs::write(&path, json).expect("the expectation writes");
        println!("wrote {}", path.display());
    }
}
