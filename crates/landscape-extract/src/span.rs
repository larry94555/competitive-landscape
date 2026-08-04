//! Choosing the ~400 tokens of a page the model actually reads.
//!
//! [`ARCHITECTURE.md`] §5.4:
//!
//! > **Span pre-selection** applies the same idea to what the model *does* read: heading
//! > structure, table proximity, and keyword windows reduce each source from ~2,500 tokens
//! > to a ~400-token candidate window before the extractor sees it. […] The selection
//! > heuristic is versioned alongside prompts and is itself part of the golden-set
//! > evaluation, because **a bad window is indistinguishable from a bad model at the
//! > output.**
//!
//! That last sentence describes exactly what `BENCHMARKS.md` **Run 5** measured. The 4B
//! model — the one scoring 90% on the golden set, which has never invented a price —
//! returned *"no price published"* for a page showing `$299/month`. Given the same prompt
//! and the same words as a **39-word span**, it answered `Pro Unlimited`, `$299`, with a
//! verbatim quote.
//!
//! **Only the number of words changed.** Which means §5.4's window was never only a latency
//! measure: without it, extraction on real pages does not work.
//!
//! # What makes a good window
//!
//! Run 5 also showed what has to be in one. The span that worked was:
//!
//! ```text
//! ## Pro Unlimited                      <- the heading, saying which plan
//! ### Top-of-the-line, all-inclusive…
//! $299/month, billed annually…          <- the price
//! - Unlimited projects                  <- what it includes
//! ```
//!
//! A window containing the price and not the heading is worse than useless: the model gets a
//! number with nothing to attach it to, and the golden set already showed what it does then
//! — it attaches the number to the nearest plan name, confidently, with a correct quote.
//!
//! So a window is **always extended upward to its enclosing heading**, even when that costs
//! tokens the price alone would not have needed.
//!
//! [`ARCHITECTURE.md`]: ../../../docs/ARCHITECTURE.md

/// Bumped whenever the heuristic changes.
///
/// Versioned like a prompt, for §5.4's reason: two spans chosen by different rules produce
/// scores that are not comparable, and a table mixing them silently is worse than no table.
pub const SPAN_VERSION: u32 = 1;

/// Roughly 400 tokens, the figure §5.4 budgets.
///
/// Characters rather than tokens because the tokenizer lives in llama.cpp and a rough
/// character bound is checkable here, in a unit test, without one. Four characters per token
/// is the usual English approximation and errs toward a smaller window.
pub const WINDOW_CHARS: usize = 1600;

/// A slice of a page, chosen because it looks like it holds the answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub text: String,
    /// Which line of the converted Markdown the window starts at. Kept so a wrong answer
    /// can be traced to the window that produced it rather than blamed on the model —
    /// §5.4's whole point about the two being indistinguishable at the output.
    pub starts_at_line: usize,
    /// The heading the window sits under, if any. This is what tells the model which plan
    /// the price belongs to.
    pub heading: Option<String>,
    /// What the scorer liked about it, for the same debugging reason.
    pub score: u32,
}

impl Span {
    /// The window, with its heading restored at the top if the cut lost it.
    #[must_use]
    pub fn prompt_text(&self) -> String {
        match &self.heading {
            Some(h) if !self.text.starts_with(h.as_str()) => format!("{h}\n{}", self.text),
            _ => self.text.clone(),
        }
    }
}

/// Find the window most likely to state a price.
///
/// Returns `None` when nothing in the page scores at all — a page with no price-shaped
/// content anywhere. That is a **finding**, and passing the whole page to a model instead
/// would convert it into a guess.
#[must_use]
pub fn for_pricing(markdown: &str) -> Option<Span> {
    let lines: Vec<&str> = markdown.lines().collect();
    if lines.is_empty() {
        return None;
    }

    let scores = score_lines(&lines);
    if scores.iter().all(|s| *s == 0) {
        return None;
    }

    // The best window is the one whose lines sum highest. Walking a window rather than
    // picking the single best line matters because a pricing block is several lines — a
    // heading, a price, and a feature list — and the price line alone is the part that
    // makes least sense on its own.
    let (best_start, best_end, best_score) = best_window(&lines, &scores);

    // Anchored to the best-scoring line inside the window, not to where the window starts.
    // A window wide enough to hold a whole short page starts at line 0, and the heading
    // there is the page title — `# Pick a package` — rather than the one that governs the
    // price. The model needs the second: it is what says which plan the number belongs to.
    let peak = (best_start..=best_end)
        .max_by_key(|i| scores.get(*i).copied().unwrap_or(0))
        .unwrap_or(best_start);
    let heading = heading_above(&lines, peak);

    let mut text = lines[best_start..=best_end].join("\n");
    if text.chars().count() > WINDOW_CHARS {
        text = text.chars().take(WINDOW_CHARS).collect();
    }

    Some(Span {
        text,
        starts_at_line: best_start,
        heading,
        score: best_score,
    })
}

