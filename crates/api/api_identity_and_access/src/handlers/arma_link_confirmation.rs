//! Game confirmations consume link codes through the shared ownership transaction. A body that
//! does not decode (a missing or unknown field included) answers through
//! [`ApiError::from_json_rejection`].
//!
//! @contract arma-link.schema.json#/definitions/LinkConfirmRequest
//! @contract arma-link.schema.json#/definitions/LinkConfirmation
use crate::services::identity_linking::confirm_identity;
use api_caller_identity::machine_caller::MachineCaller;
use api_foundation::error_handling::api_error::ApiError;
use api_identifiers::ArmaPlayerId;
use api_state::AppState;
use axum::{
    extract::{State, rejection::JsonRejection},
    response::Json,
};
use fleet_wire_contract::ExecutorKind;
use serde::Deserialize;
use serde_json::{Value, json};

/// Engine-provided identity and character accompany the player's single-use web code.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LinkConfirmRequest {
    /// The six-digit code the player typed in the game.
    pub code: String,
    /// The player's engine identity, as the game runtime reports it.
    pub arma_id: ArmaPlayerId,
    /// The player's character name, as the game runtime reports it.
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
    let arma_id = ArmaPlayerId::new(req.arma_id.as_str().trim());
    let confirmed = confirm_identity(
        &state,
        caller.server_id,
        &req.code,
        &arma_id,
        &req.arma_character,
    )
    .await?;
    Ok(Json(
        json!({"linked": true, "discord_id": confirmed.discord_id,
        "arma_id": confirmed.arma_id, "arma_character": confirmed.arma_character}),
    ))
}
