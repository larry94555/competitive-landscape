//! Turning internal failures into responses.
//!
//! Two rules, both from `docs/PRODUCT_SPEC.md` §2.4: a user is told what happened and what
//! to do about it, and they are never shown internal detail. A database error reaching a
//! browser as a connection string is a security problem as well as a bad experience.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use landscape_core::CoreError;
use landscape_db::StoreError;
use serde::Serialize;

#[derive(Debug)]
pub enum ApiError {
    /// The request was understood and rejected. The message is shown to the user.
    BadRequest(String),
    /// As above, but the caller supplies the remedy because it knows the specific mistake.
    BadRequestWithRemedy {
        message: String,
        remedy: String,
    },
    NotFound,
    /// The caller has had their share of the day.
    ///
    /// Separate from [`Self::BadRequestWithRemedy`] because it is not a mistake: nothing they
    /// typed was wrong, and a message implying otherwise would send them off editing a prompt
    /// that was fine.
    ///
    /// `resets` is carried rather than described. `UI_FLOWS.md` §2.2 requires the message to
    /// say **when** the limit resets, and "tomorrow" is not that: the counter turns over at
    /// midnight UTC, which west of it is later the same local day.
    TooManyToday {
        used: usize,
        limit: usize,
        resets: chrono::DateTime<chrono::Utc>,
    },
    /// Something broke on our side. The detail is logged, never returned.
    Internal(String),
}

#[derive(Debug, Serialize)]
struct Body {
    error: String,
    /// What the reader can do next. Present whenever there is a useful answer.
    #[serde(skip_serializing_if = "Option::is_none")]
    remedy: Option<String>,
    /// The request id, on internal failures only.
    ///
    /// Deliberately absent from 4xx: a rejected prompt is fully explained by its own
    /// message, and a reference number there would suggest the reader has hit something
    /// worth reporting when they have hit something they can fix themselves.
    #[serde(skip_serializing_if = "Option::is_none")]
    reference: Option<String>,
}

/// How long until the counter turns over, in the words the question is asked in.
///
/// **The absolute time is checkable and the relative one is actionable**, so the message
/// carries both. *"It resets at 00:00 on 14 August UTC"* is a fact a reader has to do
/// arithmetic on, in a timezone that is not theirs, to answer the only thing they wanted to
/// know: how long. A reader said so.
///
/// Rounded up, and never *"in 0 hours"*: a wait stated shorter than it is sends somebody back
/// to a door that is still shut.
fn in_about(resets: chrono::DateTime<chrono::Utc>) -> String {
    let left = resets.signed_duration_since(chrono::Utc::now());
    let minutes = left.num_minutes().max(0);
    if minutes < 1 {
        return "in under a minute".to_owned();
    }
    if minutes < 60 {
        return format!("in about {minutes} minute{}", plural(minutes));
    }
    // Rounded up: with 90 minutes left, "in about 1 hour" is a reader coming back to a refusal.
    let hours = (minutes + 59) / 60;
    format!("in about {hours} hour{}", plural(hours))
}

const fn plural(n: i64) -> &'static str {
    if n == 1 {
        ""
    } else {
        "s"
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, body) = match self {
            Self::BadRequest(message) => (
                StatusCode::BAD_REQUEST,
                Body {
                    error: message,
                    remedy: Some("Edit what you typed and try again.".to_owned()),
                    reference: None,
                },
            ),
            Self::BadRequestWithRemedy { message, remedy } => (
                StatusCode::BAD_REQUEST,
                Body {
                    error: message,
                    remedy: Some(remedy),
                    reference: None,
                },
            ),
            Self::NotFound => (
                StatusCode::NOT_FOUND,
                Body {
                    error: "No analysis with that reference.".to_owned(),
                    remedy: Some("It may have been removed. Start a new one.".to_owned()),
                    reference: None,
                },
            ),
            Self::TooManyToday {
                used,
                limit,
                resets,
            } => (
                StatusCode::TOO_MANY_REQUESTS,
                Body {
                    // `concat!` rather than a `\` continuation. This sentence was written with
                    // one, the backslash was lost in an edit, and what reached a reader was
                    // *"the free limit of                          2"* — the defect
                    // `scripts/no_lost_continuations.py` exists for, arriving in the one
                    // message somebody sees when they are already frustrated.
                    error: format!(
                        concat!(
                            "You have started {used} analyses today, which is the free ",
                            "limit of {limit}. You can start another {when}, at {at} UTC."
                        ),
                        used = used,
                        limit = limit,
                        when = in_about(resets),
                        at = resets.format("%H:%M on %-d %B")
                    ),
                    // **The honest remedy, not the specified one.** `PRODUCT_SPEC.md` §2.1
                    // offers "sign in for 10 a month" here; there are no accounts yet, and
                    // sending somebody to a door that does not exist is worse than telling
                    // them to come back. This sentence changes when signing in does.
                    // **And where the ones already run went.** A reader who has spent both of
                    // today's analyses is at the exact moment they most need to find them, and
                    // this said nothing about them — the work looked lost. The browser keeps
                    // that list, so what belongs here is the pointer to it.
                    remedy: Some(
                        concat!(
                            "The analyses you have already run are listed below, and nothing ",
                            "you did is lost. Each one reads live pages and runs a model for ",
                            "several minutes, so the free limit is per day; an analysis that ",
                            "fails does not count. Or run your own copy - it is open source."
                        )
                        .to_owned(),
                    ),
                    // No reference, for the same reason a rejected prompt has none: nothing
                    // went wrong that anybody needs to look up.
                    reference: None,
                },
            ),
            Self::Internal(detail) => {
                // The detail is logged and never returned. What *is* returned is the
                // request id, which appears on this log line too — so the sentence the
                // reader can quote leads straight to the sentence they cannot see.
                //
                // Without it both halves are true and neither is reachable from the other,
                // which is the state this whole mechanism exists to end.
                tracing::error!(detail, "request failed");
                let reference = crate::request_id::current().map(|id| id.to_string());
                let remedy = match &reference {
                    Some(id) => format!(
                        "Nothing you did caused this. Try again shortly — and if you tell us, \
                         quote {id} and we can find exactly what happened."
                    ),
                    // Outside a request there is no id. Better to drop the sentence than
                    // to invite someone to quote a reference that leads nowhere.
                    None => "Nothing you did caused this. Try again shortly.".to_owned(),
                };
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Body {
                        error: "Something went wrong at our end.".to_owned(),
                        remedy: Some(remedy),
                        reference,
                    },
                )
            }
        };
        (status, Json(body)).into_response()
    }
}

impl From<CoreError> for ApiError {
    fn from(e: CoreError) -> Self {
        // Domain validation messages are written for people and are safe to show.
        Self::BadRequest(e.to_string())
    }
}

impl From<StoreError> for ApiError {
    fn from(e: StoreError) -> Self {
        match e {
            StoreError::NotFound(_) => Self::NotFound,
            other => Self::Internal(other.to_string()),
        }
    }
}
