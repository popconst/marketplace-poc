//! The one error type handlers return, and the JSON body it sends.

use std::collections::BTreeMap;

use axum::Json;
use axum::extract::rejection::{JsonRejection, PathRejection, QueryRejection};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

/// What is wrong with each invalid field, keyed by its name in the request.
pub type FieldErrors = BTreeMap<&'static str, String>;

/// Sent as `{"error": <code>, "message": <text>}`, plus `fields` for `Validation`.
#[derive(Debug)]
pub enum ApiError {
    /// Unreadable request: malformed JSON, a missing field, a value of the wrong type.
    BadRequest(String),
    NotFound(String),
    /// The resource's current state does not allow the request.
    Conflict(String),
    /// Readable, but these fields are invalid.
    Validation(FieldErrors),
    /// Readable, but breaks a rule that is not about one field.
    Unprocessable(String),
    Internal(sqlx::Error),
}

impl ApiError {
    pub fn not_found(resource: &str, id: i64) -> Self {
        Self::NotFound(format!("{resource} {id} does not exist."))
    }
}

#[derive(Serialize)]
struct ErrorBody {
    error: &'static str,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    fields: Option<FieldErrors>,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error, message, fields) = match self {
            Self::BadRequest(message) => (StatusCode::BAD_REQUEST, "bad_request", message, None),
            Self::NotFound(message) => (StatusCode::NOT_FOUND, "not_found", message, None),
            Self::Conflict(message) => (StatusCode::CONFLICT, "conflict", message, None),
            Self::Validation(fields) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "validation",
                "Some fields are invalid.".to_owned(),
                Some(fields),
            ),
            Self::Unprocessable(message) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "validation",
                message,
                None,
            ),
            Self::Internal(error) => {
                tracing::error!(%error, "request failed");
                let message = "Something went wrong on our side.".to_owned();
                (StatusCode::INTERNAL_SERVER_ERROR, "internal", message, None)
            }
        };
        let body = ErrorBody {
            error,
            message,
            fields,
        };
        (status, Json(body)).into_response()
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(error: sqlx::Error) -> Self {
        // Handlers validate first, so a check or foreign key that still fails lost a race with
        // another request: a 422, not a 500.
        if let Some(database_error) = error.as_database_error()
            && (database_error.is_check_violation() || database_error.is_foreign_key_violation())
        {
            let constraint = database_error.constraint().unwrap_or("unknown");
            return Self::Unprocessable(format!(
                "The database rejected the request ({constraint})."
            ));
        }
        Self::Internal(error)
    }
}

// The extractors in `crate::extract` turn their rejections into these.

impl From<JsonRejection> for ApiError {
    fn from(rejection: JsonRejection) -> Self {
        Self::BadRequest(rejection.body_text())
    }
}

impl From<PathRejection> for ApiError {
    fn from(rejection: PathRejection) -> Self {
        Self::BadRequest(rejection.body_text())
    }
}

impl From<QueryRejection> for ApiError {
    fn from(rejection: QueryRejection) -> Self {
        Self::BadRequest(rejection.body_text())
    }
}
