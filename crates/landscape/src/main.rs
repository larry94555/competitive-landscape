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
    /// Turn a description into the companies it might be about.
    ///
    /// FACT_CHECKING §3.1 step 2, and the thing the disambiguation gate in
    /// `landscape-core::subject` has been waiting for since Phase 1. Prints the queries a
    /// description produces whether or not an engine is configured, then — with `SEARX_URL` —
    /// the companies, their scores, and the gate's verdict.
    Candidates,
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
        Some("candidates") => Role::Candidates,
        Some("examples") => Role::Examples,
        Some("cost") => Role::Cost,
        Some("read") => Role::Read,
        Some(other) => anyhow::bail!(
            "unknown command {other:?}.              Try: dev, serve, worker, migrate, fetch, gap, discover, search, candidates, read, examples, cost"
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
    if role == Role::Candidates {
        return suggest_candidates(&args).await;
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
/// A configured engine, as the thing an analysis takes.
///
/// One line, and it exists so the two call sites cannot disagree about it: `Option<&Searx>` to
/// `Option<&dyn SourceProvider>` is a coercion that has to be spelled out, and spelling it out
/// twice is how one of them ends up passing `None` for ever without anybody noticing.
fn searching(engine: &landscape_search::Searx) -> &dyn landscape_search::SourceProvider {
    engine
}

/// `landscape candidates "<description>"` — the companies a description might be about.
///
/// **The step that has been missing.** `landscape-core::subject::resolve` — the disambiguation
/// gate — was built in Phase 1 before anything could feed it, deliberately: written afterwards
/// it would be a check bolted onto a pipeline that already ran without it. This is what feeds
/// it, and running the two together is the point of the command.
///
/// Four things in order, and only the third needs infrastructure:
///
/// 1. **The queries the description produces.** Templated and versioned, printed with no engine
///    configured, because the auditable half of retrieval should be visible on a laptop.
/// 2. **The hosts, grouped and scored**, with the arithmetic shown — how many queries agreed,
///    and how shallow the shallowest URL was. A score nobody can explain makes the gate's
///    decision unaccountable.
/// 3. **The name and the line a reader would choose between**, fetched from each company's own
///    front page rather than taken from the engine's title.
/// 4. **The gate's verdict**: resolved, ambiguous, or nothing found.
async fn suggest_candidates(args: &[String]) -> Result<()> {
    let description = args.get(1).filter(|a| !a.starts_with("--")).context(
        "usage: landscape candidates \"<description>\" | landscape candidates <domain>

Examples:
  landscape candidates \"privacy-friendly website analytics\"
  landscape candidates \"a shared inbox for a small support team\"
  landscape candidates basecamp.com

A description is searched for; a domain has its competitors searched for instead.
Set SEARX_URL to run the queries; without it the queries are printed and nothing is asked.",
    )?;

    // **The same reading the worker does**, not a second one that agrees by coincidence.
    if let landscape_analyze::subject::Subjects::Seed(origin) =
        landscape_analyze::subject::subjects_in(description)
    {
        return rivals_of_a_named_company(&origin).await;
    }

    println!("idea      {description}");
    println!("query set {}", landscape_search::candidates::IDEA_QUERY_SET);
    let queries = landscape_search::candidates::for_idea(description);
    for q in &queries {
        println!("  {}", q.text);
    }
    if queries.is_empty() {
        println!("\nNothing to ask: a description with no words in it would send bare");
        println!("boilerplate to an engine, which returns the internet.");
        return Ok(());
    }

    let Some(engine) = landscape_search::Searx::from_env()? else {
        // Correct here with no condition: `queries` is non-empty, guaranteed by the early
        // return four lines above.
        println!(
            "\n{} is not set, so nothing was asked. The queries above are what would go.",
            landscape_search::searx::URL_VAR
        );
        return Ok(());
    };

    let (found, queried) = landscape_search::candidates::suggest(&engine, description).await;
    println!(
        "\n{:<34} {:>7} {:>8}  shallowest",
        "company", "agreed", "score"
    );
    println!("{}", "-".repeat(78));
    for one in &found {
        println!(
            "{:<34} {:>3}/{:<3} {:>8.2}  {}",
            one.host,
            one.agreed,
            queries.len(),
            one.confidence,
            one.shallowest
        );
    }
    if found.is_empty() {
        println!("(nothing that looked like a company)");
    }
    if !queried.failed.is_empty() {
        println!(
            "\n{} of {} queries did not complete, so this list is thinner than it would be - and \
             anything below saying nothing was found is about us, not about the market:",
            queried.failed.len(),
            queried.sent()
        );
        for q in &queried.failed {
            println!("  did not complete: {q}");
        }
    }

    // The names come from each company's own front page. An engine's title is the engine's.
    let fetcher = landscape_fetch::Fetcher::new();
    let words = landscape_search::competitors::content_words(description);
    let named = landscape_search::candidates::describe(&found, &words, |url| {
        let fetcher = &fetcher;
        async move {
            fetcher
                .get(&url)
                .await
                .ok()
                .map(|page| landscape_extract::markdown::from_body(&page.body))
        }
    })
    .await;

    println!("\nas a reader would see them");
    for d in &named {
        println!("  {} - {}", d.candidate.name, d.candidate.canonical_domain);
        if !d.candidate.what_it_is.is_empty() {
            println!("      {}", d.candidate.what_it_is);
        }
    }

    // **What came back, not what was sent.** The list a reader is shown as evidence of the
    // looking has to be the looking that happened.
    let checked = queried.completed.clone();
    let set = landscape_search::competitors::assemble(named.clone(), queried.sent(), &words);
    let verdict = landscape_core::subject::resolve(
        description,
        named.into_iter().map(|d| d.candidate).collect(),
        checked,
    );

    println!("\nwords     {}", words.join(", "));
    println!(
        "reading   {}",
        if landscape_search::competitors::about_a_market(&words) {
            "a market - several companies matching it is the answer"
        } else {
            "a name - several products matching it is a question"
        }
    );

    // **The same function the worker calls.** A diagnostic that agreed with the worker only by
    // coincidence was how the first version of this command drifted.
    match landscape_analyze::subject::decide(
        landscape_search::competitors::Derived {
            verdict,
            set,
            about_a_market: landscape_search::competitors::about_a_market(&words),
        },
        &queried,
    ) {
        landscape_analyze::subject::Decided::Analyse(set) => {
            for line in set_as_printed(&set) {
                println!("{line}");
            }
        }
        landscape_analyze::subject::Decided::Refuse(refusal) => {
            println!(
                "\nno report ({}): {}",
                refusal.kind.as_db_str(),
                refusal.why
            );
            for choice in &refusal.choices {
                println!(
                    "  pick: {} ({}) - send \"{}\"",
                    choice.name, choice.domain, choice.prompt
                );
            }
        }
    }
    Ok(())
}

/// `landscape candidates <domain>` — who a named company competes with.
///
/// The diagnostic for the other half of competitor-set derivation. It prints the queries
/// whether or not an engine is configured, because the queries are the part worth reviewing:
/// `FACT_CHECKING.md` P22 wants them templated from a **resolved entity**, and this is the
/// path that can actually comply.
/// What this command prints about the queries — including when there will not be any.
///
/// **A function returning lines, so it can be asserted against the decision it describes.** The
/// worker skips every query when [`landscape_search::candidates::Seed::words`] returns an error;
/// this printed the queries regardless, gated on a *different* condition (`!seed.read`). For a
/// reachable page with no prose the two disagreed: three queries listed, none sent, and nothing
/// on screen saying so. A diagnostic that agrees with the worker by coincidence is worse than no
/// diagnostic, because it is believed.
///
/// Both now read `words()`, and the test below asserts this section reports "nothing asked"
/// exactly when that call fails.
fn what_would_be_asked(seed: &landscape_search::candidates::Seed) -> Vec<String> {
    let words = match seed.words() {
        Ok(words) => words,
        Err(why) => {
            return vec![
                format!("asking    nothing - {}", why.sentence()),
                String::new(),
                "A rival is admitted by sharing a word with this company's own description of"
                    .to_owned(),
                "itself. There is none, so no query goes out and no other company's front page"
                    .to_owned(),
                "is fetched: excluding real companies on evidence we never had would be worse"
                    .to_owned(),
                "than saying nothing.".to_owned(),
            ];
        }
    };

    let mut said = vec![
        format!("matching  {}", words.join(", ")),
        format!(
            "query set {}",
            landscape_search::competitors::RIVAL_QUERY_SET
        ),
    ];
    said.extend(
        landscape_search::competitors::for_company(&seed.candidate.name)
            .into_iter()
            .map(|q| format!("  {}", q.text)),
    );
    said.push(String::new());
    said.push(
        "Those are templated from the company's own name for itself, which is what".to_owned(),
    );
    said.push("FACT_CHECKING P22 asks for. Nothing you typed reaches the engine.".to_owned());
    said
}

/// The set as this command prints it, **including why nobody else is in it**.
///
/// One function for both paths — a description's set and a named company's — because they are
/// the same thing to a reader and the last two review rounds were about a diagnostic drifting
/// from the code it describes. `set.alone` is the report's own honest empty; a diagnostic that
/// showed the comparison but not the reason for its size would be showing less than the report.
fn set_as_printed(set: &landscape_search::competitors::Set) -> Vec<String> {
    let mut said = vec!["\nthe report would compare".to_owned()];
    for m in &set.members {
        said.push(format!(
            "  {} ({})",
            m.candidate.name, m.candidate.canonical_domain
        ));
        said.push(format!("      {}", m.because.sentence()));
    }
    for (c, why) in &set.set_aside {
        said.push(format!(
            "  not compared: {} ({})",
            c.name, c.canonical_domain
        ));
        said.push(format!("      {}", why.sentence()));
    }
    if let Some(why) = set.alone.as_ref() {
        said.push(String::new());
        said.push(why.sentence());
    }
    said
}

/// What this command says when no engine is configured.
///
/// **Two different facts, and the footer used to state only one.** With no vocabulary there are
/// no queries, so *"the queries above are what would go"* pointed at nothing — and worse, it
/// implied that setting `SEARX_URL` would change the outcome. It would not: this company's own
/// page gave nothing to compare a rival against, so an engine would be asked nothing either.
///
/// A reader who acts on a diagnostic and gets the same silence has been sent somewhere for
/// nothing, which is the failure this whole command exists to prevent.
///
/// It reads [`landscape_search::candidates::Seed::words`], like everything else that decides
/// anything about a seed.
fn without_an_engine(seed: &landscape_search::candidates::Seed) -> String {
    let var = landscape_search::searx::URL_VAR;
    if seed.words().is_err() {
        return format!(
            "{var} is not set, so nothing was asked - and nothing would have been asked with \
             it either, for the reason above."
        );
    }
    format!("{var} is not set, so nothing was asked. The queries above are what would go.")
}

async fn rivals_of_a_named_company(origin: &str) -> Result<()> {
    let target = landscape_fetch::Target::parse(origin)
        .map_err(|e| anyhow::anyhow!("{origin} is not a URL we fetch: {e}"))?;
    let fetcher = landscape_fetch::Fetcher::new();
    let read = |url: String| {
        let fetcher = &fetcher;
        async move {
            fetcher
                .get(&url)
                .await
                .ok()
                .map(|page| landscape_extract::markdown::from_body(&page.body))
        }
    };
    let seed = landscape_search::candidates::named_seed(&target.host, read).await;

    println!(
        "company   {} ({})",
        seed.candidate.name, seed.candidate.canonical_domain
    );
    println!("it says   {}", seed.candidate.what_it_is);
    for line in what_would_be_asked(&seed) {
        println!("{line}");
    }

    let Some(engine) = landscape_search::Searx::from_env()? else {
        println!("\n{}", without_an_engine(&seed));
        return Ok(());
    };

    let (set, queried) = landscape_search::competitors::of_company(&engine, &seed, read).await;
    if !queried.failed.is_empty() {
        println!(
            "\n{} of {} queries did not complete, so this list is thinner than it would be:",
            queried.failed.len(),
            queried.sent()
        );
        for q in &queried.failed {
            println!("  did not complete: {q}");
        }
    }

    for line in set_as_printed(&set) {
        println!("{line}");
    }
    Ok(())
}

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

    // `Ok(None)` is "no engine configured", which is the ordinary state on a laptop. An
    // `Err` is "you set the variable and it did not work", and flattening the two would send
    // somebody to the wrong file.
    let Some(engine) = landscape_search::Searx::from_env()? else {
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
call costs on your machine - that number belongs to the model, not to this pipeline.

This counts the pages discovery planned. With SEARX_URL set, a run reads up to {}
more - the questions these pages leave empty are searched for, and which ones those
are is not knowable before the reading. A prediction that quietly omitted them would
be a prediction of a different program.",
        landscape_analyze::PAGES_FROM_SEARCH
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
    // **The same engine `landscape search` uses, or none.** Without `SEARX_URL` the run reads
    // exactly the pages discovery planned and the report says so, which is the laptop default
    // and the behaviour every test in this repository runs against.
    let engine = landscape_search::Searx::from_env()?;
    let analysis = landscape_analyze::analyse_with(
        &fetcher,
        &llm,
        origin,
        now,
        now.date_naive(),
        engine.as_ref().map(searching),
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
        | Role::Candidates
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
/// Ask the search channel who a description is about, and let the gate decide.
///
/// **Every branch that is not one company is a refusal**, and each says a different thing: no
/// engine configured, nothing found, several found, or a search that did not finish. A reader
/// can act on all four, and they are four different actions —
/// [`landscape_analyze::subject::decide`] is where that mapping lives and is tested.
async fn resolve_from_description(
    engine: Option<&dyn landscape_search::SourceProvider>,
    prompt: &str,
) -> landscape_analyze::subject::Decided {
    let Some(engine) = engine else {
        return landscape_analyze::subject::Decided::Refuse(landscape_analyze::subject::Refusal {
            why: landscape_analyze::subject::NO_SUBJECT.to_owned(),
            kind: landscape_core::Failure::NoSubject,
            choices: Vec::new(),
        });
    };
    let fetcher = landscape_fetch::Fetcher::new();
    let (derived, queried) = landscape_search::candidates::for_description(engine, prompt, |url| {
        let fetcher = &fetcher;
        async move {
            fetcher
                .get(&url)
                .await
                .ok()
                .map(|page| landscape_extract::markdown::from_body(&page.body))
        }
    })
    .await;
    if !queried.failed.is_empty() {
        tracing::warn!(
            failed = queried.failed.len(),
            sent = queried.sent(),
            "some candidate searches did not complete"
        );
    }
    landscape_analyze::subject::decide(derived, &queried)
}

/// The companies a named one competes with, and why there are none when there are none.
///
/// **Always a set, never `None`.** It used to return `None` when no engine was configured, and
/// the caller quietly analysed the one company — a perfectly good answer to what a reader typed,
/// with nothing on the page saying we had not looked. Saying so is
/// [`landscape_search::competitors::NoRivals`], and it is the row this function was left
/// half-done for: four different reasons a report covers one company, and a reader acts on each
/// of them differently.
async fn rivals_of(
    engine: Option<&dyn landscape_search::SourceProvider>,
    origin: &str,
) -> landscape_search::competitors::Set {
    let host = match landscape_fetch::Target::parse(origin) {
        Ok(target) => target.host,
        // Unreachable: `Subjects::Seed` comes from `origins_in`, which only produces parseable
        // origins. A refusal rather than an `expect`, because a panic here takes a worker down.
        Err(e) => {
            tracing::error!(%origin, error = %e, "a named origin did not parse");
            return landscape_search::competitors::Set::default();
        }
    };
    let fetcher = landscape_fetch::Fetcher::new();
    let read = |url: String| {
        let fetcher = &fetcher;
        async move {
            fetcher
                .get(&url)
                .await
                .ok()
                .map(|page| landscape_extract::markdown::from_body(&page.body))
        }
    };
    let seed = landscape_search::candidates::named_seed(&host, read).await;
    let Some(engine) = engine else {
        // **The laptop default, and now it says so.** A reader who gets a one-company report
        // deserves to know whether we looked and found nobody or never looked at all; only one
        // of those is fixed by configuring something.
        return landscape_search::competitors::Set {
            members: vec![landscape_search::competitors::Member {
                candidate: seed.candidate.clone(),
                because: landscape_search::competitors::Because::Named,
            }],
            set_aside: Vec::new(),
            alone: Some(
                landscape_search::competitors::NoRivals::when_no_engine_is_configured(&seed),
            ),
        };
    };
    let (set, queried) = landscape_search::competitors::of_company(engine, &seed, read).await;
    if !queried.failed.is_empty() {
        tracing::warn!(
            failed = queried.failed.len(),
            sent = queried.sent(),
            "some competitor searches did not complete"
        );
    }
    set
}

/// Record a refusal a reader can act on, and say so if the claim was taken away first.
async fn refuse(
    store: &Arc<dyn Store>,
    analysis: &landscape_core::Analysis,
    refusal: &landscape_analyze::subject::Refusal,
) {
    match store
        .fail(
            analysis.id,
            analysis.generation,
            landscape_db::Refused {
                kind: refusal.kind,
                reason: &refusal.why,
                choices: &refusal.choices,
            },
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
}

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
    // **Read once, for both callers.** Resolving a description and filling a company's gaps
    // are two questions for the same engine, and asking the environment twice is two answers
    // that can disagree — the second source of truth this codebase keeps deleting.
    //
    // A misconfigured `SEARX_URL` is a configuration error, not a reason to fail an analysis
    // somebody is waiting for: the run proceeds without an engine, and every surface says so.
    let engine = landscape_search::Searx::from_env().unwrap_or_else(|e| {
        tracing::warn!(error = %e, "SEARX_URL is set but unusable; this run will not search");
        None
    });
    let searching = engine.as_ref().map(searching);

    // **Three readings of one box**, and the rule lives in `subject::subjects_in` rather than
    // here because it is a decision about what somebody meant. See [`Subjects`] for why
    // naming two companies is an instruction and naming one is a starting point.
    let (origins, set) = match landscape_analyze::subject::subjects_in(&analysis.prompt) {
        // **A description is no longer the end of the road.** Until Run 29 a prompt naming no
        // domain was refused; the channel now produces candidates and hands them to the gate
        // `FACT_CHECKING.md` §3.1 built before anything could feed it.
        landscape_analyze::subject::Subjects::Describe => {
            match resolve_from_description(searching, &analysis.prompt).await {
                landscape_analyze::subject::Decided::Analyse(set) => {
                    let origins = set.origins();
                    tracing::info!(
                        id = %analysis.id,
                        companies = origins.len(),
                        "a description became a competitor set"
                    );
                    (origins, Some(*set))
                }
                landscape_analyze::subject::Decided::Refuse(refusal) => {
                    tracing::info!(
                        id = %analysis.id,
                        kind = refusal.kind.as_db_str(),
                        choices = refusal.choices.len(),
                        "no subject"
                    );
                    refuse(store, analysis, &refusal).await;
                    return;
                }
            }
        }
        // **One company named is a competitive landscape asked for, not a profile.** With no
        // engine this is exactly what it always was - the laptop default, unchanged - and
        // saying so at the level of a whole set is its own roadmap row.
        landscape_analyze::subject::Subjects::Seed(origin) => {
            let set = rivals_of(searching, &origin).await;
            let origins = set.origins();
            tracing::info!(
                id = %analysis.id,
                %origin,
                companies = origins.len(),
                alone = ?set.alone,
                "a named company seeded a report"
            );
            (origins, Some(set))
        }
        // Several named: the reader has said what to compare, and adding to it would be
        // overruling them.
        landscape_analyze::subject::Subjects::Exactly(named) => (named, None),
    };

    let Some(first) = origins.first().cloned() else {
        // Unreachable: `Decided::Analyse` always carries at least one member and a named
        // prompt is non-empty.
        // Kept as a refusal rather than an `expect`, because a panic in a worker takes the
        // process down and a reader's run with it.
        tracing::error!(id = %analysis.id, "an empty subject list reached the reading");
        match store
            .fail(
                analysis.id,
                analysis.generation,
                landscape_db::Refused {
                    kind: landscape_core::Failure::NoSubject,
                    reason: landscape_analyze::subject::NO_SUBJECT,
                    choices: &[],
                },
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
        &landscape_analyze::Asked {
            origins: &origins,
            now,
            today: now.date_naive(),
            search: searching,
            set: set.as_ref(),
        },
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

    /// A seed built by hand, so the three shapes can be compared without a network.
    fn seed(read: bool, what_it_is: &str) -> landscape_search::candidates::Seed {
        landscape_search::candidates::Seed {
            candidate: landscape_core::subject::Candidate {
                name: "Basecamp".to_owned(),
                canonical_domain: "basecamp.com".to_owned(),
                what_it_is: what_it_is.to_owned(),
                confidence: 1.0,
            },
            read,
        }
    }

    #[test]
    fn the_diagnostic_says_nothing_is_asked_exactly_when_nothing_is_asked() {
        // **Review found these two disagreeing.** `of_company` skips every query when
        // `Seed::words()` fails; this section was gated on `!seed.read` instead, so a reachable
        // page with no prose printed all three queries while the worker sent none. A diagnostic
        // that agrees with the worker by coincidence is worse than none, because it is believed.
        //
        // The assertion is the equality itself, not a wording: whatever `words()` decides, this
        // section has to report the same thing.
        for (read, what_it_is) in [
            (true, "Project management and team communication"),
            (true, ""),
            (false, "we were unable to read its front page"),
        ] {
            let seed = seed(read, what_it_is);
            let said = what_would_be_asked(&seed).join("\n");
            let nothing_asked = said.contains("asking    nothing");
            assert_eq!(
                nothing_asked,
                seed.words().is_err(),
                "the diagnostic and the worker disagree for read={read}, what_it_is={what_it_is:?}:\n{said}"
            );
            assert_eq!(
                nothing_asked,
                !said.contains("Basecamp alternatives"),
                "a query was shown for a run that would send none, or hidden from one that would:\n{said}"
            );
        }
    }

    #[tokio::test]
    async fn no_engine_and_no_vocabulary_never_recommends_configuring_one() {
        // **Review found the report sending a reader somewhere that could not help.** With the
        // seed's own page unreadable, `seed.words()` already fails - so configuring `SEARX_URL`
        // would change nothing: `of_company` would send zero queries and say so. The report
        // said *"configure an engine"* while the diagnostic beside it said an engine would not
        // have been asked either.
        //
        // `.invalid` is RFC 2606, so the seed's front page is never really fetched.
        let set = rivals_of(None, "https://basecamp.invalid").await;

        assert_eq!(set.members.len(), 1, "{:#?}", set.members);
        assert_eq!(
            set.alone,
            Some(landscape_search::competitors::NoRivals::NothingToCompare(
                landscape_search::candidates::NoVocabulary::Unreadable
            )),
            "an engine was blamed for a page we could not read"
        );
        let said = set.alone.expect("a reason").sentence();
        assert!(
            !said.contains("no search engine is configured"),
            "a remedy that cannot work was recommended: {said}"
        );
        assert!(
            said.contains("says nothing about who else is out there"),
            "{said}"
        );
    }

    #[test]
    fn the_engine_is_only_blamed_when_the_engine_is_the_problem() {
        // Both directions of the precedence, on the function both surfaces read.
        let readable = seed(true, "Project management and team communication");
        assert_eq!(
            landscape_search::competitors::NoRivals::when_no_engine_is_configured(&readable),
            landscape_search::competitors::NoRivals::NoEngine
        );
        for (read, what_it_is, why) in [
            (
                true,
                "",
                landscape_search::candidates::NoVocabulary::NothingQuotable,
            ),
            (
                false,
                "we were unable to read its front page",
                landscape_search::candidates::NoVocabulary::Unreadable,
            ),
        ] {
            assert_eq!(
                landscape_search::competitors::NoRivals::when_no_engine_is_configured(&seed(
                    read, what_it_is
                )),
                landscape_search::competitors::NoRivals::NothingToCompare(why),
                "read={read}"
            );
        }
    }

    #[test]
    fn the_report_and_the_diagnostic_agree_about_whether_an_engine_would_help() {
        // **The parity that was broken.** Two surfaces describing one run: the report's reason
        // for having nobody, and the footer this command prints when nothing is configured.
        // They read the same `seed.words()` now, and the assertion is that they cannot come to
        // different conclusions about whether configuring something would change the outcome.
        for (read, what_it_is) in [
            (true, "Project management and team communication"),
            (true, ""),
            (false, "we were unable to read its front page"),
        ] {
            let seed = seed(read, what_it_is);
            let reported =
                landscape_search::competitors::NoRivals::when_no_engine_is_configured(&seed)
                    .sentence();
            let diagnosed = without_an_engine(&seed);

            let report_blames_the_engine = reported.contains("no search engine is configured");
            let diagnostic_says_it_would_help =
                !diagnosed.contains("nothing would have been asked with it either");
            assert_eq!(
                report_blames_the_engine, diagnostic_says_it_would_help,
                "read={read}, what_it_is={what_it_is:?}\n  report: {reported}\n  cli:    {diagnosed}"
            );
        }
    }

    #[tokio::test]
    async fn a_description_with_no_engine_is_the_one_situation_naming_a_domain_fixes() {
        // **The worker's own choice of situation**, which `decide` never sees: with nothing
        // configured there is no verdict to decide from, and this is the only refusal where
        // *"try naming its website"* is the right instruction rather than a habit.
        let decided = resolve_from_description(None, "a shared inbox for a small team").await;
        let landscape_analyze::subject::Decided::Refuse(refusal) = decided else {
            panic!("a run with no engine analysed something")
        };
        assert_eq!(refusal.kind, landscape_core::Failure::NoSubject);
        let why = &refusal.why;
        assert!(why.contains("not configured here"), "{why}");
    }

    #[tokio::test]
    async fn a_run_records_the_situation_its_refusal_chose_rather_than_a_default() {
        // **The wire between the two tests below**, and the only one either misses: `decide`
        // picks the situation, `refuse` records what it is given, and the worker passes one to
        // the other. A version that hardcoded `NoSubject` there passed both.
        //
        // Distinguishing it needs a refusal that is *not* `NoSubject`, so this configures an
        // engine at a port nothing is listening on: every query fails fast, and a search that
        // did not finish is the one situation a reader fixes by waiting.
        //
        // `SEARX_URL` is process-global; nextest runs each test in its own process.
        std::env::set_var("SEARX_URL", "http://127.0.0.1:1");

        let store: Arc<dyn Store> = Arc::new(MemoryStore::new());
        let queued = store
            .enqueue(
                &landscape_core::NewAnalysis::parse("a shared inbox for a small support team")
                    .expect("valid"),
            )
            .await
            .expect("a row");
        let claimed = store.claim_next().await.expect("a claim").expect("a row");

        run_analysis(&store, &claimed).await;

        let read = store.get(queued.id).await.expect("it reads back");
        assert_eq!(read.status, landscape_core::AnalysisStatus::Failed);
        assert_eq!(
            read.failure,
            Some(landscape_core::Failure::SearchIncomplete),
            "the situation the run concluded was replaced with a default on the way out"
        );
    }

    #[tokio::test]
    async fn a_candidate_rejected_after_an_outage_reaches_the_client_as_retryable() {
        // The whole chain for the case review found: `decide` concludes the searching did not
        // finish, `refuse` records it, and the client reads back the one situation it fixes by
        // waiting rather than *"we searched and found nobody"*.
        let decided = landscape_analyze::subject::decide(
            landscape_search::competitors::Derived {
                verdict: landscape_core::subject::Resolution::Resolved {
                    entity: landscape_core::subject::Candidate {
                        name: "Alpha".to_owned(),
                        canonical_domain: "alpha.example".to_owned(),
                        what_it_is: "a company".to_owned(),
                        confidence: 0.9,
                    },
                },
                set: landscape_search::competitors::Set::default(),
                about_a_market: true,
            },
            &landscape_search::candidates::Queried {
                completed: vec!["q1".to_owned(), "q2".to_owned()],
                failed: vec!["q3".to_owned()],
            },
        );
        let landscape_analyze::subject::Decided::Refuse(refusal) = decided else {
            panic!("an empty set was analysed")
        };

        let store: Arc<dyn Store> = Arc::new(MemoryStore::new());
        let queued = store
            .enqueue(
                &landscape_core::NewAnalysis::parse("a shared inbox for a small support team")
                    .expect("valid"),
            )
            .await
            .expect("a row");
        let claimed = store.claim_next().await.expect("a claim").expect("a row");
        refuse(&store, &claimed, &refusal).await;

        let read = store.get(queued.id).await.expect("it reads back");
        assert_eq!(
            read.failure,
            Some(landscape_core::Failure::SearchIncomplete),
            "the client is told a market is empty while a query was unanswered"
        );
    }

    #[tokio::test]
    async fn the_question_reaches_the_client_and_not_only_the_situation() {
        // The whole chain for a chip: `decide` refuses to choose between two companies,
        // `refuse` records both, and the client reads back the domains it can offer as
        // buttons. **`kind` travelling without `choices` is the defect this pins** - a client
        // told a name matched several companies, and not told which, has to ask the reader to
        // work out what we would not.
        let decided = landscape_analyze::subject::decide(
            landscape_search::competitors::Derived {
                verdict: landscape_core::subject::Resolution::Ambiguous {
                    question: "which one?".to_owned(),
                    candidates: vec![
                        landscape_core::subject::Candidate {
                            name: "Notion".to_owned(),
                            canonical_domain: "notion.so".to_owned(),
                            what_it_is: "one workspace for notes and docs".to_owned(),
                            confidence: 0.9,
                        },
                        landscape_core::subject::Candidate {
                            name: "Notion Energy".to_owned(),
                            canonical_domain: "notionenergy.com".to_owned(),
                            what_it_is: "battery storage for commercial sites".to_owned(),
                            confidence: 0.88,
                        },
                    ],
                },
                set: landscape_search::competitors::Set::default(),
                about_a_market: false,
            },
            &landscape_search::candidates::Queried {
                completed: vec!["q1".to_owned(), "q2".to_owned(), "q3".to_owned()],
                failed: Vec::new(),
            },
        );
        let landscape_analyze::subject::Decided::Refuse(refusal) = decided else {
            panic!("a tie about one company is a refusal")
        };

        let store: Arc<dyn Store> = Arc::new(MemoryStore::new());
        let queued = store
            .enqueue(
                &landscape_core::NewAnalysis::parse("notion, and whoever competes with it")
                    .expect("valid"),
            )
            .await
            .expect("a row");
        let claimed = store.claim_next().await.expect("a claim").expect("a row");
        refuse(&store, &claimed, &refusal).await;

        let read = store.get(queued.id).await.expect("it reads back");
        assert_eq!(read.failure, Some(landscape_core::Failure::Ambiguous));
        assert_eq!(
            read.choices
                .iter()
                .map(|c| c.prompt.as_str())
                .collect::<Vec<_>>(),
            ["notion.so", "notionenergy.com"],
            "the client must be able to send one of these back without rewriting it"
        );
    }

    #[tokio::test]
    async fn the_situation_a_refusal_chose_is_the_one_the_store_records() {
        // **The last place the five could collapse back into one.** `refuse` used to pass
        // `Failure::NoSubject` whatever `decide` had concluded, which is how every ending came
        // to read the same to a reader in the first place - so the hand-off is asserted rather
        // than assumed.
        for kind in [
            landscape_core::Failure::NoSubject,
            landscape_core::Failure::Ambiguous,
            landscape_core::Failure::NothingFound,
            landscape_core::Failure::SearchIncomplete,
        ] {
            let store: Arc<dyn Store> = Arc::new(MemoryStore::new());
            let queued = store
                .enqueue(
                    &landscape_core::NewAnalysis::parse("an idea worth checking").expect("valid"),
                )
                .await
                .expect("a row");
            let claimed = store.claim_next().await.expect("a claim").expect("a row");

            refuse(
                &store,
                &claimed,
                &landscape_analyze::subject::Refusal {
                    why: "why, for an operator".to_owned(),
                    kind,
                    choices: Vec::new(),
                },
            )
            .await;

            let read = store.get(queued.id).await.expect("it reads back");
            assert_eq!(read.status, landscape_core::AnalysisStatus::Failed);
            assert_eq!(
                read.failure,
                Some(kind),
                "the situation was replaced on the way"
            );
        }
    }

    #[test]
    fn the_diagnostic_shows_the_reason_a_report_covers_one_company() {
        // A diagnostic that showed the comparison but not the reason for its size would show
        // less than the report does - which is the drift the last two rounds were about,
        // pointed at the other half of the same output.
        let lone = |alone| landscape_search::competitors::Set {
            members: vec![landscape_search::competitors::Member {
                candidate: landscape_core::subject::Candidate {
                    name: "Basecamp".to_owned(),
                    canonical_domain: "basecamp.com".to_owned(),
                    what_it_is: "Project management".to_owned(),
                    confidence: 1.0,
                },
                because: landscape_search::competitors::Because::Named,
            }],
            set_aside: Vec::new(),
            alone,
        };

        for why in [
            landscape_search::competitors::NoRivals::NoEngine,
            landscape_search::competitors::NoRivals::NobodyHeldUp {
                sought: landscape_search::competitors::Sought::RivalsOfTheCompany,
            },
            landscape_search::competitors::NoRivals::SearchIncomplete {
                failed: 2,
                sent: 3,
                sought: landscape_search::competitors::Sought::RivalsOfTheCompany,
            },
        ] {
            let said = set_as_printed(&lone(Some(why.clone()))).join("\n");
            assert!(
                said.contains(&why.sentence()),
                "the report would say why and this does not: {said}"
            );
        }

        // And a comparison carries no explanation for an absence that is not there.
        let said = set_as_printed(&lone(None)).join("\n");
        assert!(!said.contains("We did not look"), "{said}");
        assert!(!said.contains("none of what came back held up"), "{said}");
    }

    #[test]
    fn the_no_engine_footer_never_points_at_queries_that_are_not_there() {
        // **Review found the footer contradicting the section above it.** With no vocabulary
        // `what_would_be_asked` correctly says nothing will be asked, and then the footer said
        // *"the queries above are what would go"* - pointing at nothing, and implying that
        // setting `SEARX_URL` would change the outcome. It would not.
        for (read, what_it_is) in [(true, ""), (false, "we were unable to read its front page")] {
            let seed = seed(read, what_it_is);
            let said = without_an_engine(&seed);
            assert!(
                !said.contains("queries above"),
                "pointed at queries that were never printed: {said}"
            );
            assert!(
                said.contains("nothing would have been asked with it either"),
                "a reader is sent to install an engine that would change nothing: {said}"
            );
        }

        // And the other half, so the rule is a rule: with real vocabulary the queries above are
        // exactly what an engine would be sent.
        let said = without_an_engine(&seed(true, "Project management and team communication"));
        assert!(
            said.contains("The queries above are what would go"),
            "{said}"
        );
    }

    #[test]
    fn every_line_this_command_prints_agrees_that_nothing_will_be_asked() {
        // The section and the footer are two outputs of one decision, and this is the assertion
        // that they cannot disagree - whatever `words()` says, both have to follow it.
        for (read, what_it_is) in [
            (true, "Project management and team communication"),
            (true, ""),
            (false, "we were unable to read its front page"),
        ] {
            let seed = seed(read, what_it_is);
            let nothing_asked = seed.words().is_err();
            let whole = format!(
                "{}\n{}",
                what_would_be_asked(&seed).join("\n"),
                without_an_engine(&seed)
            );
            assert_eq!(
                nothing_asked,
                !whole.contains("Basecamp alternatives"),
                "{whole}"
            );
            assert_eq!(nothing_asked, !whole.contains("queries above"), "{whole}");
        }
    }

    #[test]
    fn a_page_with_no_prose_is_told_apart_from_one_nobody_could_read() {
        // Two ways to have no vocabulary, and only one of them is worth trying again.
        let unquotable = what_would_be_asked(&seed(true, "")).join("\n");
        assert!(
            unquotable.contains("no sentence describing what it does"),
            "{unquotable}"
        );

        let unread =
            what_would_be_asked(&seed(false, "we were unable to read its front page")).join("\n");
        assert!(unread.contains("could not read its front page"), "{unread}");
        assert_ne!(unquotable, unread, "two different silences, one sentence");
    }

    #[test]
    fn a_readable_seed_shows_the_words_its_rivals_are_matched_against() {
        let said = what_would_be_asked(&seed(true, "Project management and team communication"))
            .join("\n");
        assert!(said.contains("matching  project, management"), "{said}");
        assert!(said.contains("Basecamp alternatives"), "{said}");
        assert!(
            said.contains("Nothing you typed reaches the engine"),
            "{said}"
        );
    }

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
