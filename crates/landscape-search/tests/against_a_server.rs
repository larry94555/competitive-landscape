//! The join between the client and the parse, over a real socket.
//!
//! Every other test in this crate exercises one side: [`landscape_search::searx::hits_from_json`]
//! against a frozen body, or the admission rules against hand-made hits. **Neither of them
//! sends anything.** The mistakes register's recurring finding is that the joins are where
//! the defects live — Run 16 found two by driving the real stream rather than its helpers —
//! and the join here has four things in it that a unit test cannot see: the URL the query is
//! appended to, the percent-encoding of a template full of quotes and colons, what a
//! non-200 becomes, and whether the body actually gets read.
//!
//! So this stands up a listener on `127.0.0.1:0`, points a real [`Searx`] at it, and asserts
//! on **what arrived at the server** as well as on what came back.
//!
//! It is not a SearXNG. It answers the way SearXNG's `format=json` documents, which is the
//! contract this code is written against — the instance itself is stood up by
//! `docker compose --profile search up -d searxng`, and walking that is a deployment step
//! rather than a test.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::time::Duration;

use landscape_discover::probes::Answers;
use landscape_search::{queries, SourceProvider as _};
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::TcpListener;

/// Run a call against a server that answers one request, returning what came back **and what
/// arrived** — the second half is the point, since the encoding of the query is only
/// observable from the server's side.
macro_rules! with_server {
    ($status:expr, $body:expr, |$base:ident| $call:expr) => {{
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("a loopback port");
        let addr = listener.local_addr().expect("the bound address");
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("a connection");
            let mut buf = vec![0u8; 8192];
            let read = socket.read(&mut buf).await.expect("the request");
            let request = String::from_utf8_lossy(&buf[..read]).into_owned();
            let body: &str = $body;
            let response = format!(
                "HTTP/1.1 {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                $status,
                body.len(),
                body
            );
            socket.write_all(response.as_bytes()).await.expect("the response");
            socket.shutdown().await.ok();
            request
        });
        let $base = format!("http://{addr}");
        let outcome = $call;
        let request = tokio::time::timeout(Duration::from_secs(5), server)
            .await
            .expect("the server finished")
            .expect("the server did not panic");
        (outcome, request)
    }};
}

/// A server that keeps sending body until the client gives up, reporting how much it wrote.
///
/// No `Content-Length`, so the client cannot refuse it on the header alone — this is the
/// case where the cap has to be enforced **while the bytes arrive**. The server stops on its
/// own at `give_up_after` so a client that reads everything fails the assertion instead of
/// hanging the suite.
async fn a_server_that_will_not_stop(
    give_up_after: usize,
) -> (String, tokio::task::JoinHandle<usize>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("a loopback port");
    let addr = listener.local_addr().expect("the bound address");
    let handle = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("a connection");
        let mut buf = vec![0u8; 8192];
        let _ = socket.read(&mut buf).await;
        socket
            .write_all(
                b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n",
            )
            .await
            .expect("the headers");
        let chunk = vec![b'x'; 64 * 1024];
        let mut written = 0usize;
        while written < give_up_after {
            if socket.write_all(&chunk).await.is_err() {
                break; // the client hung up, which is the outcome under test
            }
            written += chunk.len();
        }
        written
    });
    (format!("http://{addr}"), handle)
}

fn a_query() -> queries::Query {
    queries::for_questions("Help Scout", &[Answers::Changes])
        .into_iter()
        .next()
        .expect("one query")
}

