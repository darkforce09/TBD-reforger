//! Operator authentication of the observability surface: `/metrics` and the detailed `/healthz`.
//!
//! **Role:** decides whether a request carries the operator's `OBSERVABILITY_TOKEN` as an
//! `Authorization: Bearer` credential.
//! **Position:** consumed by the API router (`api::router`); `/metrics` takes the
//! [`ObservabilityAuth`] extractor, `/healthz` asks [`observability_bearer_matches`] to choose
//! between the public and the detailed report.
//! **Signals & state:** none; reads [`api_configuration::configuration::Config::observability_token`].
//! **Invariants:** the comparison is constant-time; an unset token matches nothing, so a
//! deployment that never configured one serves neither the scrape nor the detail. No other
//! credential (user session, machine credential) is accepted here, and this token is accepted
//! nowhere else.

use std::sync::Arc;

use axum::extract::{FromRef, FromRequestParts};
use axum::http::request::Parts;
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};

use crate::authentication_primitives::constant_time_equal;
use crate::middleware::json_error;
use api_configuration::configuration::Config;

/// Proof that the request presented the configured observability token.
#[derive(Debug, Clone, Copy)]
pub struct ObservabilityAuth;

impl<S> FromRequestParts<S> for ObservabilityAuth
where
    Arc<Config>: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let cfg = Arc::<Config>::from_ref(state);
        if observability_bearer_matches(&cfg, &parts.headers) {
            Ok(ObservabilityAuth)
        } else {
            Err(json_error(StatusCode::UNAUTHORIZED, "invalid observability token").into_response())
        }
    }
}

/// Whether `headers` carry `Authorization: Bearer <OBSERVABILITY_TOKEN>`, compared in constant
/// time. Never rejects: `/healthz` must answer a credential-less prober, so a missing or wrong
/// token downgrades its payload instead.
pub fn observability_bearer_matches(cfg: &Config, headers: &HeaderMap) -> bool {
    if cfg.observability_token.is_empty() {
        return false;
    }
    let presented = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .unwrap_or("");
    constant_time_equal(presented, &cfg.observability_token)
}

#[cfg(test)]
#[path = "tests/observability_auth.rs"]
mod tests;
