//! The machine-credential extractor for game-host and game-runtime routes.
//!
//! A machine presents `Authorization: Bearer tbdm_…`. The extractor verifies the credential and
//! yields the [`MachineCaller`]; each handler then requires its executor kind and checks every
//! resource it touches against the caller's server.

use axum::extract::FromRequestParts;
use axum::http::header;
use axum::http::request::Parts;

use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::server_infrastructure::services::machine_credentials::{
    MachineCaller, authenticate_machine,
};

impl FromRequestParts<AppState> for MachineCaller {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, ApiError> {
        let secret = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "))
            .map(str::trim)
            .ok_or_else(|| ApiError::unauthorized("missing machine credential"))?;
        authenticate_machine(&state.pool, secret).await
    }
}
