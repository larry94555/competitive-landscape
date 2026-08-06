//! Routes.
//!
//! Small on purpose. Everything that is not HTTP belongs in `landscape-core` (rules) or
//! `landscape-db` (storage); this layer parses, delegates and maps errors.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
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
}

impl AppState {
    /// The ordinary way to build one: a store, and the cap the environment asks for.
    #[must_use]
    pub fn new(store: Arc<dyn Store>) -> Self {
        Self {
            store,
            cap: Arc::new(crate::cap::Cap::from_env()),
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
/// the tests would pass while asserting the behaviour of a slightly different application
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
async fn list_examples() -> Json<Examples> {
    Json(Examples {
        note: landscape_core::CURATION_NOTE,
        examples: landscape_core::examples()
            .into_iter()
            .map(|example| Listed {
                prompt: example.prompt(),
                example,
            })
            .collect(),
    })
}

#[derive(Debug, Serialize)]
struct Examples {
    /// What is curated and what is not, in one sentence.
    note: &'static str,
    examples: Vec<Listed>,
}

/// One example on the wire.
#[derive(Debug, Serialize)]
struct Listed {
    #[serde(flatten)]
    example: landscape_core::Example,
    /// The text the chip puts in the box.
    ///
    /// Sent rather than assembled in the browser: the format is one decision - which words
    /// join an idea to its companies - and a second copy of it in TypeScript would be a second
    /// thing to keep in step with the parser that has to read the result back.
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
    // **Here and nowhere else.** This is the only request that starts minutes of work on the
    // one model this machine has; reading a report, watching one arrive, or listing the
    // examples cost nothing worth rationing, and capping them would cut off a reader in the
    // middle of something they already started.
    //
    // Before the prompt is parsed, because a prompt that fails still occupied the endpoint,
    // and a cap that only counted valid ones would be walked past with invalid ones.
    let client =
        crate::cap::client_in(headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()));
    if let crate::cap::Allowed::No { used, limit } = state
        .cap
        .allow(client.as_deref(), chrono::Utc::now().date_naive())
    {
        return Err(ApiError::TooManyToday { used, limit });
    }

    let new = NewAnalysis::parse(&body.prompt)?;
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
        router(AppState::new(Arc::new(MemoryStore::new())))
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

    /// A router whose cap allows `limit` a day, so the number under test is the number here
    /// rather than whatever the environment happens to hold.
    fn app_capped_at(limit: usize) -> axum::Router {
        let store: Arc<dyn Store> = Arc::new(MemoryStore::new());
        router(AppState {
            store,
            cap: Arc::new(crate::cap::Cap::of(limit)),
        })
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
                .is_some_and(|r| r.contains("tomorrow")),
            "a refusal with no way forward is a dead end: {body}"
        );
        // A rejected request is fully explained by its own message. A reference here would
        // suggest they have hit something worth reporting.
        assert!(body["reference"].is_null(), "{body}");
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
    async fn a_prompt_that_is_refused_still_costs_one_of_the_day() {
        // Counted before the prompt is parsed. Otherwise the endpoint can be walked past with
        // prompts that fail - each one still occupying it - and the cap counts only the
        // requests that were going to be fine anyway.
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
        assert_eq!(res.status(), StatusCode::TOO_MANY_REQUESTS);
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

        let res = router(AppState::new(Arc::clone(&store)))
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
        // Every chip carries the text it will put in the box - including the companies, so a
        // reader can see and edit the curated part rather than take it on trust.
        for example in listed {
            let prompt = example["prompt"].as_str().unwrap_or_default();
            for company in example["companies"].as_array().expect("companies") {
                let company = company.as_str().unwrap_or_default();
                assert!(
                    prompt.contains(company),
                    "{company} is curated out of sight: {example}"
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