#[tokio::test]
async fn a_query_reaches_the_engine_encoded_and_comes_back_as_pages() {
    const BODY: &str = r#"{"results":[
        {"url":"https://www.helpscout.com/changelog/","title":"Changelog","content":"news"},
        {"url":"https://g2.com/products/help-scout","title":"Reviews","content":"news"}
    ]}"#;

    let query = a_query();
    let (hits, request) = with_server!("200 OK", BODY, |base| {
        landscape_search::Searx::new(&base)
            .expect("the client builds")
            .search(&query)
            .await
            .expect("the search succeeded")
    });

    // What arrived at the server. The template is full of quotes and spaces, and an
    // unencoded one produces a malformed request line rather than a search.
    assert!(
        request.starts_with("GET /search?"),
        "the endpoint is not where the query was sent: {request}"
    );
    assert!(
        request.contains("format=json"),
        "the JSON format was not asked for: {request}"
    );
    assert!(
        request.contains("q=%22Help+Scout%22+changelog")
            || request.contains("q=%22Help%20Scout%22"),
        "the quoted phrase did not survive encoding: {request}"
    );
    assert!(
        !request.contains(r#"q="Help Scout""#),
        "a raw space reached the request line: {request}"
    );

    assert_eq!(hits.len(), 2);
    assert_eq!(hits[0].url, "https://www.helpscout.com/changelog/");
}

#[tokio::test]
async fn a_forbidden_response_says_which_number_came_back() {
    // The failure a person actually hits: SearXNG answers 403 for `format=json` until the
    // instance opts in. Flattening that into "search failed" sends them to the wrong file.
    let query = a_query();
    let (err, _request) = with_server!("403 Forbidden", "{}", |base| {
        landscape_search::Searx::new(&base)
            .expect("the client builds")
            .search(&query)
            .await
            .expect_err("a 403 is not a success")
    });
    assert!(err.to_string().contains("403"), "{err}");
}

#[tokio::test]
async fn an_engine_that_is_not_listening_is_a_failure_the_analysis_survives() {
    // Nothing is bound on this port. The point is the variant: a search that cannot be made
    // must be `Unreachable`, which the caller carries on from, rather than a panic or a hang.
    let query = a_query();
    let engine = landscape_search::Searx::new("http://127.0.0.1:1").expect("the client builds");
    let err = engine
        .search(&query)
        .await
        .expect_err("nothing is listening");
    assert!(
        matches!(err, landscape_search::SearchError::Unreachable(_)),
        "{err:?}"
    );
}

#[tokio::test]
async fn a_body_that_will_not_stop_is_abandoned_while_it_arrives() {
    // Review's finding: `HITS_PER_QUERY` truncates the *output* after the whole body has
    // been read and parsed, so a hostile or misconfigured instance could drive unbounded
    // memory and parse work. The limit that holds is `MAX_RESPONSE_BYTES`, and what makes it
    // a limit rather than a measurement is that it fires part-way through.
    //
    // The server sends no `Content-Length`, so nothing can be refused from the header alone.
    let give_up_after = 64 * 1024 * 1024;
    let (base, server) = a_server_that_will_not_stop(give_up_after).await;

    let query = a_query();
    let engine = landscape_search::Searx::new(&base).expect("the client builds");
    let err = engine
        .search(&query)
        .await
        .expect_err("an endless body is not a search result");

    assert!(
        matches!(err, landscape_search::SearchError::TooLarge { .. }),
        "{err:?}"
    );

    let written = tokio::time::timeout(Duration::from_secs(20), server)
        .await
        .expect("the server finished")
        .expect("the server did not panic");
    // The client stopped reading near the cap rather than at the server's give-up point.
    // A generous multiple of the cap, because the socket buffers and the server writes in
    // 64 KiB chunks — the assertion is "bounded", not an exact byte count.
    assert!(
        written < landscape_search::searx::MAX_RESPONSE_BYTES * 8,
        "the client read {written} bytes against a {} byte cap",
        landscape_search::searx::MAX_RESPONSE_BYTES
    );
    assert!(
        written < give_up_after,
        "the client read everything the server was willing to send"
    );
}

