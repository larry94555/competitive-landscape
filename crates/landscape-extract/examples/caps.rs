//! Show which capability windows a saved page produces, without a model or the network.
//!
//! ```text
//! curl -sL https://basecamp.com/features -o features.html
//! cargo run -p landscape-extract --example caps -- features.html
//! ```
//!
//! The counterpart of `--example plans`. Every rule in `capability.rs` was written against
//! pages read here first, and the wrong answers they gave — a footer menu as fourteen
//! features, another editor's name as a capability of Linear — are in `BENCHMARKS.md` Run 8.

fn main() {
    let mut any = false;
    for path in std::env::args().skip(1) {
        any = true;
        let Ok(body) = std::fs::read_to_string(&path) else {
            eprintln!("{path}: could not read");
            continue;
        };
        let markdown = landscape_extract::markdown::from_body(&body);
        let found = landscape_extract::capability::every_capability(&markdown);
        println!(
            "\n===== {path}\n{} window(s) of {} the page names",
            found.windows.len(),
            found.considered
        );
        for window in &found.windows {
            println!(
                "\n--- line {}, {} words of description, named {:?}\n{}",
                window.starts_at_line,
                window.score,
                window.heading,
                window.text.chars().take(400).collect::<String>()
            );
        }
    }
    if !any {
        eprintln!("usage: cargo run -p landscape-extract --example caps -- <saved.html>...");
    }
}
