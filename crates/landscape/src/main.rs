//! The Landscape binary.
//!
//! One executable, three roles, chosen by argument:
//!
//! ```text
//! landscape dev       the API and a worker in one process (what you want locally)
//! landscape serve     the HTTP API alone
//! landscape worker    claims queued analyses and runs them
//! landscape migrate   applies migrations and exits
//! ```
//!
//! One binary rather than three keeps deployment to a single artefact on a machine with
//! 24GB of shared memory, and means the roles cannot drift out of version with each other.
//!
//! `--store memory` runs the whole thing with no database at all. That is what makes
//! `README.md`'s promise — the full application is testable locally — true on a machine
//! with nothing installed.

use std::sync::Arc;

use anyhow::{Context, Result};
use landscape_api::AppState;
use landscape_db::{MemoryStore, PgStore, Store};

mod progress;

/// Where the API listens unless `BIND_ADDR` says otherwise.
///
/// Deliberately **not** 8080. That is llama.cpp's default, and this application is designed
/// to run `llama-server` sidecars on the same machine — the architecture plans three
/// resident models, which will take 8080 upward. Defaulting the API there would collide
/// with the one process it cannot run without.
///
/// The rest of the map: 5173 Vite, 5432 Postgres, 8080+ llama-server.
const DEFAULT_ADDR: &str = "127.0.0.1:8787";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    /// API and worker together, sharing one store.
    ///
    /// This exists because `--store memory` gives each process its own map: a separate
    /// `serve` and `worker` would never see each other's queue, so the in-memory mode
    /// would only ever be half an application. Running both in one process is what makes
    /// "no database needed" true rather than nearly true.
    Dev,
    Serve,
    Worker,
    Migrate,
    /// Fetch one URL and report what every policy decided about it.
    ///
    /// A diagnostic, not a feature of the product. It exists because the fetch policies —
    /// the SSRF guard, `robots.txt`, the rate limit — are the kind of code that is easy to
    /// believe in and hard to observe, and "run it against a URL and watch" is the only
    /// honest way to convince somebody they work.
    Fetch,
    /// Measure the JavaScript-rendering gap over a list of pricing pages.
    ///
    /// ARCHITECTURE.md §5.5 refuses to schedule a headless browser until its size is known,
    /// and specifies two counters. This is those counters, run against real pages.
    Gap,
    /// Find the pages worth reading about one company.
    ///
    /// FACT_CHECKING §3.3's structured probes, sitemap and llms.txt, ranked and capped at 8.
    Discover,
    /// Ask a search engine for the pages probes could not reach.
    ///
    /// FACT_CHECKING §3.3 puts search *after* probes and says why: probes are deterministic,
    /// free and hit primary sources, so search fills gaps rather than leading. This command
    /// runs discovery first for exactly that reason — the queries it asks are the questions
    /// discovery came back empty on, and a company whose probes found everything asks none.
    ///
    /// **It prints the queries whether or not an engine is configured.** The query set is the
    /// auditable half of retrieval, and a laptop with no `SEARX_URL` should still be able to
    /// see what would be asked.
    Search,
    /// Check that the demo's curated ideas still have pages worth reading.
    ///
    /// The catalogue in `landscape-core` promises one thing about each domain: that discovery
    /// finds pages for the six questions. **A promise about the live web goes stale on
    /// somebody else's schedule** — a site drops its sitemap, moves pricing behind an anchor,
    /// or starts refusing our user agent — and the place that would be discovered is the demo.
    ///
    /// No model, and no database. This is discovery alone, which is what makes it something
    /// that can be run before sending somebody a link.
    Examples,
    /// What one company will cost a model, before any model is asked.
    ///
    /// **The wait is the last thing between this and a demo somebody will sit through**, and
    /// it is not the model's speed — it is how many times the pipeline asks. Every extractor
    /// works a window at a time and makes one call per window, so the number of windows on the
    /// pages a run will read *is* the run's model cost, and it can be counted with no model
    /// running at all.
    ///
    /// Needs no `llama-server` and no database. That is what makes it something to run before
    /// sending somebody a link rather than after they complain.
    Cost,
    /// Discover, fetch, convert and extract — the whole path, for one company.
    ///
    /// The first command that runs every piece in order. It exists because each piece has
    /// been testable alone for several weeks and the joins between them had never been run.
    Read,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Backing {
    Memory,
    Postgres,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                // Every workspace crate is named, because `landscape=info` does **not**
                // match `landscape_api` — an `EnvFilter` directive matches a target and
                // its `::` children, and `landscape_api` is a sibling, not a child.
                //
                // That cost an hour: request ids were being attached correctly and the
                // log lines carrying them were filtered out before they were written, so
                // the whole mechanism looked broken from the outside while every unit
                // test passed. A default that hides our own crates is not a default.
                .unwrap_or_else(|_| {
                    "landscape=info,landscape_api=info,landscape_db=info,\
                     landscape_llm=info,tower_http=info"
                        .into()
                }),
        )
        // Logs to stderr, so stdout stays free for anything a command needs to emit as
        // data. `fmt()` defaults to stdout, which would mean a future `landscape schema`
        // printing JSON with log lines spliced through it.
        .with_writer(std::io::stderr)
        // Colour codes are for a terminal. Anything reading this output through a pipe -
        // a log collector, or the test that parses the bound port - gets plain text.
        .with_ansi(std::io::IsTerminal::is_terminal(&std::io::stderr()))
        .init();

    let args: Vec<String> = std::env::args().skip(1).collect();
    let role = match args.first().map(String::as_str) {
        Some("dev") | None => Role::Dev,
        Some("serve") => Role::Serve,
        Some("worker") => Role::Worker,
        Some("migrate") => Role::Migrate,
        Some("fetch") => Role::Fetch,
        Some("gap") => Role::Gap,
        Some("discover") => Role::Discover,
        Some("search") => Role::Search,
        Some("examples") => Role::Examples,
        Some("cost") => Role::Cost,
        Some("read") => Role::Read,
        Some(other) => anyhow::bail!(
            "unknown command {other:?}.              Try: dev, serve, worker, migrate, fetch, gap, discover, search, read, examples, cost"
        ),
    };

    // Before any store is built: fetching a URL needs no database, and requiring
    // DATABASE_URL to run a diagnostic would make the diagnostic the harder thing.
    if role == Role::Fetch {
        return fetch_one(&args).await;
    }
    if role == Role::Gap {
        return measure_gap(&args).await;
    }
    if role == Role::Discover {
        return discover_sources(&args).await;
    }
    if role == Role::Search {
        return search_for_gaps(&args).await;
    }
    if role == Role::Examples {
        return check_examples().await;
    }
    if role == Role::Cost {
        return count_model_calls(&args).await;
    }
    if role == Role::Read {
        return read_company(&args).await;
    }

    let backing = if args.iter().any(|a| a == "--store=memory")
        || args
            .windows(2)
            .any(|w| w[0] == "--store" && w[1] == "memory")
    {
        Backing::Memory
    } else {
        Backing::Postgres
    };

    match backing {
        Backing::Memory => {
            if role == Role::Migrate {
                tracing::info!("nothing to migrate: the in-memory store has no schema");
                return Ok(());
            }
            tracing::warn!("using the in-memory store - nothing is saved when this stops");
            run(role, Arc::new(MemoryStore::new())).await
        }
        Backing::Postgres => {
            let url = std::env::var("DATABASE_URL").context(
                "DATABASE_URL is not set.\n\
                 Start one with:  docker compose up -d db\n\
                 Or run with no database at all:  cargo run -- serve --store memory",
            )?;
            let store = PgStore::connect(&url)
                .await
                .context("could not connect to Postgres. Is `docker compose up -d db` running?")?;
            store.migrate().await.context("migrations failed")?;
            if role == Role::Migrate {
                tracing::info!("migrations applied");
                return Ok(());
            }
            run(role, Arc::new(store)).await
        }
    }
}

