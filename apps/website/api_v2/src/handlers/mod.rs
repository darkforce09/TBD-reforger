//! HTTP handlers grouped by domain; the `/api/v1` route tree is assembled in
//! [`crate::core::http_router`].
//!
//! Where a domain directory carries a file of its own name (`admin/admin.rs`, …) the
//! domain's `mod.rs` glob re-exports it, and the `pub use` façade below restores every other
//! module at a flat path, so `handlers::servers::list_servers` and friends resolve from one
//! place. The row loaders below stay here: they are the domain-neutral floor the remaining
//! domains sit on.

pub mod admin;
pub mod content;
pub mod events;
pub mod missions;
pub mod telemetry;

pub use self::admin::audit;
pub use self::content::{announcements, cms, modpacks, wiki};
pub use self::events::factions;
pub use self::missions::{approvals, registry};
pub use self::telemetry::{dashboard, deployments, field_tools, leaderboards, servers};

use sqlx::PgPool;
use uuid::Uuid;

use crate::models::Mission;

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

/// Resolve a display name for audit messages, falling back to the id. The COALESCE tolerates
/// a NULL username defensively only, since `migrations/0001_initial_schema.sql:514` declares
/// `username text NOT NULL` (writing NULL fails with SQLSTATE 23502).
///
/// **The guard below is `trim().is_empty()`, not `is_empty()`.** Every audit line in the crate
/// takes its `actor_name` from here (14 call sites across `admin.rs`, `approvals.rs`,
/// `cms.rs`, `field_tools.rs`, `arma_link_codes.rs`), so an untrimmed guard lets a whitespace
/// username *bypass the `discord_id` fallback that exists to prevent exactly this*. Measured on
/// `PATCH /admin/users/:id` with `username = '   '`: `audit_logs.actor_name` = `'   '` (length 3)
/// and `message` = `"    set     role to admin"` — an audit line naming neither actor nor
/// target. `user.ban`, `user.unban` and `user.warn` all produced the same. That is worse than a
/// missing entry because it still looks like a record. `username = ''` already fell through
/// correctly, which is what made the whitespace case a gap rather than a design choice.
///
/// **Why trimming is safe *here* when it usually is not.** The rule is that a trim on read
/// must agree with the trim on write. Checked, not assumed:
/// - **No writer trims, so there is no counterpart to disagree with.** `users.username` has
///   exactly two writers — `identity_and_access/handlers/discord_oauth.rs` binds
///   `du.display_name()` (Discord's `global_name`, else `username`) with no trim and no guard
///   at any hop, and `identity_and_access/handlers/developer_login.rs` binds the literal
///   `'Dev Operator'`. No CHECK constraint, no trigger, no `btrim` in SQL, and no request body
///   anywhere in the crate carries a `username` field. `display_name()`
///   (`identity_and_access/services/discord_user_profile.rs`) selects on whether `global_name`
///   is blank, so a Discord `global_name` of `"   "` reaches `users.username` verbatim when
///   that selection is not trim-aware — this is the live path by which a blank-ish username
///   arrives.
/// - **This value is never a key.** All 14 consumers pass it to `services::write_audit`'s
///   `actor_name` display column or interpolate it into `message`. `write_audit`'s `actor_id`
///   is bound separately from the caller's real `discord_id`, so audit-row identity never comes
///   from this string. It is used in no `WHERE`, comparison, join or `ORDER BY`. Changing the
///   guard therefore cannot change which row anything matches — the failure mode that made a
///   one-sided trim catastrophic for `faction` at `events.rs:1735`/`:1923` does not exist here.
///
/// **Fall-through vs display-trimmed — deliberately only the former.** The guard treats
/// blank-ish as absent (`'   '` → `discord_id`), but a name that survives the guard is returned
/// **exactly as stored**, so `'  Sam  '` renders as `'  Sam  '` and *never* degrades into a
/// discord_id. Returning `n.trim()` was rejected, not overlooked: `admin.rs:342-344`
/// hand-rolls this same `SELECT COALESCE(username, '')` for `target_name` and does not trim, so
/// trimming the return value would make one audit message render its actor trimmed and its
/// target padded — a fresh two-site disagreement, in a file this slice does not own. Padding is
/// cosmetic; namelessness is not. Trimming the display is a presentation change that should land
/// together with `admin.rs:342` (which also wants this helper's missing `discord_id` fallback —
/// pre-existing: with `username = ''`, `user.warn` logs `"<id> warned '': …"`).
pub async fn username(pool: &PgPool, discord_id: &str) -> String {
    let name: Option<String> =
        sqlx::query_scalar("SELECT COALESCE(username, '') FROM users WHERE discord_id = $1")
            .bind(discord_id)
            .fetch_optional(pool)
            .await
            .ok()
            .flatten();
    match name {
        Some(n) if !n.trim().is_empty() => n,
        _ => discord_id.to_string(),
    }
}