/// The scoring window, by character budget.
fn best_window(lines: &[&str], scores: &[u32]) -> (usize, usize, u32) {
    let mut best = (0usize, 0usize, 0u32);

    for start in 0..lines.len() {
        let mut chars = 0usize;
        let mut total = 0u32;
        let mut end = start;
        for (i, line) in lines.iter().enumerate().skip(start) {
            let len = line.chars().count() + 1;
            if chars + len > WINDOW_CHARS && i > start {
                break;
            }
            chars += len;
            total += scores[i];
            end = i;
        }
        if total > best.2 {
            best = (start, end, total);
        }
    }
    best
}

/// The most significant heading governing this line.
///
/// **Not simply the nearest one.** A plan is usually written as a name and then a subtitle:
///
/// ```text
/// ## Pro Unlimited                                  <- names the plan
/// ### All-inclusive pricing. Unlimited users.       <- nearest, and says nothing useful
/// $299/month, billed annually
/// ```
///
/// Taking the nearest heading hands the model *"All-inclusive pricing"* as the thing the
/// `$299` belongs to, which is not a plan name. So the walk continues upward past subtitles
/// until it reaches a top-level heading, and returns the most significant one it saw.
///
/// The subtitle is not lost — it is inside the window text either way. What changes is which
/// line is promoted to the front of the prompt.
fn heading_above(lines: &[&str], from: usize) -> Option<String> {
    /// Far enough to cross a feature list; short enough not to reach the previous section.
    const MAX_LOOKBACK: usize = 40;

    let mut best: Option<(usize, String)> = None;
    let start = from.min(lines.len().saturating_sub(1));

    for line in lines[..=start].iter().rev().take(MAX_LOOKBACK) {
        let t = line.trim();
        if !t.starts_with('#') {
            continue;
        }
        let level = t.chars().take_while(|c| *c == '#').count();
        if best.as_ref().is_none_or(|(l, _)| level < *l) {
            best = Some((level, t.to_owned()));
        }
        // A top-level heading is as significant as it gets; nothing above it can improve
        // the answer, and continuing would cross into the previous section.
        if level <= 2 {
            break;
        }
    }
    best.map(|(_, text)| text)
}

/// Score every line, using the context a single line cannot see.
///
/// Two of the signals are structural rather than lexical, and they exist because of a
/// specific wrong answer. On `basecamp.com/pricing` the first version of this scorer chose
/// the FAQ — *"The Admin Pro Pack upgrade is $50/month flat"* — over the actual plan at
/// `$299/month`, because the FAQ mentions prices several times in close succession and the
/// plan states its price once.
///
/// What distinguishes them is not vocabulary. It is **shape**: a plan block is a heading
/// with its price immediately underneath, and an FAQ is prose a long way from its heading,
/// full of questions. Both signals below encode that, and neither is specific to Basecamp.
fn score_lines(lines: &[&str]) -> Vec<u32> {
    let mut out = Vec::with_capacity(lines.len());
    let mut since_heading = usize::MAX;

    for line in lines {
        if line.trim_start().starts_with('#') {
            since_heading = 0;
        } else if since_heading != usize::MAX {
            since_heading = since_heading.saturating_add(1);
        }

        let mut score = score_line(line);

        // A price directly under a heading is a plan's price. Four lines is enough for a
        // heading, a subtitle and a sentence — past that the connection is a coincidence.
        if score > 0 && since_heading <= 4 {
            score += 8;
        }
        // A question is a hypothetical, and the answer under it is about an exception.
        // "Could we really add 1000 users and still just pay $299/month?" is not the
        // sentence that states what the product costs.
        if line.contains('?') {
            score = score.saturating_sub(6);
        }
        out.push(score);
    }
    out
}

