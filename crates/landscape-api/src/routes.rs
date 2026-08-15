//! Routes.
//!
//! Small on purpose. Everything that is not HTTP belongs in `landscape-core` (rules) or
//! `landscape-db` (storage); this layer parses, delegates and maps errors.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse as _;
use axum::routing::{any, get, post};
use axum::{Json, Router};
use landscape_core::{Analysis, AnalysisId, AnalysisStatus, NewAnalysis};
use landscape_db::Store;
use serde::{Deserialize, Serialize};

use crate::error::ApiError;
use crate::extract::Json as ValidJson;

#[derive(Clone)]
pub struct AppState {
    pub store: Arc<dyn Store>,
    /// How many runs one anonymous address may start in a day.
    ///
    /// In the state rather than a layer because it has to be *counted where the run starts*
    /// and nowhere else: a reader watching a report they already began must never be cut off
    /// by a cap, and a middleware over the whole router would do exactly that.
    pub cap: Arc<crate::cap::Cap>,
    /// Whether a search engine is configured, and so whether an idea can be researched at all.
    ///
    /// **Read once, at startup, and served with the examples.** Without an engine a description
    /// cannot be resolved: the run refuses, and the reader has spent one of their analyses to
    /// be told about an environment variable. A reader found this the hard way — the honest
    /// thing is to say it on the first screen, beside the ideas that will not work.
    ///
    /// The same call the worker makes, so the two cannot disagree about what is configured.
    pub discovery: bool,
}

impl AppState {
    /// The ordinary way to build one: a store, the cap the environment asks for, and whether
    /// an engine answered the binary when it looked.
    ///
    /// **`discovery` is passed in rather than read here.** What counts as a configured engine
    /// is `landscape_search::Searx::configured` — it trims, and rejects an empty string — and a
    /// second copy of that rule in this crate is a second thing to keep in step. The binary
    /// already holds the answer because it is the thing that runs the searches.
    #[must_use]
    pub fn new(store: Arc<dyn Store>, discovery: bool) -> Self {
        Self {
            store,
            cap: Arc::new(crate::cap::Cap::from_env()),
            discovery,
        }
    }
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState").finish_non_exhaustive()
    }
}

/// Every route the server serves.
///
/// Both observability layers are applied here rather than in the binary. Wiring them where
/// the server is assembled would leave every test in this crate running without them, so
/// the tests would pass while asserting the behavior of a slightly different application
/// than the one that ships — and the first thing to break would be the error path, which
/// is the part hardest to notice.
///
/// The layer writes the access line itself rather than delegating to `tower_http`'s
/// `TraceLayer`; [`crate::request_id::layer`] records why.
pub fn router(state: AppState) -> Router {
    with_request_ids(routes(state))
}

/// Everything under `/api`, with no middleware on it yet.
///
/// Split out so [`with_ui`] can add the single-page fallback **before** the request-id layer
/// goes on. Applied the other way round the layer wraps only the routes that existed when it
/// was attached, and the page a visitor actually asks for is the one response with no id, no
/// span and no access line — see [`with_request_ids`].
fn routes(state: AppState) -> Router {
    Router::new()
        .route("/api/health", get(health))
        // The ideas the first screen offers. `GET` and no state: it is a constant, and it is
        // served rather than compiled into the page so that the list, the command that checks
        // it and the test that parses it are one list.
        .route("/api/examples", get(list_examples))
        .route("/api/analyses", post(create_analysis))
        .route("/api/analyses/{id}", get(get_analysis))
        // The same report as one Markdown file, for a reader to paste into whatever
        // assistant they already pay for — `IDEA_ANALYSIS.md` §5. Served rather than built
        // in the browser so the page, this, and `landscape context` cannot come to different
        // views of what a source's standing is called.
        .route("/api/analyses/{id}/context", get(get_context))
        // Server-sent events for one analysis. A reader watching a report fill in
        // is the difference between ninety seconds of spinner and twenty of
        // content — PRODUCT_SPEC.md §2.1A.
        .route("/api/analyses/{id}/events", get(crate::events::stream))
        // `/api` is claimed whole, including the parts of it that do not exist. Without this
        // the single-page fallback in `with_ui` answers a mistyped endpoint with `index.html`,
        // and the client reports a JSON parse error instead of the wrong URL.
        //
        // **All three, and review found out why.** `/api/{*rest}` matches paths *below*
        // `/api/`, and axum treats `/api`, `/api/` and `/api/x` as three different paths — so
        // the first two fell straight through to the fallback and came back as the page.
        .route("/api", any(missing))
        .route("/api/", any(missing))
        .route("/api/{*rest}", any(missing))
        .with_state(state)
}

/// The request id, the span and the access line — ADR 0005's invariant.
///
/// Applied **last**, over whatever the router has ended up containing. `Router::layer` wraps
/// the routes present when it is called and nothing added afterwards, which is exactly how the
/// static fallback came to be the one surface with no id on it.
fn with_request_ids(router: Router) -> Router {
    router.layer(axum::middleware::from_fn(crate::request_id::layer))
}

/// Anything under `/api` we do not serve. JSON, not a page.
async fn missing() -> ApiError {
    ApiError::NotFound
}

/// Serve the built single-page app alongside the API.
///
/// # Why this exists at all
///
/// Until now [`router`] served `/api/*` and nothing else, and the React app existed only
/// behind Vite's dev server. **That made the whole thing undeployable without anybody
/// noticing**: put the binary on a host and a visitor gets JSON, because there is no page to
/// ask for. `PROJECT_STATUS.md` recorded a guided demo as blocked on deployment alone, which
/// was wrong in exactly this way — the gap was not the box, it was that nothing served a page
/// to put on it.
///
/// # The fallback is the point
///
/// Any path the API does not claim returns `index.html`, because a single-page app owns its
/// own routing: `/a/<id>` has to reach the client for a permalink to survive a refresh, and a
/// 404 from the server would be the one thing that stops it. Real files still win — the
/// fallback only runs when `ServeDir` finds nothing.
///
/// Returns the API unchanged when `dir` holds no `index.html`, so `cargo run` with Vite on the
/// side keeps working exactly as `Feature_Walkthrough.md` describes. A missing build is a
/// developer running the pieces separately, not an error.
pub fn with_ui(state: AppState, dir: &std::path::Path) -> Router {
    let api = routes(state);
    if !dir.join("index.html").is_file() {
        tracing::info!(
            dir = %dir.display(),
            "no built web app here; serving the API alone (run `npm run build` in web/)"
        );
        return with_request_ids(api);
    }
    tracing::info!(dir = %dir.display(), "serving the web app");
    let files = tower_http::services::ServeDir::new(dir)
        .fallback(tower_http::services::ServeFile::new(dir.join("index.html")));
    // The layer goes on after the fallback, so a page request carries an id like any other.
    with_request_ids(api.fallback_service(files))
}

