//! HTML into Markdown, keeping the structure that carries the meaning.
//!
//! `ROADMAP.md` Phase 1: *"Extraction → Markdown, **preserving tables and headings**."*
//! Those two words are the whole point of this module existing alongside
//! [`crate::text::visible`], which throws both away.
//!
//! # What flattening costs
//!
//! A pricing table is the single most valuable structure on a pricing page, and it is the
//! one that does not survive being flattened:
//!
//! ```text
//! visible():     Plan Price Starter $19 Grower $49 Harvest $129
//! markdown():    | Plan    | Price |
//!                |---|---|
//!                | Starter | $19  |
//!                | Grower  | $49  |
//! ```
//!
//! The first is a bag of tokens in which `$19` is adjacent to both `Starter` and `Grower`.
//! A model asked which plan costs $49 has to guess from word order, and
//! `BENCHMARKS.md` Run 3 already showed what that produces: **the 1.7B answered with the
//! price of the plan on the line above and quoted it correctly.** That failure is a
//! flattened table wearing a confident face.
//!
//! Headings matter for the same reason at a larger scale: `## Enterprise` above a paragraph
//! is what tells an extractor which plan the paragraph is about.
//!
//! # Deliberately not a full converter
//!
//! Links keep their text and lose their target, images are dropped, and inline styling
//! becomes plain text. This output has exactly one reader — a small model with a tight
//! context budget — and every character spent on `[click here](https://…utm_source=nav)` is
//! a character not spent on a price. Citations come from the fetched URL, which we already
//! have, not from links inside the page.

use std::fmt::Write as _;

/// The most Markdown we will produce from one page.
///
/// A span window is ~400 tokens and a whole page is passed to the router tier, so anything
/// past this is not going to be read anyway. Truncating here rather than at the model keeps
/// the cut somewhere we can see it.
pub const MAX_CHARS: usize = 40_000;

/// Convert a fetched body to Markdown, whatever it arrived as.
///
/// **Some of what we fetch is already Markdown.** `llms.txt` exists to publish a site as
/// Markdown, discovery follows it, and `linear.app/docs/mcp.md` came back through the HTML
/// converter as **one 2,167-word line** — every heading gone, because a Markdown `#` is text
/// to an HTML parser and there were no block tags to break lines on. Nothing downstream could
/// find a section in it, so the page reported no capabilities and looked like a page that had
/// none.
///
/// The sniff is deliberately narrow: Markdown headings present, and no sign of an HTML
/// document around them. A page that is HTML and happens to contain a `#` still goes through
/// the parser.
#[must_use]
pub fn from_body(body: &str) -> String {
    if is_markdown(body) {
        let trimmed: String = body.chars().take(MAX_CHARS).collect();
        return tidy_blank_lines(&trimmed);
    }
    from_html(body)
}

/// Whether this body is Markdown already rather than HTML.
fn is_markdown(body: &str) -> bool {
    /// One heading can be a stray `#` in prose. Two is a document.
    const HEADINGS: usize = 2;

    let head: String = body
        .chars()
        .take(4000)
        .to_owned()
        .collect::<String>()
        .to_lowercase();
    if head.contains("<html") || head.contains("<body") || head.contains("<!doctype") {
        return false;
    }
    body.lines()
        .filter(|l| {
            let t = l.trim_start();
            t.starts_with("# ") || t.starts_with("## ") || t.starts_with("### ")
        })
        .count()
        >= HEADINGS
}

