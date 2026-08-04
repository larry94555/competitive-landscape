//! Show which identity windows a saved page produces, with no model and no network.
//!
//! ```text
//! curl -sL https://plausible.io/about -o about.html
//! cargo run -p landscape-extract --example who -- about.html
//! ```

fn main() {
    for path in std::env::args().skip(1) {
        let Ok(body) = std::fs::read_to_string(&path) else {
            eprintln!("{path}: could not read");
            continue;
        };
        let markdown = landscape_extract::markdown::from_body(&body);
        let found = landscape_extract::identity::every_fact(&markdown);
        println!("\n===== {path}\n{} window(s)", found.len());
        for (fact, span) in &found {
            println!(
                "\n--- {} (line {}, score {})\n{}",
                fact.name(),
                span.starts_at_line,
                span.score,
                span.text
            );
        }
    }
}