/// How much this line looks like part of a pricing block.
///
/// Weights are small integers chosen to order the signals, not fitted to anything. What the
/// scorer has to get right is which *region* of a page wins, and the signals disagree rarely
/// enough that their exact magnitudes do not decide it.
fn score_line(line: &str) -> u32 {
    let l = line.to_lowercase();
    let mut score = 0u32;

    // A currency marker beside a number is the strongest signal there is, and it is the one
    // `price::find` already treats as the definition of a price.
    if crate::price::find(line).is_some() {
        score += 10;
    }
    // A billing period turns a number into a price.
    for word in [
        "per month",
        "/month",
        "a month",
        "per year",
        "/year",
        "per user",
        "/user",
    ] {
        if l.contains(word) {
            score += 4;
            break;
        }
    }
    // Table rows are where prices live on the pages that have tables.
    if line.trim_start().starts_with('|') {
        score += 3;
    }
    // Plan vocabulary. Weak on its own — every page says "free" somewhere — but it is what
    // distinguishes a pricing block from a paragraph that happens to mention a number.
    for word in ["plan", "pricing", "tier", "package", "per seat", "billed"] {
        if l.contains(word) {
            score += 2;
            break;
        }
    }
    // A "contact sales" line is a real finding and must be able to win a window, or a page
    // with no price would hand the model something irrelevant instead of the sentence
    // saying there is no price.
    for phrase in [
        "contact sales",
        "contact us for pricing",
        "custom pricing",
        "talk to us",
    ] {
        if l.contains(phrase) {
            score += 6;
            break;
        }
    }
    score
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    /// A page shaped like the one in Run 5: navigation, then a plan, then an FAQ.
    fn realistic_page() -> String {
        let filler = "We built this for teams who were tired of doing it over email. \
                      Everything is in one place and nobody has to ask where anything is. "
            .repeat(12);
        format!(
            "# Pick a package\n{filler}\n\
             ## Pro Unlimited\n\
             ### All-inclusive pricing. Unlimited users, no per-user fees.\n\
             $299/month, billed annually. Your whole organization for one fixed price.\n\
             - Unlimited projects\n\
             - 5 terabytes of storage\n\
             ## Questions\n{filler}"
        )
    }

    #[test]
    fn the_window_finds_the_priced_block_in_a_long_page() {
        // The Run 5 case, in miniature: the price is one line in a page of prose.
        let span = for_pricing(&realistic_page()).expect("a price is on the page");
        assert!(span.text.contains("$299"), "got:\n{}", span.text);
    }

    #[test]
    fn the_window_carries_the_heading_that_names_the_plan() {
        // The property Run 5 turned on. A number with nothing to attach it to is worse
        // than no number: the golden set showed the model attaches it to the nearest plan
        // name, confidently, with a correct quote.
        let span = for_pricing(&realistic_page()).expect("a price is on the page");
        assert_eq!(
            span.heading.as_deref(),
            Some("## Pro Unlimited"),
            "{span:?}"
        );
        assert!(span.prompt_text().contains("Pro Unlimited"));
    }

    #[test]
    fn the_window_stays_inside_its_budget() {
        // §5.4 budgets ~400 tokens per source so that eight sources cost ~3,200 rather
        // than ~20,000. A window that quietly grew would put the prefill cost back.
        let span = for_pricing(&realistic_page()).expect("a price is on the page");
        assert!(
            span.text.chars().count() <= WINDOW_CHARS,
            "{} chars",
            span.text.chars().count()
        );
    }

    #[test]
    fn a_page_with_no_price_signal_returns_nothing() {
        // A finding, not a fallback. Passing the whole page instead would turn "this
        // company publishes no price" into a guess, which is the failure the product
        // cannot survive.
        let page = "# About us\n\nWe were founded in 2019 and have 42 employees.\n\
                    Our office is at 1200 Preston Road. Call us on (802) 555-0148.";
        assert!(for_pricing(page).is_none(), "invented a pricing window");
    }

    #[test]
    fn a_contact_sales_line_can_win_the_window() {
        // Otherwise a quote-only page hands the model an irrelevant paragraph instead of
        // the sentence that actually answers the question.
        let filler = "Everything you need to run delivery routes at scale. ".repeat(20);
        let page =
            format!("# Enterprise\n{filler}\n## Pricing\nContact sales for pricing.\n{filler}");
        let span = for_pricing(&page).expect("contact-sales should score");
        assert!(span.text.contains("Contact sales"), "got:\n{}", span.text);
    }

    #[test]
    fn a_plan_block_beats_an_faq_that_mentions_more_prices() {
        // The wrong answer this scorer was rewritten for. On basecamp.com the first version
        // chose the FAQ — "The Admin Pro Pack upgrade is $50/month flat" — over the actual
        // $299 plan, because the FAQ says "price" more often. Shape, not vocabulary, is
        // what tells them apart.
        let page = "# Pick a package
                    ## Pro Unlimited
                    $299/month, billed annually. Your whole organization.
                    - Unlimited projects
                    ## Questions
                    Could we add 1000 users and still pay $299/month total?
                    No. On the Pro package we only bill for employees.
                    The Admin Pro Pack upgrade is $50/month flat.
                    You can add a terabyte of storage for $50/month flat.
                    Is the $50/month charge per user? No, it is flat.";
        let span = for_pricing(page).expect("a price is on the page");
        assert!(
            span.text.contains("$299"),
            "chose the FAQ:
{}",
            span.text
        );
        assert_eq!(
            span.heading.as_deref(),
            Some("## Pro Unlimited"),
            "{span:?}"
        );
    }

    #[test]
    fn a_price_inside_a_question_does_not_win_on_its_own() {
        // A hypothetical is not a statement of what something costs.
        let filler = "Everything you need to run delivery routes. ".repeat(20);
        let page = format!(
            "# FAQ
{filler}
What if I only need $19 of capacity a month?
{filler}
             ## Starter
$19 per month, billed monthly.
- One route"
        );
        let span = for_pricing(&page).expect("a price is on the page");
        assert_eq!(span.heading.as_deref(), Some("## Starter"), "{span:?}");
    }

    #[test]
    fn a_pricing_table_wins_over_a_passing_mention_of_a_price() {
        // A blog paragraph saying "we used to charge $9" must not beat the actual table.
        let page = "# Blog\nWe used to charge $9 a month for it, years ago.\n\n\
                    ## Plans\n| Plan | Price |\n|---|---|\n| Starter | $19 |\n| Grower | $49 |";
        let span = for_pricing(page).expect("a table is on the page");
        assert!(
            span.text.contains("| Grower | $49 |"),
            "got:\n{}",
            span.text
        );
    }

    #[test]
    fn the_span_records_where_it_came_from() {
        // §5.4: a bad window is indistinguishable from a bad model at the output. This is
        // what makes them distinguishable — a wrong answer can be traced to the window.
        let span = for_pricing(&realistic_page()).expect("a price is on the page");
        assert!(span.starts_at_line > 0);
        assert!(span.score > 0);
    }

    #[test]
    fn a_subtitle_does_not_displace_the_plan_name() {
        // A plan is a name and then a subtitle. Taking the nearest heading hands the model
        // "All-inclusive pricing" as the thing $299 belongs to, which is not a plan.
        let page = "## Pro Unlimited
### All-inclusive pricing. Unlimited users.
                    $299/month, billed annually
- Unlimited projects";
        let span = for_pricing(page).expect("a price is on the page");
        assert_eq!(
            span.heading.as_deref(),
            Some("## Pro Unlimited"),
            "{span:?}"
        );
    }

    #[test]
    fn the_heading_is_not_duplicated_when_it_is_already_in_the_window() {
        let page = "## Pro\n$49 per month\n- Everything included";
        let span = for_pricing(page).expect("a price is on the page");
        let text = span.prompt_text();
        assert_eq!(text.matches("## Pro").count(), 1, "got:\n{text}");
    }

    #[test]
    fn an_empty_or_tiny_page_does_not_panic() {
        assert!(for_pricing("").is_none());
        assert!(for_pricing("\n\n\n").is_none());
        assert!(for_pricing("hello").is_none());
        let _ = for_pricing("$49");
    }

    #[test]
    fn a_page_that_is_all_price_lines_still_produces_one_window() {
        // The degenerate case: every line scores, so the window is decided by the budget
        // rather than by the scores, and it must still terminate with something usable.
        let page = "$19 per month\n".repeat(500);
        let span = for_pricing(&page).expect("prices are on the page");
        assert!(span.text.chars().count() <= WINDOW_CHARS);
        assert!(span.text.contains("$19"));
    }
}