/// `landscape fetch <url>` — run one URL past every policy and say what happened.
///
/// Prints to **stdout** as data, so it can be piped, while the reasoning goes to stderr as
/// ordinary logs. A diagnostic whose output cannot be piped is a diagnostic people
/// screenshot.
async fn fetch_one(args: &[String]) -> Result<()> {
    let url = args.get(1).filter(|a| !a.starts_with("--")).context(
        "usage: landscape fetch <url> [--markdown]

Examples:
  landscape fetch https://example.com/
  landscape fetch https://example.com/security --markdown   # what a golden page is made of",
    )?;

    let fetcher = landscape_fetch::Fetcher::new();
    let started = std::time::Instant::now();

    match fetcher.get(url).await {
        Ok(page) => {
            let ms = started.elapsed().as_millis();
            tracing::info!(status = page.status, took_ms = ms, "fetched");
            println!("url     {}", page.url);
            println!("status  {}", page.status);
            println!("bytes   {}", page.body.len());
            if let Some(tag) = &page.etag {
                println!("etag    {tag}");
            }
            println!("fetched {}", page.fetched_at.to_rfc3339());
            // **How a golden page is made.** `landscape-golden/pages/` holds real pages frozen
            // as Markdown, and until now the only account of where they came from was that
            // somebody had produced them. This is the step, so a page added next year is
            // converted by the same code that reads one in production.
            if args.iter().any(|a| a == "--markdown") {
                println!("---");
                print!("{}", landscape_extract::markdown::from_body(&page.body));
            }
            Ok(())
        }
        // Printed as an ordinary result rather than returned as an error: a refusal is this
        // command working, and exiting non-zero would make "the guard stopped me" look like
        // "the tool broke".
        Err(e) => {
            println!("refused {e}");
            Ok(())
        }
    }
}

