//! HTTP handlers grouped by domain; the `/api/v1` route tree is assembled in
//! [`crate::core::http_router`].
//!
//! Where a domain directory carries a file of its own name (`telemetry/telemetry.rs`, …) the
//! domain's `mod.rs` glob re-exports it, and the `pub use` façade below restores every other
//! module at a flat path, so `handlers::leaderboards::get_leaderboards` and friends resolve from
//! one place. The mission row loader below stays here: it is the domain-neutral floor the
//! remaining domains sit on.

pub mod events;
pub mod missions;
pub mod telemetry;

pub use self::missions::{approvals, registry};
pub use self::telemetry::{dashboard, deployments, field_tools, leaderboards};

use sqlx::PgPool;
use uuid::Uuid;

use crate::missions::models::mission::Mission;

/// Load a live mission by id (soft-delete filtered; `time_of_day::text` cast for the
/// `time without time zone` column). Returns `None` if absent or deleted.
pub async fn load_mission(pool: &PgPool, id: Uuid) -> sqlx::Result<Option<Mission>> {
    sqlx::query_as::<_, Mission>(
        "SELECT id, title, author_id, terrain, COALESCE(custom_terrain_name, '') AS custom_terrain_name, \
         game_mode, weather, time_of_day::text AS time_of_day, max_players, status, \
         COALESCE(thumbnail_url, '') AS thumbnail_url, COALESCE(briefing, '') AS briefing, \
         current_version_id, COALESCE(rejection_reason, '') AS rejection_reason, reviewed_by, reviewed_at, \
         COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at, \
         COALESCE(updated_at, '0001-01-01 00:00:00+00'::timestamptz) AS updated_at \
         FROM missions WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}
