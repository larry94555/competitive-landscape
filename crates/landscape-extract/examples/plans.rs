//! Show which windows a saved page produces, without a model and without the network.
//!
//! ```text
//! curl -sL https://basecamp.com/pricing -o basecamp.html
//! cargo run -p landscape-extract --example plans -- basecamp.html
//! ```
//!
//! `ARCHITECTURE.md` §5.4: *a bad window is indistinguishable from a bad model at the output.*
//! This is how they are told apart. Every rule in `span.rs` was written against pages dumped
//! and read here first, and the wrong windows it showed — the FAQ, the page title, the
//! marketing line above the plan name — are recorded in `BENCHMARKS.md`.

fn main() {
    let mut any = false;
    for path in std::env::args().skip(1) {
        any = true;
        let Ok(html) = std::fs::read_to_string(&path) else {
            eprintln!("{path}: could not read");
            continue;
        };
        let markdown = landscape_extract::markdown::from_html(&html);
        let plans = landscape_extract::span::every_plan(&markdown);
        println!(
            "\n===== {path}\n{} lines of Markdown, {} window(s)",
            markdown.lines().count(),
            plans.len()
        );
        for plan in &plans {
            println!(
                "\n--- line {}, score {}, heading {:?}\n{}",
                plan.starts_at_line, plan.score, plan.heading, plan.text
            );
        }
    }
    if !any {
        eprintln!("usage: cargo run -p landscape-extract --example plans -- <saved.html>...");
    }
}
