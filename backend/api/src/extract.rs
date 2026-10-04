//! Axum's extractors with an [`ApiError`] rejection, so that unreadable input gets the usual JSON
//! error body instead of axum's plain text. Handlers use these, not axum's.

use axum::extract::{FromRequest, FromRequestParts, OptionalFromRequest, Request};
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::error::ApiError;

/// A JSON request body. Handlers also respond with it.
#[derive(FromRequest)]
#[from_request(via(axum::Json), rejection(ApiError))]
pub struct Json<T>(pub T);

#[derive(FromRequestParts)]
#[from_request(via(axum::extract::Path), rejection(ApiError))]
pub struct Path<T>(pub T);

#[derive(FromRequestParts)]
#[from_request(via(axum::extract::Query), rejection(ApiError))]
pub struct Query<T>(pub T);

impl<T: Serialize> IntoResponse for Json<T> {
    fn into_response(self) -> Response {
        axum::Json(self.0).into_response()
    }
}

/// Allows `Option<Json<T>>`, which is `None` when the request has no `Content-Type` header.
impl<T, S> OptionalFromRequest<S> for Json<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request(request: Request, state: &S) -> Result<Option<Self>, ApiError> {
        let body = <axum::Json<T> as OptionalFromRequest<S>>::from_request(request, state).await?;
        Ok(body.map(|axum::Json(value)| Self(value)))
    }
}
