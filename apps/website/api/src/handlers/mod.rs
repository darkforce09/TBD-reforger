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
pub mod factions;
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

/// True if a sqlx error is a Postgres foreign-key violation (SQLSTATE 23503).
///
/// **T-576 — this sibling did not exist until now, and that absence had a cost.** T-262's `0018`
/// landed the schema's first 25 foreign keys; with no 23503 arm anywhere in the crate every one
/// of them reached the client through [`crate::error`]'s blanket `From<sqlx::Error>` as
/// `500 {"error":"internal error"}`. Reproduced over HTTP before the fix:
/// `POST /api/v1/ingest/server-status` with an unregistered `server_id` → **500**, the same
/// heartbeat for a registered one → **200**, log line
/// `violates foreign key constraint "server_statuses_server_id_fkey"`.
///
/// **In this schema 23503 means exactly one thing: the row being written names a parent that
/// does not exist.** The other direction — deleting a parent that children still reference —
/// does *not* arrive as 23503, which is what makes a 4xx that blames the request body safe here.
/// Measured on the migrated schema rather than assumed:
/// - `DELETE FROM modpacks` under `0018`'s `servers_required_modpack_id_fkey` raises **23001**
///   (`restrict_violation`), not 23503.
/// - `pg_constraint` over all 25 FKs: `confdeltype` = 17 `c` (CASCADE), 3 `n` (SET NULL),
///   5 `r` (RESTRICT), and **zero `a` (NO ACTION)** — NO ACTION on delete is the only delete rule
///   that raises 23503 from the parent side, and the schema has none.
/// - `confupdtype` is `a` for all 25, so a parent *key update* could raise 23503 from the parent
///   side. No handler updates one: every parent key is a `gen_random_uuid()` surrogate or
///   `users.discord_id`, a snowflake the platform does not mint (`0018:187-190` says so, and the
///   only `SET discord_id` writes in the crate are on the child `match_player_stats`).
///
/// Pair with [`violated_constraint`] rather than using this alone. A bare "any 23503 → 4xx" arm
/// would answer for constraints its message cannot possibly describe — the caller cannot name a
/// missing parent it did not identify, and a 400 that names the wrong one is worse than the 500
/// it replaced.
pub fn is_foreign_key_violation(e: &sqlx::Error) -> bool {
    e.as_database_error().and_then(|d| d.code()).as_deref() == Some("23503")
}

/// The name of the constraint a database error blames, when it blames one.
///
/// Reads Postgres' structured `CONSTRAINT NAME` error field, **not** the message text. The text
/// is localized by `lc_messages` and differs between the two 23503 shapes; the field is neither.
/// Measured: an unregistered-server heartbeat reports `CONSTRAINT NAME:
/// server_statuses_server_id_fkey` with `TABLE NAME: server_statuses` — the *child*, so `table()`
/// cannot tell a caller which parent is missing and the constraint name is the identifier to
/// branch on.
pub fn violated_constraint(e: &sqlx::Error) -> Option<&str> {
    e.as_database_error().and_then(|d| d.constraint())
}

/// Load a live user by Discord id (applies the soft-delete filter — one of the 4
/// soft-deletable tables). Returns `None` if absent or deleted. The
/// `attendance_rate::float8` cast decodes the `numeric` column into the model's `f64`.
pub async fn load_user(pool: &PgPool, discord_id: &str) -> sqlx::Result<Option<User>> {
    sqlx::query_as::<_, User>(
        "SELECT discord_id, username, COALESCE(discord_handle, '') AS discord_handle, \
         COALESCE(avatar_url, '') AS avatar_url, arma_id, COALESCE(arma_character, '') AS arma_character, \
         role, is_banned, COALESCE(ban_reason, '') AS ban_reason, banned_by, banned_at, total_deployments, \
         attendance_rate::float8 AS attendance_rate, last_login_at, \
         COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at, \
         COALESCE(updated_at, '0001-01-01 00:00:00+00'::timestamptz) AS updated_at \
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

/// Resolve a display name for audit messages, falling back to the id (mirrors Go
/// `h.username`). COALESCE tolerates a NULL username like GORM's `First` — defensively
/// only, since `migrations/0001_initial_schema.sql:514` declares `username text NOT NULL`
/// (writing NULL fails with SQLSTATE 23502).
///
/// **T-366 — the guard below is `trim().is_empty()`, not `is_empty()`.** Every audit line in
/// the crate takes its `actor_name` from here (14 call sites across `admin.rs`, `approvals.rs`,
/// `cms.rs`, `field_tools.rs`, `me.rs`), so an untrimmed guard let a whitespace username
/// *bypass the `discord_id` fallback that exists to prevent exactly this*. Measured pre-fix on
/// `PATCH /admin/users/:id` with `username = '   '`: `audit_logs.actor_name` = `'   '` (length 3)
/// and `message` = `"    set     role to admin"` — an audit line naming neither actor nor
/// target. `user.ban`, `user.unban` and `user.warn` all produced the same. That is worse than a
/// missing entry because it still looks like a record. `username = ''` already fell through
/// correctly, which is what made the whitespace case a gap rather than a design choice.
///
/// **Why trimming is safe *here* when T-326/T-343 showed it usually is not.** The rule those
/// tickets established is that a trim on read must agree with the trim on write. Checked, not
/// assumed:
/// - **No writer trims, so there is no counterpart to disagree with.** `users.username` has
///   exactly two writers — `handlers/oauth.rs:117` binds `du.display_name()` (Discord's
///   `global_name`, else `username`) with no trim and no guard at any hop, and `handlers/dev.rs`
///   binds the literal `'Dev Operator'`. No CHECK constraint, no trigger, no `btrim` in SQL, and
///   no request body anywhere in the crate carries a `username` field. `display_name()`
///   (`services/discord.rs:113`) selects on `global_name.is_empty()`, so a Discord
///   `global_name` of `"   "` wins that branch and is stored verbatim — this is the live path
///   by which a blank-ish username actually arrives.
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
