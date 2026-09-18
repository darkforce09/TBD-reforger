//! Loading the account row behind a Discord id.
//!
//! Every surface that needs the caller's stored tier, ban state or Arma link reads it
//! through here, so the soft-delete filter and the `numeric → f64` cast are stated once.

use sqlx::PgPool;

use crate::identity_and_access::models::user_account::User;

/// Load a live user by Discord id (applies the soft-delete filter — `users` is one of the
/// 4 soft-deletable tables). Returns `None` if absent or deleted. The
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
