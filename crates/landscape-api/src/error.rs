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
/// Rounded up **at every unit**, and never *"in 0 hours"*: a wait stated shorter than it is
/// sends somebody back to a door that is still shut.
///
/// The first version rounded the hours up and let `num_minutes` truncate underneath, so
/// 59m59s read as *"in about 59 minutes"* — the guarantee held at one unit and not at the one
/// below it, which is worse than not making it. Review caught that. Both come from the same
/// ceiling now, so there is one place for it to be wrong.
///
/// **The duration is passed in rather than measured here**, which is the whole of what made
/// the boundaries testable: a function that reads the clock can only be checked by waiting.
fn in_about(left: chrono::Duration) -> String {
    let seconds = left.num_seconds().max(0);
    if seconds == 0 {
        return "in under a minute".to_owned();
    }
    let minutes = (seconds + 59) / 60;
    if minutes < 60 {
        return format!("in about {minutes} minute{}", plural(minutes));
    }
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
                        when = in_about(resets.signed_duration_since(chrono::Utc::now())),
                        at = resets.format("%H:%M on %-d %B")
                    ),
                    // **The honest remedy, not the specified one.** `PRODUCT_SPEC.md` §2.1
                    // offers "sign in for 10 a month" here; there are no accounts yet, and
                    // sending somebody to a door that does not exist is worse than telling
                    // them to come back. This sentence changes when signing in does.
                    // **And not a word about a list this side cannot see.** The first version
                    // said *"the analyses you have already run are listed below"*. That list is
                    // in one browser's storage and **the cap is keyed by network address**, so
                    // a reader who hits it from a second device, after clearing site data, or
                    // with storage unavailable is promised something that is not on their
                    // screen. Review caught it.
                    //
                    // The sentence is true where it is decided: the browser knows whether it
                    // has a list, and says so there. This says what is true of every request.
                    remedy: Some(
                        concat!(
                            "Each analysis reads live pages and runs a model for several ",
                            "minutes, so the free limit is per day; an analysis that fails ",
                            "does not count. Or run your own copy - it is open source."
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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::in_about;
    use chrono::Duration;

    #[test]
    fn a_wait_is_never_stated_shorter_than_it_is() {
        // **The boundaries, because that is where truncation and rounding disagree.** The
        // first version read `num_minutes()`, which truncates: 59m59s came out as "in about
        // 59 minutes", and a reader who came back in 59 minutes found the door still shut.
        // Review caught it. Every one of these is the same question — does the stated wait
        // reach past the real one?
        for (left, expected) in [
            (Duration::seconds(0), "in under a minute"),
            (Duration::seconds(1), "in about 1 minute"),
            (Duration::seconds(59), "in about 1 minute"),
            (Duration::seconds(60), "in about 1 minute"),
            (Duration::seconds(61), "in about 2 minutes"),
            (Duration::minutes(59), "in about 59 minutes"),
            // The row that failed: one second short of an hour is not 59 minutes away.
            (Duration::seconds(59 * 60 + 59), "in about 1 hour"),
            (Duration::minutes(60), "in about 1 hour"),
            (Duration::minutes(61), "in about 2 hours"),
            (Duration::hours(22), "in about 22 hours"),
            (Duration::seconds(22 * 3600 + 1), "in about 23 hours"),
        ] {
            assert_eq!(in_about(left), expected, "{left:?}");
        }
    }

    #[test]
    fn a_clock_that_has_already_passed_is_not_a_negative_wait() {
        // The counter can turn over between the refusal being decided and this being written.
        // "in about -1 minutes" is the arithmetic showing through.
        for left in [Duration::seconds(-1), Duration::hours(-3)] {
            assert_eq!(in_about(left), "in under a minute", "{left:?}");
        }
    }
}
