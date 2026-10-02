//! The path-parameter extractor every route takes: axum's `Path` with its rejection answered in
//! the `{error, details?}` envelope.
//!
//! **Role:** [`PathParams`] decodes a route's path segments exactly as [`axum::extract::Path`]
//! does and answers a refused segment through [`ApiError::from_path_rejection`], so no route
//! answers axum's plain-text rejection body.
//!
//! **Position:** `core`; every handler that reads a path segment takes it in place of axum's
//! `Path`, in the same argument position, so the extraction order of a handler is unchanged.
//!
//! **Signals & state:** none; a per-request extractor over the matched route's parameters.
//!
//! **Invariants:** a segment that does not decode into the target type (a non-UUID id on a
//! `PathParams<Uuid>`, a segment that is not UTF-8) answers `400` in the envelope with a message
//! naming the parameter; an extractor that does not match its route's parameters answers a logged
//! `500 internal error`, never a `400` that blames the caller.

use axum::extract::{FromRequestParts, Path};
use axum::http::request::Parts;
use serde::de::DeserializeOwned;

use crate::core::error_handling::api_error::ApiError;

/// A route's decoded path parameters; the rejection answers in the error envelope.
///
/// `T` is anything [`axum::extract::Path`] decodes: one value for a single parameter, a tuple in
/// route order for several, or a struct keyed by parameter name.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PathParams<T>(pub T);

impl<T, S> FromRequestParts<S> for PathParams<T>
where
    T: DeserializeOwned + Send,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        match Path::<T>::from_request_parts(parts, state).await {
            Ok(Path(value)) => Ok(Self(value)),
            Err(rejection) => Err(ApiError::from_path_rejection(rejection)),
        }
    }
}

#[cfg(test)]
#[path = "tests/path_parameters.rs"]
mod tests;
