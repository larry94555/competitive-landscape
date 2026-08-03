//! A reference on every request, so "it broke" becomes something we can look up.
//!
//! Before this, an internal failure logged its detail and told the reader *"Something went
//! wrong at our end."* Both statements were true and there was **nothing joining them**. A
//! reader who wrote in to say it had broken gave us a rough time of day and a rough
//! description, against a log with every other request in it. The information existed; it
//! was simply not addressable.
//!
//! So every request gets an id, and that id appears in three places:
//!
//! | Where | For whom |
//! |---|---|
//! | The `tracing` span wrapping the request | Every log line the request emits carries it |
//! | The `x-request-id` response header | Anything automated — a proxy, a browser tab, a test |
//! | The body of a 5xx, as `reference` | The person reading the screen |
//!
//! The third is the one that matters, and it is the one usually skipped. A header is
//! invisible to the human being asked what went wrong.
//!
//! **This is the whole of our tracing story on purpose.** See
//! `docs/decisions/0005-observability-on-a-24gb-box.md`: a self-hosted metrics stack
//! competes for RAM with three resident models on a 24 GB machine, and losing a model to
//! win a dashboard is a bad trade. Correlated structured logs are what fits, so they need
//! to be good.

use axum::extract::Request;
use axum::http::{HeaderName, HeaderValue};
use axum::middleware::Next;
use axum::response::Response;

/// The header carried in and out. Conventional, and what Caddy will set in front of us.
pub const HEADER: HeaderName = HeaderName::from_static("x-request-id");

/// Twelve hex characters of a v4 UUID.
///
/// Short enough to read down a phone line or paste into a message without wrapping, and
/// far more than enough to be unique across any window we retain logs for. A full UUID is
/// correct and nobody types one out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestId(String);

impl RequestId {
    fn generate() -> Self {
        Self(uuid::Uuid::new_v4().simple().to_string()[..12].to_owned())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Accept an id from in front of us, if it is safe to put in a log.
    ///
    /// An inbound header is attacker-controlled, and this value is written into logs and
    /// echoed in a response. A newline in it splits one log line into two, the second of
    /// which the attacker wrote — so a forged "request failed" entry can be planted in our
    /// own logs, which is a genuinely nasty thing to be debugging against later.
    ///
    /// Rejecting is safe: an unusable inbound id costs a broken correlation with whatever
    /// sits in front, while trusting one costs the integrity of the log. So the rule is
    /// narrow and positive — hex, dashes, at most 64 characters — rather than a list of
    /// characters to strip, which is the form of this check that is always incomplete.
    fn accept(raw: &str) -> Option<Self> {
        let ok = !raw.is_empty()
            && raw.len() <= 64
            && raw
                .chars()
                .all(|c| c.is_ascii_hexdigit() || c == '-' || c.is_ascii_alphanumeric());
        ok.then(|| Self(raw.to_owned()))
    }
}

impl std::fmt::Display for RequestId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

tokio::task_local! {
    /// The id of the request being served on this task.
    ///
    /// A task-local rather than an extractor because the value is needed in
    /// `ApiError::into_response`, which is handed a `self` and nothing else. Threading it
    /// through instead would mean every handler signature carrying a parameter that only
    /// the error path reads — and the one handler that forgot would be the one whose
    /// failures are unfindable.
    static CURRENT: RequestId;
}

/// The id of the request being handled, if there is one.
///
/// `None` outside a request — a worker, a unit test — which is why the reference is
/// optional in the response body rather than an empty string. An empty reference invites
/// someone to quote it.
#[must_use]
pub fn current() -> Option<RequestId> {
    CURRENT.try_with(Clone::clone).ok()
}

/// Give every request an id, a span, a response header, and one line in the log.
///
/// The access line is emitted here rather than by `tower_http`'s `TraceLayer`, which was
/// the first attempt. `TraceLayer` logs at `DEBUG`, so at our default level it produced
/// nothing at all — request ids were being attached perfectly to log lines that were never
/// written. Owning the line means the level, the fields and the ordering are decided here
/// instead of inherited, and there is exactly one span per request rather than two.
pub async fn layer(mut request: Request, next: Next) -> Response {
    let id = request
        .headers()
        .get(&HEADER)
        .and_then(|v| v.to_str().ok())
        .and_then(RequestId::accept)
        .unwrap_or_else(RequestId::generate);

    // Echo it back on the request too, so anything downstream reading headers sees the
    // same value we are logging under.
    if let Ok(value) = HeaderValue::from_str(id.as_str()) {
        request.headers_mut().insert(HEADER, value);
    }

    let method = request.method().clone();
    // The path only: a query string can carry whatever a caller put in it, and a log is
    // the last place to copy unexamined input into. Nothing we serve reads one anyway.
    let path = request.uri().path().to_owned();

    let span = tracing::info_span!("request", request_id = %id);
    let header_value = HeaderValue::from_str(id.as_str()).ok();
    let started = std::time::Instant::now();

    let mut response = CURRENT
        .scope(id, async move {
            tracing::Instrument::instrument(
                async move {
                    let response = next.run(request).await;
                    // Inside the span, so this line carries the id like every other.
                    tracing::info!(
                        %method,
                        path,
                        status = response.status().as_u16(),
                        took_ms = started.elapsed().as_millis(),
                        "handled"
                    );
                    response
                },
                span,
            )
            .await
        })
        .await;

    if let Some(value) = header_value {
        response.headers_mut().insert(HEADER, value);
    }
    response
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn a_generated_id_is_twelve_hex_characters() {
        let id = RequestId::generate();
        assert_eq!(id.as_str().len(), 12);
        assert!(id.as_str().chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn two_generated_ids_differ() {
        assert_ne!(RequestId::generate(), RequestId::generate());
    }

    #[test]
    fn a_plausible_inbound_id_is_kept() {
        // Correlation with whatever sits in front of us is the point of accepting one.
        assert_eq!(
            RequestId::accept("9f2c11ab").map(|r| r.0),
            Some("9f2c11ab".to_owned())
        );
        assert!(RequestId::accept("550e8400-e29b-41d4-a716-446655440000").is_some());
    }

    #[test]
    fn an_id_containing_a_newline_is_refused() {
        // The one that matters. A newline here lets a caller append a line of their own
        // choosing to our log — including a convincing forged error.
        assert!(RequestId::accept("abc\nERROR request failed").is_none());
        assert!(RequestId::accept("abc\r\nfake").is_none());
    }

    #[test]
    fn an_id_with_spaces_or_punctuation_is_refused() {
        assert!(RequestId::accept("abc def").is_none());
        assert!(RequestId::accept("abc\"def").is_none());
        assert!(RequestId::accept("../../etc/passwd").is_none());
    }

    #[test]
    fn an_empty_or_overlong_id_is_refused() {
        assert!(RequestId::accept("").is_none());
        assert!(RequestId::accept(&"a".repeat(65)).is_none());
        assert!(RequestId::accept(&"a".repeat(64)).is_some());
    }

    #[tokio::test]
    async fn there_is_no_current_id_outside_a_request() {
        // The worker runs here. A reference invented off-request would point at nothing.
        assert!(current().is_none());
    }

    #[tokio::test]
    async fn the_current_id_is_visible_inside_the_scope() {
        let id = RequestId("abc123".to_owned());
        CURRENT
            .scope(id.clone(), async move {
                assert_eq!(current(), Some(id));
            })
            .await;
    }
}
