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
pub const SPAN_VERSION: u32 = 2;

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

/// How many plan windows one page may contribute.
///
/// Each window is a model call, so this is a cost bound as much as a sanity bound. Six is
/// more plans than a pricing page normally publishes; a page that produces more has almost
/// certainly had something other than a plan mistaken for one, and reporting the six
/// strongest is a better failure than spending twelve model calls on it.
pub const MAX_PLANS: usize = 6;

/// The line score a section must reach somewhere inside it to count as a plan.
///
/// Eighteen is exactly a price (10) directly under a heading that names something (8) — which
/// is to say **a plan block is a section that states a price under its own name**. The number
/// is the shape, not a tuned constant, and no combination of lexical signals reaches it
/// without that bonus.
///
/// It is what separates the two real plans on `basecamp.com/pricing` from the FAQ below them,
/// which reaches 16 by mentioning `$50/month` in a sentence.
const PLAN_FLOOR: u32 = 18;

/// Every plan the page publishes, in the order it publishes them.
///
/// [`for_pricing`] answers *what does this page cost*, and a pricing page does not have one
/// answer. `basecamp.com/pricing` publishes `$15/user` Pro and `$299/month` Pro Unlimited;
/// picking the higher-scoring window reports one and hides the other, and a competitor
/// report showing a rival's cheapest plan and silently dropping the rest is worse than one
/// showing none — it looks complete.
///
/// # Why sections rather than a wider window
///
/// The obvious alternative is to widen the window until it holds every plan and ask the model
/// for a list. That undoes what Run 5 established: the model answers on a small window and
/// fails on a large one. So the page is **cut deterministically into one section per plan**,
/// and each section gets the same small-window extraction that already works.
///
/// It costs one model call per plan instead of one per page. That is the trade — accuracy
/// bought with prefill, which is the same trade §5.4 makes in the other direction.
#[must_use]
pub fn every_plan(markdown: &str) -> Vec<Span> {
    let lines: Vec<&str> = markdown.lines().collect();
    if lines.is_empty() {
        return Vec::new();
    }
    let scores = score_lines(&lines);

    let mut found: Vec<Span> = crate::doc::sections(&lines)
        .into_iter()
        .filter_map(|(start, end)| {
            let peak = (start..=end).max_by_key(|i| scores[*i])?;
            if scores[peak] < PLAN_FLOOR {
                return None;
            }
            Some(window(&lines, start, end, peak, scores[peak]))
        })
        .collect();

    // A page can state a price without ever putting one under a plan name — a single
    // sentence, a bare table, "contact sales for pricing". Those are still findings, and the
    // sliding window is what Run 6 measured on them, so nothing is lost by falling back to
    // it. What would be lost is the honesty of the empty case: `for_pricing` returns nothing
    // for a page with no price-shaped content at all, and so, then, does this.
    if found.is_empty() {
        return for_pricing(markdown).into_iter().collect();
    }

    // Over the cap, keep the strongest — then put them back in the order the page presents
    // them, because that order is information: pricing pages lead with the cheap plan.
    if found.len() > MAX_PLANS {
        found.sort_by_key(|s| std::cmp::Reverse(s.score));
        found.truncate(MAX_PLANS);
        found.sort_by_key(|s| s.starts_at_line);
    }
    found
}