/// Where the built web app is expected, unless `WEB_DIR` says otherwise.
///
/// Relative to the working directory, which on the host is wherever the unit file puts it.
#[must_use]
pub fn web_dir() -> std::path::PathBuf {
    std::env::var("WEB_DIR").map_or_else(|_| std::path::PathBuf::from("web/dist"), Into::into)
}

#[derive(Debug, Serialize)]
struct Health {
    status: &'static str,
    /// Proves the process can reach storage. A health check that only proves the process
    /// is running will report healthy while every request fails.
    queued: i64,
    version: &'static str,
}

async fn health(State(state): State<AppState>) -> Result<Json<Health>, ApiError> {
    let queued = state
        .store
        .count_with_status(AnalysisStatus::Queued)
        .await?;
    Ok(Json(Health {
        status: "ok",
        queued,
        version: env!("CARGO_PKG_VERSION"),
    }))
}

/// `GET /api/examples` — the curated ideas, and what the interface must say about them.
///
/// **The note travels with the list.** An interface free to render the chips without it would
/// be free to imply the reports are curated too, which is the one misreading this feature can
/// cause — and the sentence would then live in a component nobody reviews.
async fn list_examples(State(state): State<AppState>) -> Json<Examples> {
    Json(Examples {
        discovery: state.discovery,
        note: landscape_core::CURATION_NOTE,
        examples: landscape_core::examples()
            .into_iter()
            .map(|example| Listed {
                prompt: example.prompt(),
                id: example.id,
                idea: example.idea,
            })
            .collect(),
    })
}

#[derive(Debug, Serialize)]
struct Examples {
    /// Whether these will run at all.
    ///
    /// **Beside the examples because that is where somebody is about to click.** A capability
    /// the product does not currently have is not a detail for a log — every one of these
    /// ideas is a description, and without an engine every one of them refuses.
    discovery: bool,
    /// What is curated and what is not, in one sentence.
    note: &'static str,
    examples: Vec<Listed>,
}

/// One example on the wire.
///
/// **Three fields, not the whole `Example`.** It used to `#[serde(flatten)]` the core type, so
/// the browser received `companies` and `why` — the curation fixture and the note explaining a
/// pairing — neither of which the page shows now that a prompt is the idea alone. Sending data
/// nobody renders is how a field ends up displayed by accident two changes later.
#[derive(Debug, Serialize)]
struct Listed {
    /// Stable across wording changes, and what a click is logged as.
    id: String,
    /// What the chip reads.
    idea: String,
    /// The text the chip puts in the box.
    ///
    /// Sent rather than assembled in the browser: what an example *is* has changed twice, and
    /// both times every caller had to agree about it. One decision, one place.
    prompt: String,
}

#[derive(Debug, Deserialize)]
struct CreateAnalysis {
    prompt: String,
}

async fn create_analysis(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    ValidJson(body): ValidJson<CreateAnalysis>,
) -> Result<(StatusCode, Json<Analysis>), ApiError> {
    // **The prompt is checked first, and nothing is reserved.** `PRODUCT_SPEC.md` §2.1 and
    // `UI_FLOWS.md` §2.2 both say a failed analysis costs nothing; the first version of this
    // counted before parsing, so a typo spent half of somebody's day. Review found it.
    let new = NewAnalysis::parse(&body.prompt)?;

    // Counted here and nowhere else. This is the only request that starts minutes of work on
    // the one model this machine has; reading a report, watching one arrive, or listing the
    // examples cost nothing worth rationing, and capping them would cut off a reader in the
    // middle of something they already started.
    let client =
        crate::cap::client_in(headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()));
    if state.cap.applies_to(client.as_deref()) {
        let client = client.unwrap_or_default();

        // **The day can turn while this request waits for its turn.** Acquiring the gate is a
        // wait, so the clock is read again on each attempt and the decision is abandoned if
        // the day moved underneath it.
        //
        // Without this, a queue that formed before midnight woke afterwards holding the old
        // day's gate: every one of those requests read an empty list, had its writes dropped,
        // and was admitted free. Review found it - and found that the "at most one crossing
        // request" I had claimed was not a bound at all, because the queue can be any length.
        //
        // Bounded rather than `loop`: the day only advances, so one retry is the real case,
        // and a clock behaving strangely must not spin here for ever.
        let mut attempt = 0;
        let (now, today, _deciding) = loop {
            let now = state.cap.now();
            let today = now.date_naive();
            // Owned, so the guard can outlive this iteration's binding of the gate itself.
            let deciding = state.cap.gate_for(&client, today).lock_owned().await;
            if state.cap.is_current(today) {
                break (now, today, deciding);
            }
            attempt += 1;
            if attempt >= 3 {
                // Three midnights during one request is not a clock this can reason about.
                // Refusing would turn a strange clock into an outage; the guard rail fails
                // open here as it does everywhere else in this type.
                break (now, today, deciding);
            }
        };

        let started = state.cap.started_today(&client, today);

        // **How many still count is asked of the store, not remembered.** A run the worker
        // later marked failed is not charged for - and the worker is a different process, so
        // there is nowhere a refund could have been sent even if one had been reserved.
        let mut still: Vec<landscape_core::AnalysisId> = Vec::with_capacity(started.len());
        for id in started {
            match state.store.get(id).await {
                Ok(analysis) if analysis.status == AnalysisStatus::Failed => {}
                // A row that cannot be read is counted rather than forgiven: the alternative
                // is that a store hiccup hands out an unlimited allowance.
                _ => still.push(id),
            }
        }
        let charged = still.len();
        // Written back, so a failed id is asked about once rather than on every later request.
        state.cap.keep(&client, today, still);

        let limit = state.cap.limit();
        if charged >= limit {
            return Err(ApiError::TooManyToday {
                used: charged,
                limit,
                resets: crate::cap::resets_after(now),
            });
        }

        let analysis = state.store.enqueue(&new).await?;
        // After the store accepted it, so an enqueue that fails costs nothing either.
        state.cap.record(&client, today, analysis.id);
        return Ok((StatusCode::CREATED, Json(analysis)));
    }

    let analysis = state.store.enqueue(&new).await?;
    Ok((StatusCode::CREATED, Json(analysis)))
}

