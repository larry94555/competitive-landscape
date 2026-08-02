//! Routes.
//!
//! Small on purpose. Everything that is not HTTP belongs in `landscape-core` (rules) or
//! `landscape-db` (storage); this layer parses, delegates and maps errors.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use landscape_core::{Analysis, AnalysisId, AnalysisStatus, NewAnalysis};
use landscape_db::Store;
use serde::{Deserialize, Serialize};

use crate::error::ApiError;

#[derive(Clone)]
pub struct AppState {
    pub store: Arc<dyn Store>,
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState").finish_non_exhaustive()
    }
}

/// Every route the server serves.
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/analyses", post(create_analysis))
        .route("/api/analyses/{id}", get(get_analysis))
        .with_state(state)
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

#[derive(Debug, Deserialize)]
struct CreateAnalysis {
    prompt: String,
}

async fn create_analysis(
    State(state): State<AppState>,
    Json(body): Json<CreateAnalysis>,
) -> Result<(StatusCode, Json<Analysis>), ApiError> {
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
        router(AppState {
            store: Arc::new(MemoryStore::new()),
        })
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
}
