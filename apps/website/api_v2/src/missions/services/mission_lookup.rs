//! Mission row reads shared by every surface that resolves a mission id.
//!
//! Three shapes, one owner each: the full row by id, the same read wrapped into the HTTP 404 the
//! mission routes answer, and the (title, terrain) projection the dashboard and deployment
//! surfaces enrich event rows with.

use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use crate::core::error_handling::api_error::ApiError;
use crate::missions::models::mission::{Mission, TerrainType};

const MISSION_ROW_SQL: &str = "SELECT id, title, author_id, terrain, COALESCE(custom_terrain_name, '') AS custom_terrain_name, \
         game_mode, weather, time_of_day::text AS time_of_day, max_players, status, \
         COALESCE(thumbnail_url, '') AS thumbnail_url, COALESCE(briefing, '') AS briefing, \
         current_version_id, approved_artifact_id, COALESCE(rejection_reason, '') AS rejection_reason, \
         reviewed_by, reviewed_at, COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at, \
         COALESCE(updated_at, '0001-01-01 00:00:00+00'::timestamptz) AS updated_at \
         FROM missions WHERE id = $1 AND deleted_at IS NULL";

/// Load a live mission by id (soft-delete filtered; `time_of_day::text` cast for the
/// `time without time zone` column). Returns `None` if absent or deleted.
pub async fn load_mission(pool: &PgPool, id: Uuid) -> sqlx::Result<Option<Mission>> {
    sqlx::query_as::<_, Mission>(MISSION_ROW_SQL)
        .bind(id)
        .fetch_optional(pool)
        .await
}

/// [`load_mission`] on a transaction that already holds the mission's lock.
pub async fn load_mission_on(
    connection: &mut PgConnection,
    id: Uuid,
) -> sqlx::Result<Option<Mission>> {
    sqlx::query_as::<_, Mission>(MISSION_ROW_SQL)
        .bind(id)
        .fetch_optional(connection)
        .await
}

/// Parse `:id` and load the mission (404 on bad id or missing).
pub(crate) async fn load_mission_or_404(pool: &PgPool, id: &str) -> Result<Mission, ApiError> {
    let Ok(id) = Uuid::parse_str(id) else {
        return Err(ApiError::bad_request("invalid id"));
    };
    load_mission(pool, id)
        .await?
        .ok_or_else(|| ApiError::not_found("mission not found"))
}

/// Fetch a mission's (title, terrain) for enrichment (avoids the full-row time cast).
///
/// **Errors must not look like success.** A missing mission is `Ok(None)`. A decode or SQL
/// failure is `Err` and propagates to the caller — never collapse a failed query into a silent
/// empty title/terrain via `Option`, because that makes a broken query indistinguishable from
/// "no mission".
pub(crate) async fn mission_title_terrain(
    pool: &sqlx::PgPool,
    id: Uuid,
) -> Result<Option<(String, TerrainType)>, sqlx::Error> {
    sqlx::query_as("SELECT title, terrain FROM missions WHERE id = $1 AND deleted_at IS NULL")
        .bind(id)
        .fetch_optional(pool)
        .await
}

/// Historical participation retains its mission label after operational catalog removal.
pub(crate) async fn historical_mission_title_terrain(
    pool: &PgPool,
    id: Uuid,
) -> sqlx::Result<Option<(String, TerrainType)>> {
    sqlx::query_as("SELECT title, terrain FROM missions WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
}
