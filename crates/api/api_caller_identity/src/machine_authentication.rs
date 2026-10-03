//! The machine-credential extractor for game-host and game-runtime routes.
//!
//! A machine presents `Authorization: Bearer tbdm_…`. The extractor verifies the credential and
//! yields the [`MachineCaller`]; each handler then requires its executor kind and checks every
//! resource it touches against the caller's server.
//!
//! **Role:** the axum extractor that authenticates a machine caller.
//! **Position:** taken as a handler argument by every machine route; reads the database pool out
//! of any router state that exposes one through [`FromRef`].
//! **Signals & state:** none of its own; [`authenticate_machine`] stamps the credential's use.
//! **Invariants:** a request without a `Bearer` credential is refused with 401 before any
//! database read; the router state is reached only through `PgPool: FromRef<S>`.

use axum::extract::{FromRef, FromRequestParts};
use axum::http::header;
use axum::http::request::Parts;
use sqlx::PgPool;

use crate::machine_caller::{MachineCaller, authenticate_machine};
use api_foundation::error_handling::api_error::ApiError;

impl<S> FromRequestParts<S> for MachineCaller
where
    PgPool: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, ApiError> {
        let secret = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "))
            .map(str::trim)
            .ok_or_else(|| ApiError::unauthorized("missing machine credential"))?;
        let pool = PgPool::from_ref(state);
        authenticate_machine(&pool, secret).await
    }
}