/// `landscape gap <file>` — the two counters from ARCHITECTURE.md §5.5.
///
/// Takes a file of URLs, one per line, `#` for comments. A file rather than arguments
/// because the sample is the instrument: a measurement you cannot re-run against exactly
/// the same list is not one anybody can check.
async fn measure_gap(args: &[String]) -> Result<()> {
    let path = args.get(1).filter(|a| !a.starts_with("--")).context(
        "usage: landscape gap <file-of-urls>

Example:
  landscape gap docs/js-gap-sample.txt",
    )?;
    let listing =
        std::fs::read_to_string(path).with_context(|| format!("could not read {path}"))?;

    let urls: Vec<&str> = listing
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect();

    let fetcher = landscape_fetch::Fetcher::new();
    let mut report = landscape_extract::Report::default();

    for url in urls {
        match fetcher.get(url).await {
            Ok(page) => {
                let found = landscape_extract::locate(&page.body);
                tracing::info!(url, tier = found.tier(), "measured");
                report.readings.push(landscape_extract::Reading {
                    url: url.to_owned(),
                    found,
                });
            }
            // Kept apart from "no price found". A page we could not reach says nothing
            // about JavaScript, and folding the two together would inflate the gap with
            // our own failures — in the direction of more work for us.
            Err(e) => report.unreachable.push((url.to_owned(), e.to_string())),
        }
    }

    println!("{}", report.render());
    Ok(())
}

/// `landscape discover <origin>` — the pages worth reading about one company.
///
/// Takes an origin rather than a bare domain, because the scheme is part of what gets
/// fetched and guessing `https` for somebody would be the kind of silent assumption this
/// codebase keeps finding bugs in.
async fn discover_sources(args: &[String]) -> Result<()> {
    let origin = args.get(1).filter(|a| !a.starts_with("--")).context(
        "usage: landscape discover <origin>

Example:
  landscape discover https://basecamp.com",
    )?;

    let fetcher = landscape_fetch::Fetcher::new();
    let started = std::time::Instant::now();
    let found = landscape_discover::discover(&fetcher, origin).await;

    tracing::info!(
        sources = found.sources.len(),
        checked = found.checked.len(),
        took_s = started.elapsed().as_secs(),
        "discovery finished"
    );
    println!("{}", found.render());
    Ok(())
}

