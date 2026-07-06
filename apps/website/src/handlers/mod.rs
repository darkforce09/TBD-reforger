//! HTTP handlers — Rust port of `internal/handlers`, grouped by domain. Populated
//! per phase; the `/api/v1` route tree is assembled in [`crate::app`].

pub mod admin;
pub mod announcements;
pub mod approvals;
pub mod audit;
pub mod auth;
pub mod cms;
pub mod dashboard;
pub mod deployments;
pub mod dev;
pub mod events;
pub mod field_tools;
pub mod leaderboards;
pub mod me;
pub mod missions;
pub mod modpacks;
pub mod oauth;
pub mod registry;
pub mod servers;
pub mod telemetry;
pub mod wiki;

use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{Mission, User};

/// Offset-pagination query params shared by list endpoints.
#[derive(Debug, Deserialize)]
pub struct PageParams {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

impl PageParams {
    /// `(limit, offset)` clamped like Go `parsePage`: limit default 20 / max 100,
    /// offset default 0.
    pub fn bounds(&self) -> (i64, i64) {
        let limit = self.limit.filter(|&n| n > 0 && n <= 100).unwrap_or(20);
        let offset = self.offset.filter(|&n| n >= 0).unwrap_or(0);
        (limit, offset)
    }
}

/// True if a sqlx error is a Postgres unique-violation (SQLSTATE 23505). Mirrors the
/// Go `isUniqueViolation` used for semver-conflict 409s and link-code collisions.
pub fn is_unique_violation(e: &sqlx::Error) -> bool {
    e.as_database_error().and_then(|d| d.code()).as_deref() == Some("23505")
}

/// Load a live user by Discord id (applies the soft-delete filter — one of the 4
/// soft-deletable tables). Returns `None` if absent or deleted. The
/// `attendance_rate::float8` cast decodes the `numeric` column into the model's `f64`.
pub async fn load_user(pool: &PgPool, discord_id: &str) -> sqlx::Result<Option<User>> {
    sqlx::query_as::<_, User>(
        "SELECT discord_id, username, discord_handle, avatar_url, arma_id, arma_character, \
         role, is_banned, ban_reason, banned_by, banned_at, total_deployments, \
         attendance_rate::float8 AS attendance_rate, last_login_at, created_at, updated_at \
         FROM users WHERE discord_id = $1 AND deleted_at IS NULL",
    )
    .bind(discord_id)
    .fetch_optional(pool)
    .await
}

/// Load a live mission by id (soft-delete filtered; `time_of_day::text` cast for the
/// `time without time zone` column). Returns `None` if absent or deleted.
pub async fn load_mission(pool: &PgPool, id: Uuid) -> sqlx::Result<Option<Mission>> {
    sqlx::query_as::<_, Mission>(
        "SELECT id, title, author_id, terrain, custom_terrain_name, game_mode, weather, \
         time_of_day::text AS time_of_day, max_players, status, thumbnail_url, briefing, \
         current_version_id, rejection_reason, reviewed_by, reviewed_at, created_at, updated_at \
         FROM missions WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

/// Resolve a display name for audit messages, falling back to the id (mirrors Go
/// `h.username`). COALESCE tolerates a NULL username like GORM's `First`.
pub async fn username(pool: &PgPool, discord_id: &str) -> String {
    let name: Option<String> =
        sqlx::query_scalar("SELECT COALESCE(username, '') FROM users WHERE discord_id = $1")
            .bind(discord_id)
            .fetch_optional(pool)
            .await
            .ok()
            .flatten();
    match name {
        Some(n) if !n.is_empty() => n,
        _ => discord_id.to_string(),
    }
}