async fn get_analysis(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Analysis>, ApiError> {
    // An unparseable id is "not found" rather than "bad request": from the reader's side
    // a mistyped reference and a deleted one are the same situation, and the distinction
    // would only tell a prober what our ids look like.
    let id = id
        .parse::<uuid::Uuid>()
        .map(AnalysisId)
        .map_err(|_| ApiError::NotFound)?;
    Ok(Json(state.store.get(id).await?))
}

/// The finished report as Markdown, or a reason it is not available yet.
///
/// **`text/markdown` rather than JSON**, because the whole point is that the bytes are
/// pasteable: a reader who does not use our button can `curl` this, and what comes back is
/// the file rather than a file inside a quoted string.
///
/// **Only a `Complete` analysis, and "has a report" was the wrong test for that.**
/// `save_progress` deliberately stores a report while the status is still `Running` — that is
/// how a reader watches one fill in — so checking for `Some(report)` handed out half a
/// document with a `200`. Half a report is answered from as confidently as a whole one, and
/// an assistant reading it has no way to tell. A `Failed` run is refused for the same reason:
/// what it has is where the pipeline stopped, not a report.
async fn get_context(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<axum::response::Response, ApiError> {
    // Same reading as `get_analysis`: a mistyped reference and a deleted one are one
    // situation from the reader's side.
    let id = id
        .parse::<uuid::Uuid>()
        .map(AnalysisId)
        .map_err(|_| ApiError::NotFound)?;
    let analysis = state.store.get(id).await?;
    if analysis.status != AnalysisStatus::Complete {
        return Err(ApiError::NotFound);
    }
    let report = analysis.report.as_ref().ok_or(ApiError::NotFound)?;

    let markdown = landscape_core::context::of(report, Some(&format!("/a/{}", id.0)));
    Ok((
        [(
            axum::http::header::CONTENT_TYPE,
            "text/markdown; charset=utf-8",
        )],
        markdown,
    )
        .into_response())
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
// Panicking IS how a test reports failure. The lints stay denied everywhere else.
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use axum::response::IntoResponse;
    use landscape_db::MemoryStore;
    use tower::ServiceExt;

    fn app() -> Router {
        router(AppState::new(Arc::new(MemoryStore::new()), false))
    }

    async fn json_body(res: axum::response::Response) -> serde_json::Value {
        let bytes = axum::body::to_bytes(res.into_body(), 1 << 20)
            .await
            .expect("read body");
        serde_json::from_slice(&bytes).expect("body is json")
    }

    fn post_analysis(prompt: &str) -> Request<Body> {
        Request::builder()
            .method("POST")
            .uri("/api/analyses")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({ "prompt": prompt }).to_string(),
            ))
            .expect("build request")
    }

    /// A prompt the API accepts, so a test about the cap is not also a test about parsing.
    const IDEA: &str = "an app that helps small farms sell to restaurants";

    async fn text_body(res: axum::response::Response) -> String {
        let bytes = axum::body::to_bytes(res.into_body(), 1 << 20)
            .await
            .expect("read body");
        String::from_utf8(bytes.to_vec()).expect("body is utf-8")
    }

    /// An analysis with a finished report in it, so the context endpoint has something to
    /// render. The report is deliberately minimal: what this endpoint owes is the *bytes*
    /// `landscape_core::context` produced, and that module's own tests cover the rendering.
    async fn a_finished_analysis(store: &Arc<dyn Store>) -> AnalysisId {
        let analysis = store
            .enqueue(&NewAnalysis::parse(IDEA).expect("a valid prompt"))
            .await
            .expect("enqueued");
        let report = landscape_core::Report {
            chosen: None,
            progress: None,
            asked: None,
            searches: None,
            subject: "basecamp.com".to_owned(),
            searched_as: "basecamp.com".to_owned(),
            generated_at: chrono::Utc::now(),
            model_id: "test".to_owned(),
            prompt_version: 1,
            subjects: Vec::new(),
            sections: vec![landscape_core::report::Section {
                key: "pricing".to_owned(),
                title: "Pricing & packaging".to_owned(),
                status: landscape_core::report::SectionStatus::Populated,
                claims: vec![landscape_core::report::Claim {
                    text: "Pro costs $15".to_owned(),
                    subject: "https://basecamp.com".to_owned(),
                    source_label: "S1".to_owned(),
                    evidence_quote: "Pro $15 per user".to_owned(),
                    confidence: landscape_core::report::Confidence::High,
                    as_of: chrono::Utc::now(),
                }],
                checked: Vec::new(),
                notes: Vec::new(),
            }],
            sources: vec![landscape_core::source::Source {
                label: "S1".to_owned(),
                url: "https://basecamp.com/pricing".to_owned(),
                title: "Pricing".to_owned(),
                disposition: landscape_core::source::Disposition::Primary,
                fetched_at: chrono::Utc::now(),
                independence_group: "basecamp.com".to_owned(),
            }],
            interpreted: None,
            notes: Vec::new(),
        };
        // Claimed then completed, which is the path a worker takes - a report only reaches a
        // reader through `complete`, and this endpoint must not find one any other way.
        store.claim_next().await.expect("claimable");
        store
            .complete(analysis.id, 1, &report)
            .await
            .expect("completed");
        analysis.id
    }

    fn get_context_request(id: &str) -> Request<Body> {
        Request::builder()
            .uri(format!("/api/analyses/{id}/context"))
            .body(Body::empty())
            .expect("build request")
    }

    #[tokio::test]
    async fn the_report_comes_back_as_markdown_a_reader_can_paste() {
        // **`text/markdown`, not JSON.** The whole point is that the bytes are pasteable: a
        // reader who does not use the button can `curl` this and get the file rather than a
        // file inside a quoted string.
        let store: Arc<dyn Store> = Arc::new(MemoryStore::new());
        let id = a_finished_analysis(&store).await;
        let res = router(AppState::new(Arc::clone(&store), false))
            .oneshot(get_context_request(&id.0.to_string()))
            .await
            .expect("response");

        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(
            res.headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok()),
            Some("text/markdown; charset=utf-8"),
        );
        let body = text_body(res).await;
        assert!(
            body.starts_with(landscape_core::context::OPENING_LINE),
            "{body}"
        );
        assert!(body.contains("Pro costs $15"), "{body}");
        assert!(body.contains("https://basecamp.com/pricing"), "{body}");
        // The permalink is what the closing note points at when a report is too big to fit,
        // and it is the only place this endpoint knows the id.
        assert!(body.contains(&format!("/a/{}", id.0)), "{body}");
    }

    #[tokio::test]
    async fn an_analysis_with_no_report_yet_has_nothing_to_paste() {
        // Half a report handed to an assistant is answered from as confidently as a whole
        // one, so a run still going is a `404` rather than a short document.
        let store: Arc<dyn Store> = Arc::new(MemoryStore::new());
        let queued = store
            .enqueue(&NewAnalysis::parse(IDEA).expect("a valid prompt"))
            .await
            .expect("enqueued");
        let res = router(AppState::new(Arc::clone(&store), false))
            .oneshot(get_context_request(&queued.id.0.to_string()))
            .await
            .expect("response");
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn a_run_still_going_has_nothing_to_paste_even_though_it_has_a_report() {
        // **`save_progress` stores a report while the status is `Running`** - that is how a
        // reader watches one fill in - so "has a report" was never the same question as
        // "is finished", and this endpoint was answering the wrong one.
        let store: Arc<dyn Store> = Arc::new(MemoryStore::new());
        let a = store
            .enqueue(&NewAnalysis::parse(IDEA).expect("a valid prompt"))
            .await
            .expect("enqueued");
        store.claim_next().await.expect("claimable");
        let half = landscape_core::Report {
            chosen: None,
            asked: None,
            searches: None,
            subject: "basecamp.com".to_owned(),
            searched_as: "basecamp.com".to_owned(),
            generated_at: chrono::Utc::now(),
            model_id: "test".to_owned(),
            prompt_version: 1,
            subjects: Vec::new(),
            sections: Vec::new(),
            sources: Vec::new(),
            interpreted: None,
            notes: Vec::new(),
            progress: None,
        };
        store.save_progress(a.id, 1, &half).await.expect("saved");
        let running = store.get(a.id).await.expect("read back");
        assert_eq!(
            running.status,
            AnalysisStatus::Running,
            "the fixture has to be the case under test"
        );
        assert!(running.report.is_some(), "and it has to carry a report");

        let res = router(AppState::new(Arc::clone(&store), false))
            .oneshot(get_context_request(&a.id.0.to_string()))
            .await
            .expect("response");
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn a_reference_that_is_not_ours_is_not_found_rather_than_explained() {
        // The same reading `get_analysis` takes: telling a prober that their id was the
        // wrong *shape* tells them what our ids look like.
        for reference in ["not-a-uuid", "0195b0d0-0000-7000-8000-000000000000"] {
            let res = app()
                .oneshot(get_context_request(reference))
                .await
                .expect("response");
            assert_eq!(res.status(), StatusCode::NOT_FOUND, "{reference}");
        }
    }

    #[tokio::test]
    async fn every_chip_a_reader_can_click_is_a_prompt_this_endpoint_accepts() {
        // **Review found a chip that was a dead end.** `Choice::prompt` was the bare canonical
        // domain, and this endpoint rejects anything under `MIN_PROMPT` characters. `box.com`
        // is seven: the button rendered, the click posted, and the reader was told their prompt
        // was too short - about a company we had resolved ourselves and offered them.
        //
        // **The prompt under test is the one `choices_from` really produces.** Asserting
        // against a string typed here would pass forever after the two sides drifted apart,
        // which is the failure this test exists to prevent rather than to imitate.
        for domain in ["box.com", "wix.com", "notionenergy.com"] {
            let offered =
                landscape_analyze::subject::choices_from(&[landscape_core::subject::Candidate {
                    name: "Whoever".to_owned(),
                    canonical_domain: domain.to_owned(),
                    what_it_is: "a company".to_owned(),
                    confidence: 0.9,
                }]);

            let res = app()
                .oneshot(post_analysis(&offered[0].prompt))
                .await
                .expect("the request is served");
            assert_eq!(
                res.status(),
                StatusCode::CREATED,
                "clicking the chip for {domain} sent {:?} and the API refused it",
                offered[0].prompt
            );
        }
    }

    /// A router whose cap allows `limit` a day, so the number under test is the number here
    /// rather than whatever the environment happens to hold.
    fn app_capped_at(limit: usize) -> axum::Router {
        capped_with_store(limit).0
    }

    /// The same, keeping the store — for the tests that need to drive a run to a terminal
    /// state the way the worker would.
    fn capped_with_store(limit: usize) -> (axum::Router, Arc<dyn Store>) {
        let (app, store, _) = capped_parts(limit);
        (app, store)
    }

    /// A store that gives the scheduler a chance between its steps.
    ///
    /// **The in-memory store is too fast to expose a race that a real one exposes every time.**
    /// `MemoryStore::get` takes a lock and returns without ever awaiting anything, so twenty
    /// concurrent requests can run to completion one after another and a genuine
    /// check-then-act bug looks fine. Postgres does I/O and always yields.
    ///
    /// A regression that depends on the fast store's timing is a flaky one - it caught the
    /// missing lock on one run and missed it on the next, which is how this was found. This
    /// puts the interleaving point back where a real deployment has it.
    #[derive(Debug)]
    struct YieldsBetweenSteps(Arc<dyn Store>);

    #[async_trait::async_trait]
    impl Store for YieldsBetweenSteps {
        async fn enqueue(&self, new: &NewAnalysis) -> landscape_db::Result<Analysis> {
            tokio::task::yield_now().await;
            self.0.enqueue(new).await
        }
        async fn get(&self, id: AnalysisId) -> landscape_db::Result<Analysis> {
            tokio::task::yield_now().await;
            self.0.get(id).await
        }
        async fn claim_next(&self) -> landscape_db::Result<Option<Analysis>> {
            self.0.claim_next().await
        }
        async fn save_progress(
            &self,
            id: AnalysisId,
            generation: u32,
            report: &landscape_core::Report,
        ) -> landscape_db::Result<landscape_core::Applied> {
            self.0.save_progress(id, generation, report).await
        }
        async fn complete(
            &self,
            id: AnalysisId,
            generation: u32,
            report: &landscape_core::Report,
        ) -> landscape_db::Result<landscape_core::Applied> {
            self.0.complete(id, generation, report).await
        }
        async fn fail(
            &self,
            id: AnalysisId,
            generation: u32,
            refused: landscape_db::Refused<'_>,
        ) -> landscape_db::Result<landscape_core::Applied> {
            self.0.fail(id, generation, refused).await
        }
        async fn reclaim_stale(&self, max_age: chrono::Duration) -> landscape_db::Result<u64> {
            self.0.reclaim_stale(max_age).await
        }
        async fn count_with_status(&self, status: AnalysisStatus) -> landscape_db::Result<i64> {
            self.0.count_with_status(status).await
        }
    }

    /// A store that holds the very first `enqueue` until it is let go.
    ///
    /// This is what puts a midnight *inside* a request: one request is stopped with the gate in
    /// its hand while others queue behind it, and the test moves the clock in between.
    #[derive(Debug)]
    struct HoldsTheFirstEnqueue {
        inner: Arc<dyn Store>,
        seen: std::sync::atomic::AtomicUsize,
        go: Arc<tokio::sync::Notify>,
    }

    #[async_trait::async_trait]
    impl Store for HoldsTheFirstEnqueue {
        async fn enqueue(&self, new: &NewAnalysis) -> landscape_db::Result<Analysis> {
            if self.seen.fetch_add(1, std::sync::atomic::Ordering::SeqCst) == 0 {
                self.go.notified().await;
            }
            self.inner.enqueue(new).await
        }
        async fn get(&self, id: AnalysisId) -> landscape_db::Result<Analysis> {
            tokio::task::yield_now().await;
            self.inner.get(id).await
        }
        async fn claim_next(&self) -> landscape_db::Result<Option<Analysis>> {
            self.inner.claim_next().await
        }
        async fn save_progress(
            &self,
            id: AnalysisId,
            generation: u32,
            report: &landscape_core::Report,
        ) -> landscape_db::Result<landscape_core::Applied> {
            self.inner.save_progress(id, generation, report).await
        }
        async fn complete(
            &self,
            id: AnalysisId,
            generation: u32,
            report: &landscape_core::Report,
        ) -> landscape_db::Result<landscape_core::Applied> {
            self.inner.complete(id, generation, report).await
        }
        async fn fail(
            &self,
            id: AnalysisId,
            generation: u32,
            refused: landscape_db::Refused<'_>,
        ) -> landscape_db::Result<landscape_core::Applied> {
            self.inner.fail(id, generation, refused).await
        }
        async fn reclaim_stale(&self, max_age: chrono::Duration) -> landscape_db::Result<u64> {
            self.inner.reclaim_stale(max_age).await
        }
        async fn count_with_status(&self, status: AnalysisStatus) -> landscape_db::Result<i64> {
            self.inner.count_with_status(status).await
        }
    }

    /// All three, for the tests that need to see what an address is holding.
    fn capped_parts(limit: usize) -> (axum::Router, Arc<dyn Store>, Arc<crate::cap::Cap>) {
        let store: Arc<dyn Store> = Arc::new(MemoryStore::new());
        let cap = Arc::new(crate::cap::Cap::of(limit));
        let app = router(AppState {
            store: Arc::clone(&store),
            cap: Arc::clone(&cap),
            discovery: false,
        });
        (app, store, cap)
    }

    /// The same POST, arriving through a reverse proxy that has appended what it saw.
    fn post_from(client: &str, prompt: &str) -> Request<Body> {
        Request::builder()
            .method("POST")
            .uri("/api/analyses")
            .header("content-type", "application/json")
            .header("x-forwarded-for", client)
            .body(Body::from(
                serde_json::json!({ "prompt": prompt }).to_string(),
            ))
            .expect("build request")
    }

    #[tokio::test]
    async fn a_third_analysis_in_one_day_is_refused_with_something_to_do_about_it() {
        // ROADMAP D6. One request starts minutes of work on the only model this machine has,
        // and until now a public URL was an invitation to spend all of it.
        let app = app_capped_at(2);
        for _ in 0..2 {
            let res = app
                .clone()
                .oneshot(post_from("198.51.100.7", IDEA))
                .await
                .expect("response");
            assert_eq!(res.status(), StatusCode::CREATED);
        }

        let res = app
            .oneshot(post_from("198.51.100.7", IDEA))
            .await
            .expect("response");
        assert_eq!(res.status(), StatusCode::TOO_MANY_REQUESTS);

        let body = json_body(res).await;
        assert!(
            body["error"]
                .as_str()
                .is_some_and(|e| e.contains("limit of 2")),
            "the refusal does not say what the limit is: {body}"
        );
        assert!(
            body["remedy"]
                .as_str()
                .is_some_and(|r| r.contains("does not count")),
            "a refusal with no way forward is a dead end: {body}"
        );
        // `UI_FLOWS.md` section 2.2: the message says **when** it resets. "Tomorrow" is not
        // that - west of UTC the allowance comes back later the same local day.
        assert!(
            body["error"].as_str().is_some_and(|e| e.contains("UTC")),
            "the refusal does not say when the limit resets: {body}"
        );
        // **And how long that is**, which is the thing a reader actually wanted to know. A
        // timestamp in a timezone that is not theirs is arithmetic homework; a reader said so.
        // Both, because the relative one is actionable and the absolute one is checkable.
        assert!(
            body["error"]
                .as_str()
                .is_some_and(|e| e.contains("in about") || e.contains("in under")),
            "the refusal makes a reader work out how long to wait: {body}"
        );
        // **And it says nothing about a list this side cannot see.** The remedy briefly read
        // *"the analyses you have already run are listed below"* — but the cap is keyed by
        // network address and the list is in one browser's storage, so a reader hitting the
        // shared cap from a second device, after clearing site data, or with no storage at all
        // was promised something not on their screen. Review caught it. The sentence is drawn
        // by the browser, the only side that knows; `App.test.tsx` asserts it appears exactly
        // when the list does.
        assert!(
            body["remedy"]
                .as_str()
                .is_some_and(|r| !r.contains("listed below") && !r.contains("lost")),
            "the refusal describes a screen the server cannot see: {body}"
        );
        // A rejected request is fully explained by its own message. A reference here would
        // suggest they have hit something worth reporting.
        assert!(body["reference"].is_null(), "{body}");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn twenty_requests_at_once_still_only_get_two() {
        // **Review reproduced this against the previous commit**: twenty simultaneous POSTs
        // carrying one address produced nine acceptances against a limit of two. Deciding is
        // four steps with an `await` between each - read what the address started, ask the
        // store which still count, enqueue, record - and every gap is one two requests fit
        // through together.
        //
        // The barrier is what makes it a race rather than twenty requests in a queue: every
        // task is held at the same point and released together.
        let inner: Arc<dyn Store> = Arc::new(MemoryStore::new());
        let store: Arc<dyn Store> = Arc::new(YieldsBetweenSteps(inner));
        let app = router(AppState {
            store,
            discovery: false,
            cap: Arc::new(crate::cap::Cap::of(2)),
        });
        let together = Arc::new(tokio::sync::Barrier::new(20));

        let mut attempts = Vec::with_capacity(20);
        for _ in 0..20 {
            let app = app.clone();
            let together = Arc::clone(&together);
            attempts.push(tokio::spawn(async move {
                together.wait().await;
                app.oneshot(post_from("198.51.100.7", IDEA))
                    .await
                    .expect("response")
                    .status()
            }));
        }

        let mut accepted = 0usize;
        for attempt in attempts {
            if attempt.await.expect("the request finished") == StatusCode::CREATED {
                accepted += 1;
            }
        }
        assert_eq!(
            accepted, 2,
            "the limit is two and {accepted} requests were accepted"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn a_queue_that_formed_before_midnight_is_not_admitted_whole() {
        // **Review found that my bound was not a bound.** Making the stale path harmless made
        // it *harmless*, not *finite*: `gate_for` is called before its gate is awaited, so any
        // number of requests can take the old day's gate and queue behind one another. When a
        // new-day request advances the day and clears the gate map, every one of those waiters
        // still holds the old `Arc` — and each then read an empty list, had its writes
        // dropped, and was admitted free. A queue built before midnight was admitted whole, at
        // a boundary anybody can predict.
        //
        // **What is actually allowed to cross is one request**: the one already admitted under
        // the old day when the day turned. Everything behind it has to start again under the
        // new day, which is what `is_current` after the gate is for.
        //
        // **Deterministic, and nothing sleeps.** The clock is a value this test owns, so the
        // midnight happens exactly where it is put.
        use std::sync::atomic::{AtomicI64, AtomicUsize, Ordering};

        fn at(day: u32, hour: u32, minute: u32) -> i64 {
            chrono::NaiveDate::from_ymd_opt(2026, 8, day)
                .expect("a real date")
                .and_hms_opt(hour, minute, 0)
                .expect("a real time")
                .and_utc()
                .timestamp()
        }

        let clock = Arc::new(AtomicI64::new(at(6, 23, 59)));
        // Every request reads the clock once before asking for a gate, with no `await` in
        // between — so counting the reads is how this test knows they are all holding one.
        let reads = Arc::new(AtomicUsize::new(0));
        let cap = {
            let clock = Arc::clone(&clock);
            let reads = Arc::clone(&reads);
            Arc::new(crate::cap::Cap::telling_the_time(
                2,
                Box::new(move || {
                    reads.fetch_add(1, Ordering::SeqCst);
                    chrono::DateTime::from_timestamp(clock.load(Ordering::SeqCst), 0)
                        .expect("a real instant")
                }),
            ))
        };

        let go = Arc::new(tokio::sync::Notify::new());
        let store: Arc<dyn Store> = Arc::new(HoldsTheFirstEnqueue {
            inner: Arc::new(MemoryStore::new()),
            seen: AtomicUsize::new(0),
            go: Arc::clone(&go),
        });
        let app = router(AppState {
            store,
            cap: Arc::clone(&cap),
            discovery: false,
        });

        let send = |app: axum::Router| {
            tokio::spawn(async move {
                app.oneshot(post_from("198.51.100.7", IDEA))
                    .await
                    .expect("response")
                    .status()
            })
        };

        // Four arrive a minute before midnight. The first takes the gate and stops inside the
        // store; the other three queue behind it.
        let queued: Vec<_> = (0..4).map(|_| send(app.clone())).collect();
        while reads.load(Ordering::SeqCst) < 4 {
            tokio::task::yield_now().await;
        }

        // Midnight. A fresh request arrives, rolls the day forward and clears the gate map —
        // which is what strands the four behind a gate that no longer belongs to anything.
        clock.store(at(7, 0, 30), Ordering::SeqCst);
        let after_midnight = send(app.clone())
            .await
            .expect("the new day's request finished");
        assert_eq!(after_midnight, StatusCode::CREATED);

        // Now let the one holding the old gate finish, and the three behind it wake.
        go.notify_one();

        let mut accepted = usize::from(after_midnight == StatusCode::CREATED);
        for one in queued {
            if one.await.expect("the request finished") == StatusCode::CREATED {
                accepted += 1;
            }
        }

        // Two a day, plus the single request that was already admitted when the day turned.
        // Without the retry this was five: the whole queue, free.
        assert!(
            accepted <= 3,
            "a queue that formed before midnight was admitted whole: {accepted} accepted \
             against a limit of 2"
        );
    }

    #[tokio::test]
    async fn failed_runs_are_forgotten_rather_than_asked_about_for_ever() {
        // Failures are free, so an address can collect them without limit - and each one would
        // otherwise cost a store read on every later request, turning n failures into n² reads
        // across a day while this map grows for as long as the process lives. Review found it.
        let (app, store, cap) = capped_parts(1);

        for _ in 0..3 {
            let res = app
                .clone()
                .oneshot(post_from("198.51.100.7", IDEA))
                .await
                .expect("response");
            assert_eq!(res.status(), StatusCode::CREATED);

            let claimed = store
                .claim_next()
                .await
                .expect("claim")
                .expect("one queued");
            store
                .fail(
                    claimed.id,
                    claimed.generation,
                    landscape_db::Refused {
                        kind: landscape_core::Failure::NoSubject,
                        reason: "no company named",
                        choices: &[],
                    },
                )
                .await
                .expect("fail");
        }

        // A fourth request reconciles, and what it leaves behind is its own run alone.
        let res = app
            .oneshot(post_from("198.51.100.7", IDEA))
            .await
            .expect("response");
        assert_eq!(res.status(), StatusCode::CREATED);

        let held = cap.started_today("198.51.100.7", chrono::Utc::now().date_naive());
        assert_eq!(
            held.len(),
            1,
            "three failed runs are still being asked about: {held:?}"
        );
    }

    #[tokio::test]
    async fn one_visitor_spending_the_day_does_not_refuse_the_next_one() {
        // The way a cap can be worse than no cap: the first person through exhausting it for
        // everybody who arrives after them.
        let app = app_capped_at(1);
        let _ = app.clone().oneshot(post_from("198.51.100.7", IDEA)).await;

        let res = app
            .oneshot(post_from("203.0.113.9", IDEA))
            .await
            .expect("response");
        assert_eq!(res.status(), StatusCode::CREATED);
    }

    #[tokio::test]
    async fn a_forged_forwarded_header_does_not_buy_a_fresh_allowance() {
        // **The bypass.** A proxy appends what it saw rather than replacing the header, so a
        // client that sends its own value gets `theirs, ours` - and reading the leftmost entry
        // would let one machine mint a new quota with every request.
        let app = app_capped_at(1);
        let res = app
            .clone()
            .oneshot(post_from("198.51.100.7", IDEA))
            .await
            .expect("response");
        assert_eq!(res.status(), StatusCode::CREATED);

        let res = app
            .oneshot(post_from("10.0.0.99, 198.51.100.7", IDEA))
            .await
            .expect("response");
        assert_eq!(
            res.status(),
            StatusCode::TOO_MANY_REQUESTS,
            "a client that names itself in X-Forwarded-For got a second allowance"
        );
    }

    #[tokio::test]
    async fn reading_a_report_is_never_capped() {
        // The cap counts where a run *starts*. A reader who has already begun one - watching
        // it arrive, reloading its URL, opening the examples - must never be cut off, and a
        // middleware over the whole router would have done exactly that.
        let app = app_capped_at(1);
        let created = app
            .clone()
            .oneshot(post_from("198.51.100.7", IDEA))
            .await
            .expect("response");
        let id = json_body(created).await["id"]
            .as_str()
            .expect("an id")
            .to_owned();

        for _ in 0..5 {
            let res = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri(format!("/api/analyses/{id}"))
                        .header("x-forwarded-for", "198.51.100.7")
                        .body(Body::empty())
                        .expect("build request"),
                )
                .await
                .expect("response");
            assert_eq!(res.status(), StatusCode::OK, "a reader was cut off");
        }

        // And the first screen still offers its ideas to somebody who has used their runs.
        let res = app
            .oneshot(
                Request::builder()
                    .uri("/api/examples")
                    .header("x-forwarded-for", "198.51.100.7")
                    .body(Body::empty())
                    .expect("build request"),
            )
            .await
            .expect("response");
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn a_request_with_no_proxy_in_front_of_it_is_not_capped() {
        // A laptop. `landscape dev` with a two-a-day limit would be unusable to the person
        // building it, and the abuse this exists for arrives over the internet.
        let app = app_capped_at(1);
        for _ in 0..4 {
            let res = app
                .clone()
                .oneshot(post_analysis(IDEA))
                .await
                .expect("response");
            assert_eq!(res.status(), StatusCode::CREATED);
        }
    }

    #[tokio::test]
    async fn a_prompt_that_was_refused_costs_nothing() {
        // `PRODUCT_SPEC.md` §2.1: *"a failed analysis costs nothing"*. The first version of
        // this counted before the prompt was parsed, so a typo spent half of somebody's day -
        // and nothing had started, no page was read and no model was asked anything. Review
        // found it.
        let app = app_capped_at(1);
        let res = app
            .clone()
            .oneshot(post_from("198.51.100.7", "a crm"))
            .await
            .expect("response");
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);

        let res = app
            .oneshot(post_from("198.51.100.7", IDEA))
            .await
            .expect("response");
        assert_eq!(
            res.status(),
            StatusCode::CREATED,
            "a rejected prompt was charged for"
        );
    }

    #[tokio::test]
    async fn an_analysis_the_worker_failed_gives_the_allowance_back() {
        // The other half, and the one that decides the shape: a run can fail *after* it was
        // accepted, in a different process. There is nowhere a reservation could be refunded
        // to - so nothing is reserved, and how many still count is asked of the store.
        let (app, store) = capped_with_store(1);

        let created = app
            .clone()
            .oneshot(post_from("198.51.100.7", IDEA))
            .await
            .expect("response");
        assert_eq!(created.status(), StatusCode::CREATED);

        // Spent, until it fails.
        let res = app
            .clone()
            .oneshot(post_from("198.51.100.7", IDEA))
            .await
            .expect("response");
        assert_eq!(res.status(), StatusCode::TOO_MANY_REQUESTS);

        // What the worker does to a prompt naming no company: claim it, then fail it.
        let claimed = store
            .claim_next()
            .await
            .expect("claim")
            .expect("one queued");
        store
            .fail(
                claimed.id,
                claimed.generation,
                landscape_db::Refused {
                    kind: landscape_core::Failure::NoSubject,
                    reason: "no company named",
                    choices: &[],
                },
            )
            .await
            .expect("fail");

        let res = app
            .oneshot(post_from("198.51.100.7", IDEA))
            .await
            .expect("response");
        assert_eq!(
            res.status(),
            StatusCode::CREATED,
            "a failed analysis was still charged for"
        );
    }

    #[tokio::test]
    async fn the_event_stream_opens_for_an_analysis_that_exists() {
        // A reader watching a report fill in. The stream is opened before anything has been
        // written, which is exactly when a reader opens it.
        let store: Arc<dyn Store> = Arc::new(MemoryStore::new());
        let created = store
            .enqueue(&landscape_core::NewAnalysis::parse("compare basecamp.com").expect("valid"))
            .await
            .expect("enqueue");

        let res = router(AppState::new(Arc::clone(&store), false))
            .oneshot(
                Request::builder()
                    .uri(format!("/api/analyses/{}/events", created.id))
                    .body(Body::empty())
                    .expect("build request"),
            )
            .await
            .expect("response");

        assert_eq!(res.status(), StatusCode::OK);
        let content_type = res
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default()
            .to_owned();
        assert!(
            content_type.starts_with("text/event-stream"),
            "not a stream: {content_type}"
        );
    }

    #[tokio::test]
    async fn a_stream_for_an_unknown_analysis_is_not_found() {
        // Resolved before the stream opens. A connection that opens and then says nothing
        // is indistinguishable, from the browser, from an analysis that is merely slow.
        let res = app()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/analyses/{}/events", uuid::Uuid::new_v4()))
                    .body(Body::empty())
                    .expect("build request"),
            )
            .await
            .expect("response");
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn a_malformed_id_is_not_found_rather_than_a_bad_request() {
        // Same reading as `get_analysis`: telling a prober that an id is *shaped* wrong
        // tells them what our ids look like.
        let res = app()
            .oneshot(
                Request::builder()
                    .uri("/api/analyses/not-a-uuid/events")
                    .body(Body::empty())
                    .expect("build request"),
            )
            .await
            .expect("response");
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn health_reports_the_queue_depth() {
        let res = app()
            .oneshot(
                Request::builder()
                    .uri("/api/health")
                    .body(Body::empty())
                    .expect("build request"),
            )
            .await
            .expect("response");

        assert_eq!(res.status(), StatusCode::OK);
        let body = json_body(res).await;
        assert_eq!(body["status"], "ok");
        assert_eq!(body["queued"], 0);
    }

    #[tokio::test]
    async fn the_examples_arrive_with_the_sentence_that_explains_them() {
        // The endpoint the first screen calls. It carries the note as well as the list, so an
        // interface cannot render the chips and leave off what is curated about them.
        let res = app()
            .oneshot(
                Request::builder()
                    .uri("/api/examples")
                    .body(Body::empty())
                    .expect("build request"),
            )
            .await
            .expect("response");

        assert_eq!(res.status(), StatusCode::OK);
        let body = json_body(res).await;
        let listed = body["examples"].as_array().expect("a list of examples");
        assert!(!listed.is_empty(), "the first screen would have no chips");
        assert!(
            body["note"]
                .as_str()
                .is_some_and(|n| n.contains("chosen by hand")),
            "the list arrived without the sentence that qualifies it: {body}"
        );
        // **And whether these will run at all**, beside the ideas somebody is about to click.
        // Without an engine every one of them refuses, and a reader who clicks first spends an
        // analysis to be told about an environment variable. `app()` configures none.
        assert_eq!(
            body["discovery"],
            serde_json::json!(false),
            "the first screen cannot tell a reader these will not work: {body}"
        );

        // **The curation fixture stays on this side of the wire.** `companies` and `why` are
        // what `--check` verifies against; a browser that received them could render them, and
        // a field sent for no reason is one displayed by accident two changes later.
        for example in listed {
            let prompt = example["prompt"].as_str().unwrap_or_default();
            assert!(
                !prompt.contains('.'),
                "an example reads like a domain, so nothing would be discovered: {example}"
            );
            assert_eq!(
                prompt, example["idea"],
                "the prompt is not the idea: {example}"
            );
            for gone in ["companies", "why"] {
                assert!(
                    example.get(gone).is_none(),
                    "{gone} reached the browser, which has no use for it: {example}"
                );
            }
        }
    }

    #[tokio::test]
    async fn a_valid_prompt_is_queued_and_returned() {
        let res = app()
            .oneshot(post_analysis(
                "an app that helps small farms sell to restaurants",
            ))
            .await
            .expect("response");

        assert_eq!(res.status(), StatusCode::CREATED);
        let body = json_body(res).await;
        assert_eq!(body["status"], "queued");
        assert!(
            body["report"].is_null(),
            "a queued analysis has no report yet"
        );
        assert!(body["id"].is_string());
    }

    #[tokio::test]
    async fn a_short_prompt_is_rejected_with_something_a_person_can_act_on() {
        let res = app()
            .oneshot(post_analysis("a crm"))
            .await
            .expect("response");

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        let body = json_body(res).await;
        let message = body["error"].as_str().expect("error is a string");
        assert!(
            message.contains('8'),
            "the message should say what the limit is, got {message:?}"
        );
        assert!(
            body["remedy"].is_string(),
            "a rejection tells them what to do"
        );
    }

    #[tokio::test]
    async fn a_body_sent_without_the_json_header_says_so_in_json() {
        // curl without -H content-type is the single most common way to hit this endpoint
        // by hand. axum's own rejection is plain text with no remedy, which breaks the
        // contract every other rejection here keeps.
        let res = app()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/analyses")
                    .body(Body::from(r#"{"prompt":"a crm"}"#))
                    .expect("build request"),
            )
            .await
            .expect("response");

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        let body = json_body(res).await;
        let remedy = body["remedy"].as_str().expect("a remedy is offered");
        assert!(
            remedy.contains("content-type"),
            "the remedy should name the missing header, got {remedy:?}"
        );
    }

    #[tokio::test]
    async fn a_malformed_body_is_told_apart_from_a_missing_header() {
        let res = app()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/analyses")
                    .header("content-type", "application/json")
                    .body(Body::from("{not json"))
                    .expect("build request"),
            )
            .await
            .expect("response");

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        let body = json_body(res).await;
        let message = body["error"].as_str().expect("error is a string");
        assert!(
            message.contains("valid JSON"),
            "a syntax error should say so rather than blaming the header, got {message:?}"
        );
    }

    #[tokio::test]
    async fn a_body_missing_the_prompt_field_says_which_field() {
        let res = app()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/analyses")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"idea":"a crm"}"#))
                    .expect("build request"),
            )
            .await
            .expect("response");

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        let remedy = json_body(res).await["remedy"]
            .as_str()
            .expect("a remedy is offered")
            .to_owned();
        assert!(remedy.contains("prompt"), "got {remedy:?}");
    }

    #[tokio::test]
    async fn an_analysis_can_be_read_back_by_id() {
        let app = app();

        let created = app
            .clone()
            .oneshot(post_analysis("a tool that chases unpaid invoices"))
            .await
            .expect("response");
        let id = json_body(created).await["id"]
            .as_str()
            .expect("id is a string")
            .to_owned();

        let res = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/analyses/{id}"))
                    .body(Body::empty())
                    .expect("build request"),
            )
            .await
            .expect("response");

        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(json_body(res).await["id"], id);
    }

    #[tokio::test]
    async fn an_unknown_id_is_not_found() {
        let res = app()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/analyses/{}", uuid::Uuid::new_v4()))
                    .body(Body::empty())
                    .expect("build request"),
            )
            .await
            .expect("response");

        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn a_malformed_id_is_not_found_rather_than_a_server_error() {
        let res = app()
            .oneshot(
                Request::builder()
                    .uri("/api/analyses/not-a-uuid")
                    .body(Body::empty())
                    .expect("build request"),
            )
            .await
            .expect("response");

        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn an_internal_failure_never_leaks_detail() {
        // Exercised directly: the mapping is what matters, not the cause.
        let response =
            ApiError::Internal("connection string postgres://user:hunter2@db".to_owned())
                .into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let body = json_body(response).await;
        let text = body.to_string();
        assert!(!text.contains("hunter2"), "internal detail leaked: {text}");
        assert!(
            !text.contains("postgres://"),
            "internal detail leaked: {text}"
        );
    }

    #[tokio::test]
    async fn every_response_carries_a_request_id() {
        let response = app()
            .oneshot(
                Request::builder()
                    .uri("/api/health")
                    .body(Body::empty())
                    .expect("build request"),
            )
            .await
            .expect("response");

        let id = response
            .headers()
            .get("x-request-id")
            .expect("every response carries a request id")
            .to_str()
            .expect("ascii");
        assert_eq!(id.len(), 12, "unexpected id shape: {id}");
    }

    #[tokio::test]
    async fn an_id_supplied_by_a_proxy_is_the_one_we_answer_under() {
        // Caddy will sit in front of this. If it stamped one id and we logged another,
        // correlating the two would need a join nobody has written.
        let response = app()
            .oneshot(
                Request::builder()
                    .uri("/api/health")
                    .header("x-request-id", "abc123def456")
                    .body(Body::empty())
                    .expect("build request"),
            )
            .await
            .expect("response");

        assert_eq!(
            response.headers().get("x-request-id").expect("id"),
            "abc123def456"
        );
    }

    #[tokio::test]
    async fn a_forged_id_is_replaced_rather_than_echoed() {
        // A newline in the header would let a caller write their own line into our log.
        // The rejection has to happen end to end, not only in the parser's unit test.
        let response = app()
            .oneshot(
                Request::builder()
                    .uri("/api/health")
                    .header("x-request-id", "aaa\tERROR forged")
                    .body(Body::empty())
                    .expect("build request"),
            )
            .await
            .expect("response");

        let id = response
            .headers()
            .get("x-request-id")
            .expect("id")
            .to_str()
            .expect("ascii");
        assert!(!id.contains("forged"), "a forged id was echoed: {id}");
        assert_eq!(id.len(), 12);
    }

    #[tokio::test]
    async fn an_internal_failure_returns_a_reference_that_matches_its_header() {
        // The whole point. The number on the screen has to be the number in the log, and
        // the header is the only other place it is published — so if those two disagree,
        // one of them is misleading whoever is trying to find the failure.
        //
        // A handler that simply fails, rather than a store rigged to break: the mapping
        // from an internal error to a response is the subject, and routing a real failure
        // through storage to reach it would test three things to assert one.
        let app = Router::new()
            .route(
                "/boom",
                get(|| async { Err::<(), ApiError>(ApiError::Internal("a secret".to_owned())) }),
            )
            .layer(axum::middleware::from_fn(crate::request_id::layer));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/boom")
                    .body(Body::empty())
                    .expect("build request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let header = response
            .headers()
            .get("x-request-id")
            .expect("id")
            .to_str()
            .expect("ascii")
            .to_owned();

        let body = json_body(response).await;
        let reference = body
            .get("reference")
            .and_then(|r| r.as_str())
            .expect("an internal failure carries a reference");
        assert_eq!(reference, header);
        assert!(
            body["remedy"].as_str().expect("remedy").contains(reference),
            "the remedy should tell the reader what to quote: {body}"
        );
    }

    #[tokio::test]
    async fn a_rejected_prompt_carries_no_reference() {
        // A reference here would tell someone who mistyped that they have found a fault
        // worth reporting. They have found a typo, and the message already says so.
        let response = app()
            .oneshot(post_analysis("a crm"))
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = json_body(response).await;
        assert!(
            body.get("reference").is_none(),
            "a 400 should not carry a reference: {body}"
        );
    }
}