/// `landscape search <origin> [--name "Help Scout"]` — the gaps, the queries, and the pages.
///
/// Three things in order, because the third is the only one that needs infrastructure:
///
/// 1. **Which questions discovery left unanswered.** Discovery runs first every time. Search
///    that leads rather than fills is the failure FACT_CHECKING §3.3 is written to prevent,
///    and computing the gaps from a real run is what makes that structural here rather than
///    a comment.
/// 2. **The queries those gaps produce.** Printed whether or not an engine is configured, so
///    the auditable half of retrieval is visible on a laptop with nothing installed.
/// 3. **The pages, if `SEARX_URL` is set.** With their disposition, which says what each one
///    is allowed to be used for.
///
/// The name searched for defaults to the host, and `--name` overrides it. **That default is a
/// placeholder and is labelled as one**: turning *"an app that helps small farms sell to local
/// restaurants"* into a company's name is entity resolution, which is the next piece of work
/// and not this one. `landscape-core::subject` already holds the gate it will feed.
async fn search_for_gaps(args: &[String]) -> Result<()> {
    use landscape_discover::probes::Answers;

    let origin = args.get(1).filter(|a| !a.starts_with("--")).context(
        "usage: landscape search <origin> [--name \"Help Scout\"]

Examples:
  landscape search https://linear.app
  landscape search https://helpscout.com --name \"Help Scout\"

Set SEARX_URL to run the queries; without it the queries are printed and nothing is asked.",
    )?;

    let target = landscape_fetch::Target::parse(origin)
        .map_err(|e| anyhow::anyhow!("{origin} is not a URL we fetch: {e}"))?;
    let name = args
        .windows(2)
        .find(|w| w[0] == "--name")
        .map_or_else(|| target.host.clone(), |w| w[1].clone());

    let fetcher = landscape_fetch::Fetcher::new();
    let found = landscape_discover::discover(&fetcher, origin).await;

    const EVERY_QUESTION: [Answers; 6] = [
        Answers::Pricing,
        Answers::Features,
        Answers::Changes,
        Answers::Identity,
        Answers::Trust,
        Answers::Direction,
    ];
    let unanswered: Vec<Answers> = EVERY_QUESTION
        .into_iter()
        .filter(|q| !found.sources.iter().any(|s| s.answers == *q))
        .collect();

    println!("subject   {name}");
    println!("host      {}", target.host);
    println!(
        "answered  {}",
        EVERY_QUESTION
            .into_iter()
            .filter(|q| !unanswered.contains(q))
            .map(Answers::name)
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!(
        "gaps      {}",
        if unanswered.is_empty() {
            "none - probes answered every question, so nothing is searched for".to_owned()
        } else {
            unanswered
                .iter()
                .map(|q| q.name())
                .collect::<Vec<_>>()
                .join(", ")
        }
    );

    let queries = landscape_search::queries::for_questions(&name, &unanswered);
    println!("query set {}", landscape_search::QUERY_SET);
    for q in &queries {
        println!("  {:<10} {}", q.answers.name(), q.text);
    }
    if queries.is_empty() {
        return Ok(());
    }

    let Some(engine) = landscape_search::Searx::from_env() else {
        println!(
            "\nno engine configured - set {} to ask these. Nothing was sent anywhere.",
            landscape_search::searx::URL_VAR
        );
        return Ok(());
    };

    use landscape_search::SourceProvider as _;
    let mut results = Vec::new();
    for query in queries {
        match engine.search(&query).await {
            Ok(hits) => results.push((query, hits)),
            // A search that fails is not an analysis that fails. The section keeps the
            // coverage note it already had, which is what the report says today anyway.
            Err(e) => tracing::warn!(question = query.answers.name(), error = %e, "search failed"),
        }
    }

    let already: Vec<String> = found.sources.iter().map(|s| s.url.clone()).collect();
    let admitted = landscape_search::admit::admit(
        &target.host,
        &already,
        &results,
        engine.name(),
        landscape_discover::rank::CAP_RUNG_0,
    );

    println!("\n{:<58} answers      may set a table value", "page");
    println!("{}", "-".repeat(96));
    for f in &admitted {
        println!(
            "{:<58} {:<12} {} ({})",
            f.url,
            f.answers.name(),
            if f.disposition.may_set_a_table_value() {
                "yes"
            } else {
                "no"
            },
            f.disposition.code()
        );
    }
    if admitted.is_empty() {
        println!("(nothing new - every result was already found by discovery, or was not a page)");
    }
    Ok(())
}

/// `landscape cost <origin>` — how many model calls this company is worth, without a model.
///
/// Prints the pages a run would read **in the order it would read them**, the windows each one
/// holds, and the two numbers that decide whether somebody waits: **calls before the first
/// thing on screen**, and calls in total.
///
/// The per-call figure is not measured here and deliberately so — it belongs to the model and
/// the machine, `BENCHMARKS.md` has it, and multiplying is the reader's job. What this counts
/// is the part the pipeline decides.
async fn count_model_calls(args: &[String]) -> Result<()> {
    let origin = args.get(1).filter(|a| !a.starts_with("--")).context(
        "usage: landscape cost <origin>

Example:
  landscape cost https://basecamp.com",
    )?;

    let fetcher = landscape_fetch::Fetcher::new();
    let found = landscape_discover::discover(&fetcher, origin).await;
    let plan = landscape_analyze::plan(&found.sources);

    // Every admitted page fetched once, including the ones the run will not read - their cost
    // is the thing being compared against.
    let mut read_pages: Vec<(String, &str, usize, bool)> = Vec::new();
    for source in &found.sources {
        let (calls, yields) = match fetcher.get(&source.url).await {
            Ok(page) => {
                let markdown = landscape_extract::markdown::from_body(&page.body);
                (
                    landscape_analyze::model_calls_for(source.answers, &markdown),
                    landscape_analyze::may_yield_content(source.answers, &markdown),
                )
            }
            Err(_) => (0, false),
        };
        read_pages.push((source.url.clone(), source.answers.name(), calls, yields));
    }
    let cost_of = |url: &str| -> (usize, bool) {
        read_pages
            .iter()
            .find(|(u, _, _, _)| u == url)
            .map_or((0, false), |(_, _, calls, yields)| (*calls, *yields))
    };

    println!(
        "
{:<52} {:<10} {:>6}",
        "page, in reading order", "answers", "calls"
    );
    println!("{}", "-".repeat(72));
    for source in &plan.read {
        let (calls, yields) = cost_of(&source.url);
        println!(
            "{:<52} {:<10} {:>6}{}",
            trim(&source.url),
            source.answers.name(),
            calls,
            if yields && calls == 0 {
                "   <- content, no model"
            } else {
                ""
            }
        );
    }
    for source in &plan.skipped {
        let (calls, _) = cost_of(&source.url);
        println!(
            "{:<52} {:<10} {:>6}   <- not read, one page a question",
            trim(&source.url),
            source.answers.name(),
            calls
        );
    }
    println!("{}", "-".repeat(72));

    let now = landscape_analyze::tally(plan.read.iter().map(|c| cost_of(&c.url)));
    let before = landscape_analyze::tally(found.sources.iter().map(|c| cost_of(&c.url)));
    println!(
        "{:<44} {:>8} {:>18}",
        "", "in total", "before a first chance"
    );
    println!(
        "{:<44} {:>8} {:>18}",
        "every admitted page, discovery's order", before.0, before.1
    );
    println!(
        "{:<44} {:>8} {:>18}",
        "as this run reads them", now.0, now.1
    );
    println!(
        "
A chance of content, not a promise of one: whether a call answers is unknowable
without running it. `landscape read` prints the measured figure. Multiply by what one
call costs on your machine - that number belongs to the model, not to this pipeline."
    );
    Ok(())
}

/// A URL short enough for a column.
fn trim(url: &str) -> String {
    let bare = url
        .trim_start_matches("https://")
        .trim_start_matches("http://");
    if bare.chars().count() <= 50 {
        return bare.to_owned();
    }
    format!("{}…", bare.chars().take(49).collect::<String>())
}

/// `landscape examples` — run discovery against every company the demo offers.
///
/// **The curated list is the only part of this product that makes a promise about somebody
/// else's website**, and it is the part that will rot without anybody touching this
/// repository. This is the command that finds out, and it exits non-zero when an example has
/// lost the question a comparison is mostly about.
///
/// Pricing is the one treated as fatal. A missing changelog produces a coverage note that is a
/// real finding — *"no page found, checked /changelog, /releases"* — and reads as the product
/// working. A missing pricing page on every example turns the demo into a page of coverage
/// notes.
async fn check_examples() -> Result<()> {
    let fetcher = landscape_fetch::Fetcher::new();
    let mut poor: Vec<String> = Vec::new();

    for example in landscape_core::examples() {
        println!(
            "
{} - {}",
            example.id,
            example.prompt()
        );
        for company in &example.companies {
            let origin = format!("https://{company}");
            let started = std::time::Instant::now();
            let found = landscape_discover::discover(&fetcher, &origin).await;
            let mut answered: Vec<&str> = found
                .sources
                .iter()
                .map(|c| c.answers.name())
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect();
            answered.sort_unstable();
            let priced = answered.contains(&"pricing");
            println!(
                "  {:<24} {} source(s) in {}s, answering: {}{}",
                company,
                found.sources.len(),
                started.elapsed().as_secs(),
                if answered.is_empty() {
                    "nothing".to_owned()
                } else {
                    answered.join(", ")
                },
                if priced { "" } else { "   <- NO PRICING PAGE" }
            );
            if !priced {
                poor.push(format!("{company} (in {})", example.id));
            }
        }
    }

    println!();
    if poor.is_empty() {
        println!("every example still has a pricing page to read.");
        return Ok(());
    }
    // Non-zero, because a demo that has quietly stopped working is exactly the thing a
    // summary line at the end of a long output gets skimmed past.
    anyhow::bail!(
        "{} of the demo's companies no longer admit a pricing page: {}",
        poor.len(),
        poor.join(", ")
    )
}

/// `landscape read <origin>` — discovery, extraction, and the report they add up to.
///
/// The orchestration lives in `landscape-analyze`, which is the change that made this a
/// command rather than the pipeline. What stays here is the printing, and it prints two
/// things: **the run log first, then the report.**
///
/// The run log is ordered by page and shows every join — the window sizes, the dropped
/// answers, the quotes that were not verbatim. It is a diagnostic and the product will not
/// have it. The report is ordered by question, cites every claim, and says what was checked
/// where it has nothing. That one is the product.
async fn read_company(args: &[String]) -> Result<()> {
    let origin = args.get(1).filter(|a| !a.starts_with("--")).context(
        "usage: landscape read <origin>

Example:
  landscape read https://basecamp.com",
    )?;

    let fetcher = landscape_fetch::Fetcher::new();
    let llm = landscape_llm::LlamaClient::from_env();
    // The one place the clock is read. Everything downstream takes the time as an argument so
    // that what a report says about a 90-day window can be tested without waiting 90 days.
    let now = chrono::Utc::now();
    // **The two numbers `PRODUCT_SPEC.md` 2.1A actually asks for**, taken rather than
    // estimated: when the first claim reached whoever was waiting, and when the run ended.
    // `landscape cost` counts the model calls with no model running; this is the same run with
    // one, and it is the measurement a person takes for themselves.
    let started = std::time::Instant::now();
    let mut first_content: Option<std::time::Duration> = None;
    let analysis = landscape_analyze::analyse_with(
        &fetcher,
        &llm,
        origin,
        now,
        now.date_naive(),
        &mut |report: &landscape_core::Report| {
            if first_content.is_none() && report.sections.iter().any(|s| !s.claims.is_empty()) {
                first_content = Some(started.elapsed());
            }
            landscape_analyze::Wanted::Yes
        },
    )
    .await;
    let took = started.elapsed();

    println!(
        "{:<44} {:<6} {:<5} {:<6} extracted",
        "page", "words", "qual", "span"
    );
    println!("{}", "-".repeat(100));
    for page in &analysis.pages {
        println!(
            "{:<44} {:<6} {:<5} {:<6} {}",
            short(&page.url),
            page.words.map_or_else(|| "-".to_owned(), |w| w.to_string()),
            page.quality.unwrap_or("-"),
            page.window_words
                .map_or_else(|| "-".to_owned(), |w| w.to_string()),
            page.summary
        );
        for detail in &page.details {
            println!("{:<44}   {detail}", "");
        }
    }

    println!("\n{:<12} coverage", "question");
    println!("{}", "-".repeat(100));
    for coverage in &analysis.coverage {
        println!("{:<12} {}", coverage.question, coverage.note());
    }

    println!("\n{}", "=".repeat(100));
    println!("{}", analysis.render());

    // Last, because it is the line worth carrying away. Against PRODUCT_SPEC 2.1A: content in
    // twenty to forty seconds, the whole report inside ninety to a hundred and eighty.
    println!(
        "first content {} - whole report {:.0}s",
        first_content.map_or_else(
            || "never - nothing was extracted".to_owned(),
            |d| format!("{:.0}s", d.as_secs_f64())
        ),
        took.as_secs_f64()
    );
    Ok(())
}

fn short(url: &str) -> String {
    let s = url.strip_prefix("https://").unwrap_or(url);
    if s.chars().count() <= 42 {
        return s.to_owned();
    }
    s.chars().take(41).collect::<String>() + "…"
}

async fn run(role: Role, store: Arc<dyn Store>) -> Result<()> {
    match role {
        Role::Migrate => Ok(()),
        // Handled before any store exists; see `main`.
        Role::Fetch
        | Role::Gap
        | Role::Discover
        | Role::Search
        | Role::Read
        | Role::Examples
        | Role::Cost => Ok(()),
        Role::Serve => serve(store).await,
        Role::Worker => worker(store).await,
        Role::Dev => {
            // Both halves, one store. Whichever stops first ends the process, so a crash
            // in the worker is visible rather than leaving a server that never completes
            // anything.
            let worker_store = Arc::clone(&store);
            tokio::select! {
                r = serve(store) => r,
                r = worker(worker_store) => r,
            }
        }
    }
}

async fn serve(store: Arc<dyn Store>) -> Result<()> {
    let addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| DEFAULT_ADDR.to_owned());

    // Tracing lives inside `router()` alongside the request id, so the two cannot be
    // ordered wrongly here — see the note there.
    // The API and the built web app, when there is one. Without the app the binary serves JSON
    // and a visitor gets nothing to look at, which is what made the whole thing undeployable.
    // **No CORS layer at all**, and that is the tightening the comment here used to promise.
    //
    // It was `CorsLayer::permissive()` — every origin, every method, every header — with a note
    // saying it would be narrowed before deployment. Review pointed out what that costs the
    // moment there is a deployment: a page on any other site, opened by somebody whose address
    // is allowed through the firewall, can POST an analysis from their browser and spend the
    // box's only scarce resource. An allow-list at the network edge does not help, because the
    // request comes *from* the allowed machine.
    //
    // Nothing needs it. In production the binary serves the page and the API from one origin,
    // and `vite.config.ts` proxies `/api` in development — so the browser has never actually
    // been making a cross-origin request.
    let app = landscape_api::with_ui(AppState::new(store), &landscape_api::web_dir());

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .with_context(|| bind_failure_help(&addr))?;

    // Log what was actually bound, not what was asked for. With BIND_ADDR=127.0.0.1:0 the
    // OS picks the port, and the docs test relies on reading it back from this line.
    let bound = listener
        .local_addr()
        .map_or_else(|_| addr.clone(), |a| a.to_string());
    tracing::info!("listening on http://{bound}");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("server stopped unexpectedly")
}