/// Convert an HTML page to Markdown.
#[must_use]
pub fn from_html(html: &str) -> String {
    let cleaned = crate::text::strip_invisible(html);
    let mut out = String::with_capacity(cleaned.len() / 2);
    let mut table: Option<Table> = None;
    let mut list_depth = 0usize;
    let mut ordered_index: Vec<usize> = Vec::new();
    let mut pending_text = String::new();

    for event in events(&cleaned) {
        match event {
            Event::Text(text) => {
                let text = crate::text::decode_entities(&text);
                let trimmed = squash(&text);
                if trimmed.is_empty() {
                    continue;
                }
                if let Some(t) = table.as_mut() {
                    t.push_text(&trimmed);
                } else {
                    if !pending_text.is_empty() && !pending_text.ends_with(' ') {
                        pending_text.push(' ');
                    }
                    pending_text.push_str(&trimmed);
                }
            }
            Event::Open(tag) => match tag.as_str() {
                "table" => {
                    flush(&mut out, &mut pending_text);
                    table = Some(Table::default());
                }
                "tr" => {
                    if let Some(t) = table.as_mut() {
                        t.start_row();
                    }
                }
                "td" | "th" => {
                    if let Some(t) = table.as_mut() {
                        t.start_cell(tag == "th");
                    }
                }
                "ul" | "ol" => {
                    flush(&mut out, &mut pending_text);
                    list_depth += 1;
                    ordered_index.push(if tag == "ol" { 1 } else { 0 });
                }
                "li" => {
                    flush(&mut out, &mut pending_text);
                    let indent = "  ".repeat(list_depth.saturating_sub(1));
                    match ordered_index.last_mut() {
                        Some(n) if *n > 0 => {
                            let _ = write!(pending_text, "{indent}{n}. ");
                            *n += 1;
                        }
                        _ => pending_text.push_str(&format!("{indent}- ")),
                    }
                }
                h if h.len() == 2
                    && h.starts_with('h')
                    && h[1..].chars().all(|c| c.is_ascii_digit()) =>
                {
                    flush(&mut out, &mut pending_text);
                    let level: usize = h[1..].parse().unwrap_or(1);
                    pending_text.push_str(&"#".repeat(level.clamp(1, 6)));
                    pending_text.push(' ');
                }
                "p" | "div" | "section" | "article" | "br" => flush(&mut out, &mut pending_text),
                _ => {}
            },
            Event::Close(tag) => match tag.as_str() {
                "table" => {
                    if let Some(t) = table.take() {
                        out.push_str(&t.render());
                    }
                }
                "tr" => {
                    if let Some(t) = table.as_mut() {
                        t.end_row();
                    }
                }
                "td" | "th" => {
                    if let Some(t) = table.as_mut() {
                        t.end_cell();
                    }
                }
                "ul" | "ol" => {
                    flush(&mut out, &mut pending_text);
                    list_depth = list_depth.saturating_sub(1);
                    ordered_index.pop();
                }
                "li" | "p" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                    flush(&mut out, &mut pending_text);
                }
                _ => {}
            },
        }
        if out.len() > MAX_CHARS {
            break;
        }
    }
    // A page ending mid-table still has rows worth keeping.
    if let Some(t) = table.take() {
        out.push_str(&t.render());
    }
    flush(&mut out, &mut pending_text);

    let trimmed: String = out.chars().take(MAX_CHARS).collect();
    tidy_blank_lines(&trimmed)
}

fn flush(out: &mut String, pending: &mut String) {
    let line = pending.trim();
    if !line.is_empty() {
        out.push_str(line);
        out.push('\n');
    }
    pending.clear();
}

/// A table being built.
#[derive(Debug, Default)]
struct Table {
    rows: Vec<Vec<String>>,
    current: Vec<String>,
    cell: String,
    in_cell: bool,
    header_row: Option<usize>,
}

impl Table {
    fn start_row(&mut self) {
        self.current = Vec::new();
    }

    fn start_cell(&mut self, is_header: bool) {
        self.cell.clear();
        self.in_cell = true;
        if is_header && self.header_row.is_none() {
            self.header_row = Some(self.rows.len());
        }
    }

    fn push_text(&mut self, text: &str) {
        if !self.in_cell {
            return;
        }
        if !self.cell.is_empty() {
            self.cell.push(' ');
        }
        self.cell.push_str(text);
    }

    fn end_cell(&mut self) {
        // A pipe inside a cell would end the cell early in the rendered Markdown, turning
        // one row into two columns that do not line up with the header.
        self.current
            .push(self.cell.replace('|', "\\|").trim().to_owned());
        self.cell.clear();
        self.in_cell = false;
    }

    fn end_row(&mut self) {
        if !self.current.is_empty() {
            self.rows.push(std::mem::take(&mut self.current));
        }
    }

