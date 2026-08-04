//! Print what a saved page converts to, which is what every other stage actually reads.
//!
//! ```text
//! cargo run -p landscape-extract --example md -- saved.html
//! ```
//!
//! The first thing to look at when a page reports nothing. `linear.app/docs/mcp.md` came out
//! of here as a single 2,167-word line — it was already Markdown, and the HTML converter had
//! nothing to break lines on — which is why `markdown::from_body` exists.

fn main() {
    for path in std::env::args().skip(1) {
        let Ok(body) = std::fs::read_to_string(&path) else {
            eprintln!("{path}: could not read");
            continue;
        };
        println!("===== {path}");
        println!("{}", landscape_extract::markdown::from_body(&body));
    }
}