/// What to say when the port is taken.
///
/// "Is something already using it?" is a question, and the person reading it cannot answer
/// it without knowing the incantation for their platform. So give them the incantation,
/// and name the process most likely to be responsible — during development on this project
/// that is nearly always `llama-server`, which defaults to 8080.
fn bind_failure_help(addr: &str) -> String {
    let port = addr.rsplit(':').next().unwrap_or(addr);
    let finder = if cfg!(windows) {
        format!("netstat -ano | findstr :{port}      then: tasklist /FI \"PID eq <pid>\"")
    } else {
        format!("lsof -i :{port}")
    };
    format!(
        "could not bind {addr} - something else is already listening there.\n\n\
         Find it with:\n    {finder}\n\n\
         Then either stop it, or pick another port:\n    \
         BIND_ADDR=127.0.0.1:8788 cargo run -- dev --store memory\n\n\
         Note: port 8080 is llama.cpp's default, so a running llama-server is the usual \
         culprit. This binary avoids 8080 for that reason."
    )
}

/// Claim queued analyses and run them.
///
/// The loop polls rather than listening for a notification. At the volumes this starts
/// with, a one-second poll costs nothing measurable and removes a whole class of
/// missed-wakeup bug; `LISTEN`/`NOTIFY` is worth adding when the queue is busy enough for
/// the latency to matter.
/// How long an analysis may be `running` before another worker may take it.
///
/// Generous on purpose. Reclaiming too early hands a second worker a job the first is
/// still doing, which is worse than a slow one: the reader gets a report built twice, and
/// the machine spends prefill it does not have. Twenty minutes is well past the 90-180s a
/// healthy analysis should take, so anything beyond it is a dead worker rather than a slow
/// one.
const STALE_AFTER: i64 = 20 * 60;

