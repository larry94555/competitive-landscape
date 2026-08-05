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
    let analysis = landscape_analyze::analyse(&fetcher, &llm, origin, now, now.date_naive()).await;

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
    let Some(origin) = landscape_analyze::subject::origin_in(&analysis.prompt) else {
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

    tracing::info!(id = %analysis.id, %origin, "running analysis");
    let fetcher = landscape_fetch::Fetcher::new();
    let llm = landscape_llm::LlamaClient::from_env();
    let now = chrono::Utc::now();

    let progress = progress::Progress::new(Arc::clone(store), analysis.id, analysis.generation);
    let outcome = landscape_analyze::analyse_with(
        &fetcher,
        &llm,
        &origin,
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