    /// GFM table, or a bulleted list when there is no grid to speak of.
    fn render(&self) -> String {
        let mut rows = self.rows.clone();
        if !self.current.is_empty() {
            rows.push(self.current.clone());
        }
        if rows.is_empty() {
            return String::new();
        }
        // A single-column table is a layout table — very common, and rendering it as a
        // one-column grid adds pipes without adding structure.
        let widest = rows.iter().map(Vec::len).max().unwrap_or(0);
        if widest < 2 {
            let mut out = String::from("\n");
            for row in &rows {
                for cell in row {
                    if !cell.is_empty() {
                        let _ = writeln!(out, "- {cell}");
                    }
                }
            }
            return out;
        }

        let mut out = String::from("\n");
        let header_at = self.header_row.unwrap_or(0);
        for (i, row) in rows.iter().enumerate() {
            let mut padded = row.clone();
            padded.resize(widest, String::new());
            let _ = writeln!(out, "| {} |", padded.join(" | "));
            if i == header_at {
                let _ = writeln!(out, "|{}", "---|".repeat(widest));
            }
        }
        out.push('\n');
        out
    }
}

/// A minimal tag walk. Same approach as [`crate::text`], for the same reason.
enum Event {
    Text(String),
    Open(String),
    Close(String),
}

fn events(html: &str) -> Vec<Event> {
    let mut out = Vec::new();
    let mut rest = html;

    while let Some(start) = rest.find('<') {
        if start > 0 {
            out.push(Event::Text(rest[..start].to_owned()));
        }
        let after = &rest[start + 1..];
        let Some(end) = after.find('>') else {
            out.push(Event::Text(rest[start..].to_owned()));
            return out;
        };
        let inner = &after[..end];
        let closing = inner.starts_with('/');
        let name: String = inner
            .trim_start_matches('/')
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric())
            .collect::<String>()
            .to_lowercase();
        if !name.is_empty() {
            out.push(if closing {
                Event::Close(name)
            } else {
                Event::Open(name)
            });
        }
        rest = &after[end + 1..];
    }
    if !rest.is_empty() {
        out.push(Event::Text(rest.to_owned()));
    }
    out
}