async fn worker(store: Arc<dyn Store>) -> Result<()> {
    tracing::info!("worker started");
    let mut shutdown = Box::pin(shutdown_signal());
    let mut sweep = tokio::time::interval(std::time::Duration::from_secs(60));

    loop {
        // Cheap and idempotent: several workers running this concurrently is harmless,
        // because the UPDATE only matches rows that are still stale when it runs.
        if sweep.poll_tick(&mut std::task::Context::from_waker(std::task::Waker::noop()))
            != std::task::Poll::Pending
        {
            match store
                .reclaim_stale(chrono::Duration::seconds(STALE_AFTER))
                .await
            {
                Ok(0) => {}
                Ok(n) => tracing::warn!(count = n, "requeued analyses whose worker died"),
                Err(e) => tracing::error!(error = %e, "could not sweep for stale analyses"),
            }
        }

        tokio::select! {
            () = &mut shutdown => {
                tracing::info!("worker stopping");
                return Ok(());
            }
            claimed = store.claim_next() => {
                match claimed {
                    Err(e) => {
                        tracing::error!(error = %e, "could not claim work");
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    }
                    Ok(None) => {
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    }
                    Ok(Some(analysis)) => {
                        run_analysis(&store, &analysis).await;
                    }
                }
            }
        }
    }
}

