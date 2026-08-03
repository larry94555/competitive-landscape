//! A JSON extractor that fails the way the rest of the API fails.
//!
//! `axum::Json`'s own rejections are plain text with no remedy — "Expected request with
//! `Content-Type: application/json`" and nothing else. That is a reasonable default and
//! the wrong one here: every other rejection this API produces is JSON carrying a message
//! and something the caller can do about it, and a single endpoint that breaks that shape
//! is one a client has to special-case.
//!
//! The messages below name the actual mistake, because the two common ones — a missing
//! header and a malformed body — look identical from the caller's side otherwise.

use axum::extract::rejection::JsonRejection;
use axum::extract::{FromRequest, Request};

use crate::error::ApiError;

/// Drop-in replacement for [`axum::Json`] as an extractor.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Json<T>(pub(crate) T);

impl<S, T> FromRequest<S> for Json<T>
where
    axum::Json<T>: FromRequest<S, Rejection = JsonRejection>,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        match axum::Json::<T>::from_request(req, state).await {
            Ok(axum::Json(value)) => Ok(Self(value)),
            Err(rejection) => Err(describe(&rejection)),
        }
    }
}

/// Turn a rejection into something a person can act on.
fn describe(rejection: &JsonRejection) -> ApiError {
    match rejection {
        JsonRejection::MissingJsonContentType(_) => ApiError::BadRequestWithRemedy {
            message: "This endpoint takes JSON, and the request did not say it was sending any."
                .to_owned(),
            remedy: "Add -H 'content-type: application/json' to the request.".to_owned(),
        },
        JsonRejection::JsonDataError(e) => ApiError::BadRequestWithRemedy {
            message: format!("The JSON was readable but not the shape this endpoint wants: {e}"),
            remedy: "This endpoint expects an object with a \"prompt\" string.".to_owned(),
        },
        JsonRejection::JsonSyntaxError(e) => ApiError::BadRequestWithRemedy {
            message: format!("That is not valid JSON: {e}"),
            remedy: "Check the quoting - a shell often eats the quotes around a JSON body."
                .to_owned(),
        },
        JsonRejection::BytesRejection(_) => ApiError::BadRequestWithRemedy {
            message: "The request body could not be read.".to_owned(),
            remedy: "Send it again.".to_owned(),
        },
        // JsonRejection is non-exhaustive: a future axum may add a variant, and a generic
        // message is better than failing to compile on an upgrade.
        _ => ApiError::BadRequestWithRemedy {
            message: "The request body could not be understood as JSON.".to_owned(),
            remedy: "Send a JSON object with a \"prompt\" string, and set the content type."
                .to_owned(),
        },
    }
}
