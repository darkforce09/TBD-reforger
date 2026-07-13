//! Faction library CRUD (T-153) — operator-authored reusable factions for the Mission
//! Creator palette. Owner-scoped (mission_maker+): every route reads/writes only the
//! caller's rows. The full faction document lives in `doc` jsonb and is validated
//! against faction-library.schema.json on every write; `side`/`name` are projected out
//! of the validated doc (single source of truth) for listing + the (owner, name)
//! uniqueness rule.
//!
//! @contract faction-library.schema.json#/

use axum::extract::rejection::JsonRejection;
use axum::extract::{Json, Path, State};
use axum::http::StatusCode;
use serde_json::Value;
use sqlx::types::Json as SqlxJson;
use uuid::Uuid;

use crate::contract::validate_faction_library_doc;
use crate::error::ApiError;
use crate::middleware::MissionMakerUser;
use crate::models::UserFaction;
use crate::state::AppState;


/// Validate the raw doc and project (side, name) out of it.
fn validated_side_name(doc: &Value) -> Result<(String, String), ApiError> {
    let raw = serde_json::to_vec(doc).map_err(|_| ApiError::bad_request("doc is not valid JSON"))?;
    let details = validate_faction_library_doc(&raw)
        .map_err(|e| ApiError::internal(format!("faction schema compile failed: {e}")))?;
    if !details.is_empty() {
        return Err(ApiError::bad_request(format!(
            "doc violates faction-library.schema.json: {}",
            details.join("; ")
        )));
    }
    // Safe unwraps: the schema just guaranteed both required string fields.
    let side = doc["side"].as_str().unwrap_or_default().to_string();
    let name = doc["name"].as_str().unwrap_or_default().to_string();
    Ok((side, name))
}

/// `GET /api/v1/factions` — the caller's faction library, side then name order.
///
/// @route GET /api/v1/factions
pub async fn list_factions(
    State(state): State<AppState>,
    maker: MissionMakerUser,
) -> Result<Json<Value>, ApiError> {
    let rows: Vec<UserFaction> = sqlx::query_as(
        "SELECT id, owner_id, side, name, doc, created_at, updated_at FROM user_factions \
         WHERE owner_id = $1 ORDER BY side ASC, name ASC",
    )
    .bind(&maker.0.discord_id)
    .fetch_all(&state.pool)
    .await?;
    let total = rows.len();
    Ok(Json(serde_json::json!({
        "data": rows, "total": total, "limit": total, "offset": 0
    })))
}

/// `GET /api/v1/factions/:id` — one owned faction.
///
/// @route GET /api/v1/factions/:id
pub async fn get_faction(
    State(state): State<AppState>,
    maker: MissionMakerUser,
    Path(id): Path<Uuid>,
) -> Result<Json<UserFaction>, ApiError> {
    let row: Option<UserFaction> = sqlx::query_as(
        "SELECT id, owner_id, side, name, doc, created_at, updated_at FROM user_factions \
         WHERE id = $1 AND owner_id = $2",
    )
            .bind(id)
            .bind(&maker.0.discord_id)
            .fetch_optional(&state.pool)
            .await?;
    row.map(Json).ok_or_else(|| ApiError::not_found("faction not found"))
}

/// `POST /api/v1/factions` — create a faction from a full faction-library doc.
///
/// @route POST /api/v1/factions
pub async fn create_faction(
    State(state): State<AppState>,
    maker: MissionMakerUser,
    body: Result<Json<Value>, JsonRejection>,
) -> Result<(StatusCode, Json<UserFaction>), ApiError> {
    let Json(doc) = body.map_err(|_| ApiError::bad_request("a faction-library doc is required"))?;
    let (side, name) = validated_side_name(&doc)?;

    let row: Option<UserFaction> = sqlx::query_as(
        "INSERT INTO user_factions (owner_id, side, name, doc) VALUES ($1, $2, $3, $4) \
         ON CONFLICT (owner_id, name) DO NOTHING \
         RETURNING id, owner_id, side, name, doc, created_at, updated_at",
    )
    .bind(&maker.0.discord_id)
    .bind(&side)
    .bind(&name)
    .bind(SqlxJson(&doc))
    .fetch_optional(&state.pool)
    .await?;
    let row = row.ok_or_else(|| ApiError::conflict("a faction with this name already exists"))?;
    Ok((StatusCode::CREATED, Json(row)))
}

/// `PUT /api/v1/factions/:id` — replace an owned faction's doc.
///
/// @route PUT /api/v1/factions/:id
pub async fn update_faction(
    State(state): State<AppState>,
    maker: MissionMakerUser,
    Path(id): Path<Uuid>,
    body: Result<Json<Value>, JsonRejection>,
) -> Result<Json<UserFaction>, ApiError> {
    let Json(doc) = body.map_err(|_| ApiError::bad_request("a faction-library doc is required"))?;
    let (side, name) = validated_side_name(&doc)?;

    // Name collision with a DIFFERENT owned row → 409 (the unique index would abort).
    let clash: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM user_factions WHERE owner_id = $1 AND name = $2 AND id <> $3",
    )
    .bind(&maker.0.discord_id)
    .bind(&name)
    .bind(id)
    .fetch_optional(&state.pool)
    .await?;
    if clash.is_some() {
        return Err(ApiError::conflict("a faction with this name already exists"));
    }

    let row: Option<UserFaction> = sqlx::query_as(
        "UPDATE user_factions SET side = $1, name = $2, doc = $3, updated_at = now() \
         WHERE id = $4 AND owner_id = $5 \
         RETURNING id, owner_id, side, name, doc, created_at, updated_at",
    )
    .bind(&side)
    .bind(&name)
    .bind(SqlxJson(&doc))
    .bind(id)
    .bind(&maker.0.discord_id)
    .fetch_optional(&state.pool)
    .await?;
    row.map(Json).ok_or_else(|| ApiError::not_found("faction not found"))
}

/// `DELETE /api/v1/factions/:id` — delete an owned faction.
///
/// @route DELETE /api/v1/factions/:id
pub async fn delete_faction(
    State(state): State<AppState>,
    maker: MissionMakerUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let res = sqlx::query("DELETE FROM user_factions WHERE id = $1 AND owner_id = $2")
        .bind(id)
        .bind(&maker.0.discord_id)
        .execute(&state.pool)
        .await?;
    if res.rows_affected() == 0 {
        return Err(ApiError::not_found("faction not found"));
    }
    Ok(StatusCode::NO_CONTENT)
}
