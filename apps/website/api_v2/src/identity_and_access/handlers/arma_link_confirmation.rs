//! Game confirmations consume link codes through the shared ownership transaction. A body that
//! does not decode (a missing or unknown field included) answers through
//! [`ApiError::from_json_rejection`].
//!
//! @contract arma-link.schema.json#/definitions/LinkConfirmRequest
//! @contract arma-link.schema.json#/definitions/LinkConfirmation
use crate::core::{application_state::AppState, error_handling::api_error::ApiError};
use crate::identity_and_access::services::identity_linking::confirm_identity;
use crate::server_infrastructure::models::machine_credential::ExecutorKind;
use crate::server_infrastructure::services::machine_credentials::MachineCaller;
use axum::{
    extract::{State, rejection::JsonRejection},
    response::Json,
};
use serde::Deserialize;
use serde_json::{Value, json};

/// Engine-provided identity and character accompany the player's single-use web code.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LinkConfirmRequest {
    pub code: String,
    pub arma_id: String,
    pub arma_character: String,
}

/// Consume the code, attribute history, and recompute statistics before acknowledging; the game
/// runtime authenticates with its `mod_runtime` machine credential and the audit names its server.
/// @route POST /api/v1/ingest/link-confirm
pub async fn ingest_link_confirm(
    State(state): State<AppState>,
    caller: MachineCaller,
    body: Result<Json<LinkConfirmRequest>, JsonRejection>,
) -> Result<Json<Value>, ApiError> {
    caller.require_executor(ExecutorKind::ModRuntime)?;
    let Json(req) = body.map_err(ApiError::from_json_rejection)?;
    let arma_id = req.arma_id.trim();
    let confirmed = confirm_identity(
        &state,
        caller.server_id,
        &req.code,
        arma_id,
        &req.arma_character,
    )
    .await?;
    Ok(Json(
        json!({"linked": true, "discord_id": confirmed.discord_id,
        "arma_id": confirmed.arma_id, "arma_character": confirmed.arma_character}),
    ))
}

#[cfg(test)]
#[path = "tests/arma_link_confirmation.rs"]
mod tests;