#[tokio::test]
async fn a_declared_length_over_the_cap_is_refused_before_the_body() {
    // The cheap half of the same guard. A `Content-Length` is only a hint — a hostile server
    // can understate it, which is why the streaming check above is the real one — but when it
    // is honest it saves the transfer entirely.
    let oversized = landscape_search::searx::MAX_RESPONSE_BYTES + 1;
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("a loopback port");
    let addr = listener.local_addr().expect("the bound address");
    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("a connection");
        let mut buf = vec![0u8; 8192];
        let _ = socket.read(&mut buf).await;
        let headers = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {oversized}\r\nConnection: close\r\n\r\n"
        );
        let _ = socket.write_all(headers.as_bytes()).await;
        // Deliberately never sends the body it promised.
        let _ = socket.shutdown().await;
    });

    let query = a_query();
    let err = landscape_search::Searx::new(&format!("http://{addr}"))
        .expect("the client builds")
        .search(&query)
        .await
        .expect_err("a declared oversized body is refused");
    assert!(
        matches!(err, landscape_search::SearchError::TooLarge { .. }),
        "{err:?}"
    );
}

#[tokio::test]
async fn a_redirect_from_the_engine_is_not_followed() {
    // SearXNG's own query language can turn a search into a redirect: `!!` goes to the first
    // result and an external bang leaves the instance. `queries` refuses those tokens at the
    // text boundary; this is the transport refusing as well, because following one would
    // fetch an arbitrary page before `admit` or the SSRF guard saw a URL.
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("a loopback port");
    let addr = listener.local_addr().expect("the bound address");
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("a connection");
        let mut buf = vec![0u8; 8192];
        let _ = socket.read(&mut buf).await;
        let _ = socket
            .write_all(
                b"HTTP/1.1 302 Found\r\nLocation: http://169.254.169.254/latest/meta-data/\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            )
            .await;
        let _ = socket.shutdown().await;
        // A second connection would mean the client followed it. Nothing accepts one, so a
        // following client would fail on connect instead — which the assertion below sees.
    });

    let query = a_query();
    let err = landscape_search::Searx::new(&format!("http://{addr}"))
        .expect("the client builds")
        .search(&query)
        .await
        .expect_err("a redirect is not a search result");

    // The redirect is surfaced as the status it was, not chased to the link-local address in
    // the Location header.
    match err {
        landscape_search::SearchError::Status { status } => assert_eq!(status, 302),
        other => panic!("a redirect was not reported as its status: {other:?}"),
    }
    tokio::time::timeout(Duration::from_secs(5), server)
        .await
        .expect("the server finished")
        .expect("the server did not panic");
}

#[tokio::test]
async fn what_comes_back_over_the_wire_is_admitted_by_the_same_rules() {
    // The whole slice in one place: a real request, a real response, and the standing each
    // page is given. The aggregator must not end up able to set a table value.
    const BODY: &str = r#"{"results":[
        {"url":"https://www.helpscout.com/changelog/","title":"Changelog","content":"x"},
        {"url":"https://www.g2.com/products/help-scout/reviews","title":"Reviews","content":"x"}
    ]}"#;

    let query = a_query();
    let (hits, _request) = with_server!("200 OK", BODY, |base| {
        landscape_search::Searx::new(&base)
            .expect("the client builds")
            .search(&query)
            .await
            .expect("the search succeeded")
    });

    let admitted = landscape_search::admit::admit(
        "helpscout.com",
        &[],
        &[(query, hits)],
        "searxng",
        landscape_discover::rank::CAP_RUNG_0,
    );
    assert_eq!(admitted.len(), 2, "{admitted:#?}");

    let own = admitted
        .iter()
        .find(|f| f.url.contains("helpscout.com"))
        .expect("the company's own page");
    assert!(
        own.disposition.may_set_a_table_value(),
        "the company's own changelog is not primary: {own:#?}"
    );

    let aggregator = admitted
        .iter()
        .find(|f| f.url.contains("g2.com"))
        .expect("the aggregator");
    assert!(
        !aggregator.disposition.may_set_a_table_value(),
        "an aggregator was allowed to set a figure in a comparison table: {aggregator:#?}"
    );
}
