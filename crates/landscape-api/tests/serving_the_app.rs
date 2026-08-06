//! Whether a visitor gets a page.
//!
//! **This is the test that was missing when the project believed it was one deploy away from a
//! demo.** `router` served `/api/*` and nothing else, the React app existed only behind Vite's
//! dev server, and nothing anywhere asserted that a browser pointed at the binary would receive
//! HTML. `PROJECT_STATUS.md` recorded the guided-demo state as blocked on deployment alone; the
//! truth was that there was nothing to deploy that a person could look at.

#![allow(clippy::expect_used, clippy::panic)]
// Panicking IS how a test reports failure. The lints stay denied everywhere else.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::body::Body;
use axum::http::Request;
use landscape_api::{with_ui, AppState};
use landscape_db::MemoryStore;
use tower::ServiceExt;

/// A stand-in for `web/dist`: an `index.html` and one hashed asset.
fn built_app() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/ui")
}

fn app_with_ui(dir: &Path) -> axum::Router {
    with_ui(AppState::new(Arc::new(MemoryStore::new())), dir)
}

async fn get(app: axum::Router, path: &str) -> (u16, String) {
    let response = app
        .oneshot(
            Request::builder()
                .uri(path)
                .body(Body::empty())
                .expect("a request"),
        )
        .await
        .expect("a response");
    let status = response.status().as_u16();
    let bytes = axum::body::to_bytes(response.into_body(), 1 << 20)
        .await
        .expect("a body");
    (status, String::from_utf8_lossy(&bytes).into_owned())
}

#[tokio::test]
async fn the_root_returns_the_page_rather_than_json() {
    let (status, body) = get(app_with_ui(&built_app()), "/").await;
    assert_eq!(status, 200);
    assert!(
        body.contains("the built single-page app"),
        "a visitor to the root got something that is not the app: {body}"
    );
}

#[tokio::test]
async fn the_hashed_assets_are_served() {
    // Without these the page loads and renders nothing, which looks like a broken deploy
    // rather than a missing route.
    let (status, body) = get(app_with_ui(&built_app()), "/assets/app.js").await;
    assert_eq!(status, 200);
    assert!(body.contains("built"), "{body}");
}

#[tokio::test]
async fn a_deep_link_reaches_the_app_rather_than_a_404() {
    // The single-page app owns its own routing, so a permalink like `/a/<id>` has to arrive at
    // the client. A 404 from the server is the one thing that would stop a shared link — or a
    // refresh mid-run — from working, which is the whole point of having a URL at all.
    let (status, body) = get(app_with_ui(&built_app()), "/a/2f9c1e88-not-a-real-id").await;
    assert_eq!(status, 200, "a deep link 404'd: {body}");
    assert!(body.contains("the built single-page app"), "{body}");
}

#[tokio::test]
async fn the_api_still_wins_over_the_fallback() {
    // The fallback is greedy by design, so the thing worth asserting is that it did not eat
    // the API. A demo where every request returns HTML is worse than one that returns JSON.
    let (status, body) = get(app_with_ui(&built_app()), "/api/health").await;
    assert_eq!(status, 200);
    assert!(
        body.contains("\"status\":\"ok\""),
        "the static fallback swallowed the API: {body}"
    );
}

#[tokio::test]
async fn an_unknown_api_route_is_not_answered_with_the_page() {
    // `/api/*` belongs to the API even when the API has no such route. Returning `index.html`
    // to a mistyped endpoint gives a client HTML where it expects JSON, and the error it then
    // reports is about parsing rather than about the wrong URL.
    let (status, _) = get(app_with_ui(&built_app()), "/api/nothing-here").await;
    assert_ne!(status, 200, "a mistyped API path was answered with the app");
}

#[tokio::test]
async fn the_namespace_root_is_the_apis_too() {
    // `/api/{*rest}` matches paths *below* `/api/` and not `/api` itself, so the namespace
    // root fell through to the fallback and answered with the page. Review found it.
    for path in ["/api", "/api/"] {
        let (status, body) = get(app_with_ui(&built_app()), path).await;
        assert_ne!(status, 200, "{path} was answered with the app: {body}");
        assert!(
            !body.contains("the built single-page app"),
            "{path} returned HTML where a client expects JSON"
        );
    }
}

#[tokio::test]
async fn every_response_carries_a_request_id_including_the_page() {
    // ADR 0005's invariant, and the new surface is the one a visitor actually touches. The
    // fallback was added *after* the middleware, so page and asset responses skipped it: no
    // header, no span, no access line for the only URL anybody types.
    for path in ["/", "/assets/app.js", "/a/some-id", "/api/health"] {
        let response = app_with_ui(&built_app())
            .oneshot(
                Request::builder()
                    .uri(path)
                    .body(Body::empty())
                    .expect("a request"),
            )
            .await
            .expect("a response");
        assert!(
            response.headers().contains_key("x-request-id"),
            "{path} came back with no request id, so nothing in the log joins to it"
        );
    }
}

#[tokio::test]
async fn with_no_build_present_the_api_is_served_alone() {
    // `Feature_Walkthrough.md` tells a developer to run `cargo run` and Vite side by side, and
    // there is no `web/dist` in that arrangement. A missing build is somebody working on the
    // pieces separately, not a failure.
    let nowhere = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/no-such-build");
    let (status, body) = get(app_with_ui(&nowhere), "/api/health").await;
    assert_eq!(status, 200, "the API stopped working without a web build");
    assert!(body.contains("\"status\":\"ok\""), "{body}");

    let (status, _) = get(app_with_ui(&nowhere), "/").await;
    assert_ne!(
        status, 200,
        "the root answered without anything to serve from"
    );
}
