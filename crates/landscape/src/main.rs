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
use landscape_api::{router, AppState};
use landscape_core::{Report, Section};
use landscape_db::{MemoryStore, PgStore, Store};

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
        Some("read") => Role::Read,
        Some(other) => anyhow::bail!(
            "unknown command {other:?}.              Try: dev, serve, worker, migrate, fetch, gap, discover, read"
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
        "usage: landscape fetch <url>

Example:
  landscape fetch https://example.com/",
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

/// `landscape read <origin>` — discovery, fetch, Markdown, and extraction in one run.
///
/// Every stage prints what it decided, because the point of this command is watching the
/// joins rather than seeing an answer. A pipeline that only reports its last step is one
/// where a wrong answer has six possible causes and no way to tell them apart.
async fn read_company(args: &[String]) -> Result<()> {
    let origin = args.get(1).filter(|a| !a.starts_with("--")).context(
        "usage: landscape read <origin>

Example:
  landscape read https://basecamp.com",
    )?;

    let fetcher = landscape_fetch::Fetcher::new();
    let found = landscape_discover::discover(&fetcher, origin).await;
    println!("{}", found.render());

    if found.sources.is_empty() {
        return Ok(());
    }

    // Only built if a model answers. Everything above this line is deterministic and worth
    // seeing on its own, which is why the check happens here rather than at the top.
    let llm = landscape_llm::LlamaClient::from_env();
    let model_ready = llm.is_ready().await;
    if !model_ready {
        println!(
            "No llama-server at {} - stopping after conversion.
             Start one and re-run to see extraction:
               llama-server -hf Qwen/Qwen3-4B-GGUF:Q4_K_M --host 127.0.0.1 --port 8080",
            llm.base()
        );
    }

    println!("{:<44} {:<6} {:<7} extracted", "page", "words", "quality");
    println!("{}", "-".repeat(96));

    for source in &found.sources {
        let Ok(page) = fetcher.get(&source.url).await else {
            println!(
                "{:<44} {:<6} {:<7} could not fetch",
                short(&source.url),
                "-",
                "-"
            );
            continue;
        };
        let markdown = landscape_extract::markdown::from_html(&page.body);
        let assessment = landscape_extract::quality::assess(&markdown);

        if !assessment.quality.worth_extracting() {
            // Not an error. The page was read and there was nothing on it, which is what a
            // report says rather than something it retries.
            println!(
                "{:<44} {:<6} {:<7} skipped - nothing to read",
                short(&source.url),
                assessment.words,
                assessment.quality.name()
            );
            continue;
        }
        if !model_ready {
            println!(
                "{:<44} {:<6} {:<7} (no model)",
                short(&source.url),
                assessment.words,
                assessment.quality.name()
            );
            continue;
        }

        let prompt = extraction_prompt(&source.url, &markdown);
        let decode = landscape_llm::Decode {
            max_tokens: 300,
            temperature: 0.0,
            seed: Some(7),
        };
        let extracted: Result<landscape_core::PricingExtraction, _> =
            llm.generate(&prompt, &decode).await;

        let summary = match extracted {
            Ok(e) => match (e.plan_name.as_deref(), e.price_usd) {
                (Some(plan), Some(price)) => format!("{plan} at ${price}"),
                (Some(plan), None) => format!("{plan}, no price published"),
                (None, Some(price)) => format!("${price}, no plan named"),
                (None, None) => "nothing found".to_owned(),
            },
            Err(e) => format!("model error: {e}"),
        };
        println!(
            "{:<44} {:<6} {:<7} {summary}",
            short(&source.url),
            assessment.words,
            assessment.quality.name()
        );
    }
    Ok(())
}

/// The extraction prompt, kept beside the only thing that sends it.
///
/// **This must name a plan, and the first draft did not.** The golden set's prompt opens
/// with `Plan to extract: <name>`; this one said only "a single plan", and against
/// basecamp.com — a page showing `$299/month` and `$15/user` plainly — the model answered
/// *"no price published"*. The price was in the Markdown and inside the prompt; the model
/// simply had no way to know which of two plans was being asked about, and abstaining was
/// the honest thing for it to do.
///
/// That is a real limitation and not only a prompt bug: [`PricingExtraction`] models **one**
/// plan, and a pricing page has several. Naming the first plan the page presents makes the
/// question answerable; extracting *all* the plans needs a different type and is the next
/// piece of work. `ROADMAP.md` records it.
///
/// [`PricingExtraction`]: landscape_core::PricingExtraction
fn extraction_prompt(url: &str, markdown: &str) -> String {
    let page: String = markdown.chars().take(6000).collect();
    format!(
        "You are reading one page from a company's website and extracting what it says          about the pricing of one plan.

         Plan to extract: the first plan this page presents. If the page names several,          take the one appearing earliest.

         Page: {url}

         Rules:
         - Use only what this page states. Do not use anything you know from elsewhere.
         - If the page does not state something, leave that field null. A missing price is          a fact worth reporting; a guessed one is not.
         - Report the price of that one plan only. A price on this page for a different          plan, a different product, or an add-on is not this plan's price.
         - If price_usd is null then billing_period must be null too.
         - price_usd is in US dollars. Ignore prices given in other currencies.
         - evidence_quote must be copied from the page word for word.

         PAGE
---
{page}
---

Return the extraction as JSON."
    )
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
        Role::Fetch | Role::Gap | Role::Discover | Role::Read => Ok(()),
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
    let app = router(AppState { store })
        // The dev frontend runs on a different port, so the browser treats it as another
        // origin. Permissive is fine while everything is served from localhost; this
        // tightens to an allow-list before anything is deployed.
        .layer(tower_http::cors::CorsLayer::permissive());

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
                        tracing::info!(id = %analysis.id, "running analysis");
                        let report = placeholder_report(&analysis.prompt);
                        if let Err(e) = store.complete(analysis.id, &report).await {
                            tracing::error!(id = %analysis.id, error = %e, "could not save report");
                        }
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
fn placeholder_report(prompt: &str) -> Report {
    Report {
        subject: prompt.to_owned(),
        searched_as: String::new(),
        generated_at: chrono::Utc::now(),
        model_id: "none".to_owned(),
        prompt_version: 0,
        sections: vec![
            Section::not_found(
                "pricing",
                "Prices",
                vec!["nothing was fetched: the gathering pipeline is not built yet".to_owned()],
            ),
            Section::not_found(
                "features",
                "What each one does",
                vec!["nothing was fetched: the gathering pipeline is not built yet".to_owned()],
            ),
        ],
        sources: Vec::new(),
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
    use landscape_core::SectionStatus;

    #[test]
    fn the_placeholder_report_claims_nothing_it_cannot_show() {
        let r = placeholder_report("an app for farm-to-restaurant orders");
        assert!(
            r.sources.is_empty(),
            "no sources were read, so none are listed"
        );
        for section in &r.sections {
            assert_eq!(
                section.status,
                SectionStatus::NotFoundInPublicSources,
                "a section with no evidence must say so"
            );
            assert!(section.claims.is_empty());
            assert!(
                !section.checked.is_empty(),
                "even a placeholder negative shows its working"
            );
        }
        assert!(
            r.every_claim_is_traceable(),
            "a report with no claims is trivially traceable"
        );
    }

    #[tokio::test]
    async fn a_queued_analysis_is_picked_up_and_completed() {
        // The whole path, with no database: enqueue, claim, complete, read back.
        let store = MemoryStore::new();
        let new = landscape_core::NewAnalysis::parse("an app for farm-to-restaurant orders")
            .expect("valid prompt");
        let queued = store.enqueue(&new).await.expect("enqueue");

        let claimed = store
            .claim_next()
            .await
            .expect("claim")
            .expect("one queued");
        assert_eq!(claimed.id, queued.id);

        let report = placeholder_report(&claimed.prompt);
        store.complete(claimed.id, &report).await.expect("complete");

        let done = store.get(queued.id).await.expect("get");
        assert_eq!(done.status, landscape_core::AnalysisStatus::Complete);
        assert!(done.report.is_some());
    }
}