/// A report shaped exactly like a real one, containing only what is actually known.
///
/// The pipeline that fills this in is Phase 1. Until then it returns a report whose every
/// section is `not_found` with the pages it would have checked listed — which is the same
/// treatment a real run gives a genuine gap, so the frontend renders the honest case from
/// day one rather than a happy path that has to be unwritten later.
/// Run one claimed analysis, writing progress as it goes.
///
/// **The queue finally carries something.** Until now this wrote a placeholder saying the
/// gathering pipeline was not built; it is built, and this is the join.
///
/// Progress is saved after every page rather than at the end, which is what lets a reader
/// watch a report fill in — `PRODUCT_SPEC.md` §2.1A. A failed save is logged and the run
/// continues: losing an intermediate write costs a reader a few seconds of staleness, and
/// abandoning the analysis over it would cost them the report.
async fn run_analysis(store: &Arc<dyn Store>, analysis: &landscape_core::Analysis) {
    let origins = landscape_analyze::subject::origins_in(&analysis.prompt);
    let Some(first) = origins.first().cloned() else {
        // Not an error — a capability we do not have. Guessing a domain from a description
        // would produce a report that is correctly cited and about the wrong company.
        tracing::info!(id = %analysis.id, "no subject in prompt");
        match store
            .fail(
                analysis.id,
                analysis.generation,
                landscape_core::Failure::NoSubject,
                landscape_analyze::subject::NO_SUBJECT,
            )
            .await
        {
            Ok(landscape_core::Applied::ClaimRevoked) => {
                tracing::warn!(id = %analysis.id, "claim revoked before the failure was recorded");
            }
            Ok(landscape_core::Applied::Yes) => {}
            Err(e) => {
                tracing::error!(id = %analysis.id, error = %e, "could not record the failure");
            }
        }
        return;
    };

    tracing::info!(
        id = %analysis.id,
        subjects = origins.len(),
        first = %first,
        "running analysis"
    );
    let fetcher = landscape_fetch::Fetcher::new();
    let llm = landscape_llm::LlamaClient::from_env();
    let now = chrono::Utc::now();

    let progress = progress::Progress::new(Arc::clone(store), analysis.id, analysis.generation);
    let outcome = landscape_analyze::analyse_many(
        &fetcher,
        &llm,
        &origins,
        now,
        now.date_naive(),
        &mut |so_far| progress.record(so_far),
    )
    .await;
    // Before `complete`, so the last progress write cannot land after the finished report.
    if progress.finish().await == progress::Outcome::Revoked {
        // The sweep decided this run was dead and gave it to somebody else while we were
        // still working on it. Saying so is the whole value of the generation: without it,
        // this worker would finish, overwrite a live run's report with its own, and nothing
        // anywhere would record that two workers had produced two reports for one reader.
        //
        // **And there is nothing left to do.** `complete` would be refused for the same
        // reason every write since has been, so making the call would buy a log line that
        // says what this one already says. The pages this run did not read are the saving.
        tracing::warn!(
            id = %analysis.id,
            generation = analysis.generation,
            pages_read = outcome.pages.len(),
            abandoned = outcome.stopped_early,
            "this run was reclaimed while we were working on it; discarding what it produced"
        );
        return;
    }

    match store
        .complete(analysis.id, analysis.generation, &outcome.report)
        .await
    {
        Ok(landscape_core::Applied::ClaimRevoked) => {
            // Reachable when the sweep lands between the last progress write and this one:
            // no write had been refused yet, so nothing told the run to stop.
            tracing::warn!(id = %analysis.id, "finished a run that had been reclaimed; discarded");
        }
        Ok(landscape_core::Applied::Yes) => {}
        Err(e) => {
            tracing::error!(id = %analysis.id, error = %e, "could not save report");
        }
    }
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
// Panicking IS how a test reports failure. The lints stay denied everywhere else.
mod tests {
    use super::*;

    #[tokio::test]
    async fn a_prompt_that_names_no_site_fails_with_a_reason() {
        // The path this replaced wrote a placeholder report saying the pipeline was not
        // built. The pipeline is built, and the honest failure for a description is that we
        // cannot find the company from it yet — not a report about a guessed domain.
        let store: Arc<dyn Store> = Arc::new(MemoryStore::new());
        let new = landscape_core::NewAnalysis::parse("an app for farm-to-restaurant orders")
            .expect("valid prompt");
        let queued = store.enqueue(&new).await.expect("enqueue");
        let claimed = store
            .claim_next()
            .await
            .expect("claim")
            .expect("one queued");

        run_analysis(&store, &claimed).await;

        let done = store.get(queued.id).await.expect("get");
        assert_eq!(
            done.status,
            landscape_core::AnalysisStatus::Failed,
            "a subject we cannot resolve is a failure, not an empty report"
        );
        assert!(
            done.report.is_none(),
            "nothing was read, so there is nothing to show"
        );
    }

    #[tokio::test]
    async fn a_queued_analysis_is_claimed_exactly_once() {
        // The queue's own guarantee, with no database and no network.
        let store = MemoryStore::new();
        let new = landscape_core::NewAnalysis::parse("compare basecamp.com for me")
            .expect("valid prompt");
        let queued = store.enqueue(&new).await.expect("enqueue");

        let claimed = store
            .claim_next()
            .await
            .expect("claim")
            .expect("one queued");
        assert_eq!(claimed.id, queued.id);
        assert_eq!(claimed.status, landscape_core::AnalysisStatus::Running);
        assert!(
            store.claim_next().await.expect("claim again").is_none(),
            "a claimed analysis is never handed out twice"
        );
    }

    #[test]
    fn the_subject_comes_from_the_prompt_or_the_run_stops() {
        // The two branches `run_analysis` takes, asserted without a runtime: a named site is
        // read, and a description is refused.
        assert_eq!(
            landscape_analyze::subject::origin_in("compare basecamp.com for me").as_deref(),
            Some("https://basecamp.com")
        );
        assert!(landscape_analyze::subject::origin_in("an app for farms").is_none());
    }
}