fn squash(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// At most one blank line in a row. A model's context is the scarcest thing here.
fn tidy_blank_lines(s: &str) -> String {
    let mut out: Vec<&str> = Vec::new();
    let mut blanks = 0usize;
    for line in s.lines() {
        if line.trim().is_empty() {
            blanks += 1;
            if blanks > 1 {
                continue;
            }
        } else {
            blanks = 0;
        }
        out.push(line);
    }
    out.join("\n").trim().to_owned()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {

    #[test]
    fn a_body_that_is_already_markdown_keeps_its_headings() {
        // `llms.txt` publishes a site as Markdown and discovery follows it. Through the HTML
        // converter, linear.app/docs/mcp.md came back as one 2,167-word line: a `#` is text
        // to an HTML parser, and there were no block tags to break lines on. Nothing
        // downstream can find a section in that, so the page looked like it had none.
        let body = "# MCP server

The server provides an interface.

## Setup

Use OAuth.";
        let out = from_body(body);
        assert!(out.contains("# MCP server"), "{out}");
        assert!(out.contains("## Setup"), "{out}");
        assert_eq!(out.lines().filter(|l| l.starts_with('#')).count(), 2);
    }

    #[test]
    fn an_html_body_still_goes_through_the_parser() {
        // Including one that mentions a hash. The sniff must not fire on prose.
        let html = "<html><body><h2>Plans</h2><p>Ticket #1 is open</p></body></html>";
        assert!(from_body(html).contains("## Plans"));
    }

    #[test]
    fn a_markdown_body_is_bounded_like_any_other() {
        let body = "# H
## H2
"
        .to_owned()
            + &"a word ".repeat(40_000);
        assert!(from_body(&body).chars().count() <= MAX_CHARS);
    }
    use super::*;

    #[test]
    fn a_pricing_table_survives_as_a_table() {
        // The reason this module exists. Flattened, `$49` is adjacent to both `Grower` and
        // `Harvest`, and a model asked which plan costs $49 is guessing from word order —
        // which is exactly the failure BENCHMARKS Run 3 recorded.
        let html = "<table><tr><th>Plan</th><th>Price</th></tr>\
                    <tr><td>Starter</td><td>$19</td></tr>\
                    <tr><td>Grower</td><td>$49</td></tr></table>";
        let md = from_html(html);
        assert!(md.contains("| Plan | Price |"), "got:\n{md}");
        assert!(md.contains("| Grower | $49 |"), "got:\n{md}");
        assert!(md.contains("|---|---|"), "no header separator:\n{md}");
    }

    #[test]
    fn headings_keep_their_level() {
        let md = from_html("<h1>Pricing</h1><h2>Enterprise</h2><p>Contact us.</p>");
        assert!(md.contains("# Pricing"), "got:\n{md}");
        assert!(md.contains("## Enterprise"), "got:\n{md}");
    }

    #[test]
    fn a_heading_stays_attached_to_the_text_it_introduces() {
        // `## Enterprise` above a paragraph is what tells an extractor which plan the
        // paragraph is about. If they merge into one line, that association is lost.
        let md = from_html("<h2>Enterprise</h2><p>Contact sales for pricing.</p>");
        let lines: Vec<&str> = md.lines().filter(|l| !l.trim().is_empty()).collect();
        assert_eq!(lines[0], "## Enterprise");
        assert!(lines[1].starts_with("Contact sales"), "got: {lines:?}");
    }

    #[test]
    fn lists_become_lists() {
        let md = from_html("<ul><li>Unlimited orders</li><li>Five routes</li></ul>");
        assert!(md.contains("- Unlimited orders"), "got:\n{md}");
        assert!(md.contains("- Five routes"), "got:\n{md}");
    }

    #[test]
    fn an_ordered_list_is_numbered() {
        let md = from_html("<ol><li>First</li><li>Second</li></ol>");
        assert!(md.contains("1. First"), "got:\n{md}");
        assert!(md.contains("2. Second"), "got:\n{md}");
    }

    #[test]
    fn script_and_style_contents_never_appear() {
        // Same rule as `text::visible`, and for a stronger reason here: a Next.js page's
        // embedded JSON would otherwise be handed to the model as if it were prose, at
        // enormous cost in context for no information.
        let html = "<script>var price = 49;</script><style>.a{}</style><p>Contact us</p>";
        let md = from_html(html);
        assert!(!md.contains("49"), "script leaked:\n{md}");
        assert!(md.contains("Contact us"));
    }

    #[test]
    fn a_pipe_inside_a_cell_does_not_break_the_row() {
        // Otherwise one row silently becomes a row with an extra column, and every cell
        // after it lines up under the wrong header.
        let md = from_html("<table><tr><td>A|B</td><td>$9</td></tr></table>");
        assert!(md.contains(r"A\|B"), "got:\n{md}");
    }

    #[test]
    fn a_single_column_table_becomes_a_list_rather_than_a_grid() {
        // Layout tables are extremely common and a one-column grid adds pipes without
        // adding structure — pure context cost.
        let md = from_html("<table><tr><td>Just a layout cell</td></tr></table>");
        assert!(!md.contains('|'), "rendered a pointless grid:\n{md}");
        assert!(md.contains("- Just a layout cell"), "got:\n{md}");
    }

    #[test]
    fn a_ragged_table_still_lines_up() {
        // Real tables have rows with missing cells. Left ragged, the Markdown is not a
        // table at all and the structure is lost silently.
        let md =
            from_html("<table><tr><th>Plan</th><th>Price</th></tr><tr><td>Free</td></tr></table>");
        assert!(md.contains("| Free |  |"), "got:\n{md}");
    }

    #[test]
    fn a_page_ending_mid_table_keeps_the_rows_it_had() {
        // Our fetches are capped at 2 MB, so a truncated page is a real case.
        let md = from_html("<table><tr><td>Grower</td><td>$49</td></tr>");
        assert!(md.contains("$49"), "lost a truncated table:\n{md}");
    }

    #[test]
    fn the_output_is_bounded() {
        let huge = "<p>word</p>".repeat(200_000);
        assert!(from_html(&huge).len() <= MAX_CHARS);
    }

    #[test]
    fn malformed_html_does_not_panic() {
        for html in [
            "",
            "<",
            "<p",
            "</p>",
            "<table><tr><td>",
            "<<>>",
            "<h9>x</h9>",
        ] {
            let _ = from_html(html);
        }
    }

    #[test]
    fn blank_lines_do_not_accumulate() {
        // Context is the scarcest thing the model has. Six blank lines between two facts
        // is six tokens that could have been a price.
        let md = from_html("<div></div><div></div><p>A</p><div></div><div></div><p>B</p>");
        assert!(!md.contains("\n\n\n"), "got:\n{md:?}");
    }
}