/// One section, cut to the token budget.
///
/// Cut from the top, which is where the answer is: a section qualified as a plan by stating a
/// price within a few lines of its own heading, so a plan with forty feature bullets loses the
/// bullets and keeps the name and the number. That is the right end to lose.
fn window(lines: &[&str], start: usize, end: usize, peak: usize, score: u32) -> Span {
    let from = start;
    let mut text = lines[from..=end].join("\n");
    if text.chars().count() > WINDOW_CHARS {
        text = text.chars().take(WINDOW_CHARS).collect();
    }
    Span {
        text,
        starts_at_line: from,
        // A section's own heading names the plan — that is what made it a section. The walk
        // upward is only needed for text sitting above the page's first heading.
        heading: crate::doc::section_heading(lines, start, end)
            .or_else(|| heading_above(lines, peak)),
        score,
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

    // The best window is the one whose lines sum highest — but only among windows that
    // actually state something about a price. Without that condition a long feature-
    // comparison table wins on structure alone: forty table rows at three points each
    // outscore any real pricing block, and not one of them contains a currency symbol.
    //
    // That is not hypothetical. `todoist.com/pricing` renders its prices in JavaScript, so
    // the Markdown holds the comparison table and nothing else, and the window chosen from
    // it was headed `| | Beginner | Pro | Business |` with `| Personal projects | 5 |`
    // underneath. The model returned **"Beginner at $5"** — a feature limit read as a price,
    // on a page whose HTML contains no dollar amount at all.
    //
    // The right answer for that page is that it publishes no price we can see, which is what
    // ARCHITECTURE §5.5's JavaScript-gap counter is *for*. A window with no price in it
    // cannot produce that answer; it can only produce a guess.
    let (best_start, best_end, best_score) = best_window(&lines, &scores)?;

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

/// The highest-scoring window that says something about a price, by character budget.
///
/// `None` when no window does — see the note at the call site for the page that made this a
/// condition rather than a tiebreak.
fn best_window(lines: &[&str], scores: &[u32]) -> Option<(usize, usize, u32)> {
    let mut best: Option<(usize, usize, u32)> = None;

    for start in 0..lines.len() {
        let mut chars = 0usize;
        let mut total = 0u32;
        let mut end = start;
        let mut states_price = false;
        for (i, line) in lines.iter().enumerate().skip(start) {
            let len = line.chars().count() + 1;
            if chars + len > WINDOW_CHARS && i > start {
                break;
            }
            chars += len;
            total += scores[i];
            states_price |= states_a_price(line);
            end = i;
        }
        if states_price && best.is_none_or(|(_, _, b)| total > b) {
            best = Some((start, end, total));
        }
    }
    best
}

/// Whether this line says something about what something costs.
///
/// The hard signals, as against the ones that only say *this looks like a pricing page*: a
/// currency amount, or a sentence saying the price is on request. A window containing neither
/// contains no answer, however much pricing-shaped furniture surrounds it.
fn states_a_price(line: &str) -> bool {
    let lower = line.to_lowercase();
    crate::price::find(line).is_some() || CONTACT_SALES.iter().any(|p| lower.contains(p))
}

/// Ways of publishing that there is a price and it is not on the page.
const CONTACT_SALES: [&str; 4] = [
    "contact sales",
    "contact us for pricing",
    "custom pricing",
    "talk to us",
];

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
    let mut under_a_question = false;

    for line in lines {
        if line.trim_start().starts_with('#') {
            since_heading = 0;
            under_a_question = asks_rather_than_names(line);
        } else if since_heading != usize::MAX {
            since_heading = since_heading.saturating_add(1);
        }

        let mut score = score_line(line);

        // A price directly under a heading is a plan's price. Four lines is enough for a
        // heading, a subtitle and a sentence — past that the connection is a coincidence.
        //
        // Unless the heading is `## Questions`, which names no plan. Without that exception
        // the first two answers of an FAQ collect the bonus and the FAQ becomes a plan.
        if score > 0 && since_heading <= 4 && !under_a_question {
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

/// Whether a heading introduces questions rather than naming something purchasable.
///
/// `## Frequently asked questions` and `## Pro Unlimited` are both headings with prices under
/// them. Only one of them names a plan.
fn asks_rather_than_names(heading: &str) -> bool {
    let h = heading.to_lowercase();
    h.contains('?') || h.contains("question") || h.contains("faq") || h.contains("asked")
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
    if CONTACT_SALES.iter().any(|phrase| l.contains(phrase)) {
        score += 6;
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

    /// The shape of every real pricing page measured in Run 7: plan sections, then an FAQ.
    fn two_plan_page() -> String {
        "# Pick a package
## Pro Unlimited
### Top-of-the-line, all-inclusive pricing. Unlimited users.
$299/month, billed annually. Your whole organization for one fixed price.
- Unlimited projects
- 5 terabytes of storage
## Pro
### A great choice for freelancers and smaller teams.
$15/user, billed monthly. We only bill you for employees.
- Unlimited projects
- 500 GB of storage
## I have pricing questions
Could we add 1000 users and still pay $299/month total?
No. On the Pro package we only bill for employees.
The Admin Pro Pack upgrade is $50/month flat.
You can add a terabyte of storage for $50/month flat."
            .to_owned()
    }

    #[test]
    fn a_page_with_two_plans_produces_two_windows() {
        // The finding this function exists for. Reporting one of these and hiding the other
        // does not look incomplete to a reader — it looks wrong.
        let plans = every_plan(&two_plan_page());
        assert_eq!(plans.len(), 2, "{plans:#?}");
        assert!(plans[0].text.contains("$299"), "{:?}", plans[0].text);
        assert!(plans[1].text.contains("$15"), "{:?}", plans[1].text);
    }

    #[test]
    fn the_plans_come_back_in_the_order_the_page_presents_them() {
        // Page order is information: it is the ordering the company chose, and re-sorting
        // by price would answer a question the page did not ask.
        let plans = every_plan(&two_plan_page());
        assert!(plans[0].starts_at_line < plans[1].starts_at_line);
    }

    #[test]
    fn each_window_is_named_by_its_own_plan() {
        let plans = every_plan(&two_plan_page());
        let headings: Vec<_> = plans.iter().filter_map(|p| p.heading.clone()).collect();
        assert_eq!(headings, ["## Pro Unlimited", "## Pro"], "{plans:#?}");
    }

    #[test]
    fn the_faq_underneath_is_not_a_third_plan() {
        // It mentions $50/month twice, which is more prices than either plan states. What
        // stops it is that `## I have pricing questions` names no plan, so the prices under
        // it never collect the heading bonus.
        let plans = every_plan(&two_plan_page());
        assert!(
            !plans.iter().any(|p| p.text.contains("$50")),
            "the FAQ became a plan: {plans:#?}"
        );
    }

    #[test]
    fn a_marketing_line_does_not_displace_the_plan_name() {
        // notion.com writes the pair the other way up from basecamp.com — the heading above
        // the plan name is the marketing sentence. Levels cannot tell the two apart.
        let page = "## Essentials for staying organized.
### Free
$0 per member / month
For individuals to organize personal projects.
## The workspace for work that matters.
### Business
$20 per member / month
For growing businesses.";
        let plans = every_plan(page);
        let headings: Vec<_> = plans.iter().filter_map(|p| p.heading.clone()).collect();
        assert_eq!(headings, ["### Free", "### Business"], "{plans:#?}");
    }

    #[test]
    fn every_window_stays_inside_the_budget() {
        // The cost argument only holds if each window is small. Six plans at ~400 tokens is
        // the trade this makes; six plans at page size is the thing Run 5 measured failing.
        let long = "- Everything in the plan below, and rather a lot more besides. ".repeat(60);
        let page = format!(
            "## Pro\n$15/user, billed monthly\n{long}\n## Max\n$99/user, billed monthly\n{long}"
        );
        for span in every_plan(&page) {
            assert!(
                span.text.chars().count() <= WINDOW_CHARS,
                "{}",
                span.text.len()
            );
        }
    }

    #[test]
    fn a_long_section_loses_its_feature_list_and_keeps_its_price() {
        // Forty bullets is the common shape, and the bullets are the part to lose: the
        // section qualified by stating a price near its own heading, so the top of it is
        // where the answer is.
        let long = "- One more feature, described at some length for the sake of it.\n".repeat(60);
        let page = format!("## Pro\n$15/user, billed monthly\n{long}");
        let plans = every_plan(&page);
        assert_eq!(plans.len(), 1, "{plans:#?}");
        assert!(plans[0].text.contains("$15"), "cut the price off");
        assert!(plans[0].prompt_text().contains("## Pro"));
        assert!(plans[0].text.chars().count() <= WINDOW_CHARS);
    }

    #[test]
    fn a_price_far_below_its_heading_is_not_a_plan_section() {
        // The limit of the rule, asserted rather than left to be discovered: `PLAN_FLOOR`
        // means *a price under its own name*, so a section that buries the number sixty
        // lines down does not qualify. Five real pricing pages put it in the first two, and
        // the single-window fallback still finds the number — it just does not claim to
        // know which plan the section was about.
        let long = "- One more feature, described at some length.\n".repeat(60);
        let page = format!("## Pro\n{long}$15/user, billed monthly\n");
        let plans = every_plan(&page);
        assert_eq!(plans.len(), 1, "{plans:#?}");
        assert_eq!(
            plans[0].heading, None,
            "claimed a plan name it had not earned"
        );
    }

    #[test]
    fn a_page_with_no_price_anywhere_produces_no_plans() {
        let page = "# About us\n\nWe were founded in 2019 and have 42 employees.";
        assert!(every_plan(page).is_empty());
    }

    #[test]
    fn a_page_that_prices_without_naming_a_plan_still_produces_a_window() {
        // No section states a price under its own name, so there are no plan blocks at all
        // — and this page still says something about pricing. Falling back to the single
        // best window keeps the finding rather than reporting silence.
        let page = "Everything is included. It costs $19 per month, and that is that.";
        let plans = every_plan(page);
        assert_eq!(plans.len(), 1, "{plans:#?}");
        assert!(plans[0].text.contains("$19"));
    }

    #[test]
    fn a_feature_table_with_no_prices_in_it_is_not_a_window() {
        // `todoist.com/pricing` renders its prices in JavaScript. What reaches the Markdown
        // is the feature-comparison table and nothing else, and forty table rows outscore
        // any real pricing block on structure alone. The window chosen from it produced
        // **"Beginner at $5"** — the 5 is how many personal projects the plan allows.
        //
        // A page that publishes no price we can see is a finding, and ARCHITECTURE §5.5's
        // JavaScript-gap counter is what it feeds. A window with no price in it cannot
        // produce that finding; it can only produce a guess.
        let mut page = String::from(
            "# Compare plans\n###\n\n|  | Beginner | Pro | Business |\n|---|---|---|---|\n",
        );
        for n in 0..40 {
            page.push_str(&format!(
                "| Feature number {n} | 5 | 300 | 300 for each member |\n"
            ));
        }
        assert!(every_plan(&page).is_empty(), "invented a pricing window");
    }

    #[test]
    fn a_bare_heading_is_not_a_plan_name() {
        // Same page: three empty `###` sit above those tables, and "shortest heading wins"
        // would hand the model `###` as the plan it is being asked about.
        let page = "###\n## Pro\n$15/user, billed monthly\n- Everything included";
        let plans = every_plan(page);
        assert_eq!(plans.len(), 1, "{plans:#?}");
        assert_eq!(plans[0].heading.as_deref(), Some("## Pro"));
    }

    #[test]
    fn a_page_of_many_sections_is_capped() {
        // Each window is a model call. A page producing thirty of them has mistaken
        // something else for a plan, and thirty calls is the wrong way to find that out.
        let mut page = String::new();
        for n in 0..30 {
            page.push_str(&format!(
                "## Plan {n}\n${n}9 per month, billed monthly\n- A feature\n"
            ));
        }
        assert_eq!(every_plan(&page).len(), MAX_PLANS);
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
