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
