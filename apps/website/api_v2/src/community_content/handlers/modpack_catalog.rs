//! The member-facing modpack catalog: every pack with its mods, and the active manifest.

use axum::extract::State;
use axum::response::Json;
use serde_json::{Value, json};

use crate::community_content::models::modpack::Modpack;
use crate::community_content::services::modpack_lookup::{
    ModpackDto, load_current_modpack, modpack_cols, with_mods,
};
use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::core::middleware::AuthUser;

/// `GET /api/v1/modpacks` — every modpack with its mods (current first).
///
/// @route GET /api/v1/modpacks
pub async fn list_modpacks(
    State(state): State<AppState>,
    _u: AuthUser,
) -> Result<Json<Value>, ApiError> {
    let packs: Vec<Modpack> = sqlx::query_as(concat!(
        "SELECT ",
        modpack_cols!(),
        " FROM modpacks ORDER BY is_current DESC, created_at DESC"
    ))
    .fetch_all(&state.pool)
    .await?;
    let mut out = Vec::with_capacity(packs.len());
    for mp in packs {
        out.push(with_mods(&state.pool, mp).await?);
    }
    Ok(Json(json!({ "data": out })))
}

/// `GET /api/v1/modpacks/current` — the active modpack.
///
/// @route GET /api/v1/modpacks/current
pub async fn get_current_modpack(
    State(state): State<AppState>,
    _u: AuthUser,
) -> Result<Json<ModpackDto>, ApiError> {
    load_current_modpack(&state.pool)
        .await?
        .map(Json)
        .ok_or_else(|| ApiError::not_found("no current modpack configured"))
}
