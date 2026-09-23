//! Game confirmations consume link codes through the shared ownership transaction.
use crate::core::{
    application_state::AppState, error_handling::api_error::ApiError, middleware::ServiceAuth,
};
use crate::identity_and_access::services::identity_linking::confirm_identity;
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

/// Consume the code, attribute history, and recompute statistics before acknowledging.
/// @route POST /api/v1/ingest/link-confirm
pub async fn ingest_link_confirm(
    State(state): State<AppState>,
    _svc: ServiceAuth,
    body: Result<Json<LinkConfirmRequest>, JsonRejection>,
) -> Result<Json<Value>, ApiError> {
    let Json(req) =
        body.map_err(|_| ApiError::bad_request("code, arma_id and arma_character are required"))?;
    let arma_id = req.arma_id.trim();
    let confirmed = confirm_identity(&state, &req.code, arma_id, &req.arma_character).await?;
    Ok(Json(
        json!({"linked": true, "discord_id": confirmed.discord_id,
        "arma_id": confirmed.arma_id, "arma_character": confirmed.arma_character}),
    ))
}

#[cfg(test)]
#[path = "tests/arma_link_confirmation.rs"]
mod tests;
