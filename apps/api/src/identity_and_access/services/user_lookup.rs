//! Loading the account row behind a Discord id.
//!
//! Every surface that needs the caller's stored tier, ban state or Arma link reads it
//! through here, so the soft-delete filter and the `numeric → f64` cast are stated once.

use crate::command_center::services::user_stats::ATTENDANCE_RATE_SQL;
use sqlx::PgPool;

use crate::identity_and_access::models::user_account::User;

/// Load a live user by Discord id (applies the soft-delete filter — `users` is one of the
/// 4 soft-deletable tables). Returns `None` if absent or deleted.
/// Attendance uses current facts in the same snapshot, including clock-driven schedule boundaries.
pub async fn load_user(pool: &PgPool, discord_id: &str) -> sqlx::Result<Option<User>> {
    let mut query = sqlx::QueryBuilder::<sqlx::Postgres>::new(
        "SELECT discord_id, username, COALESCE(discord_handle, '') AS discord_handle,
         COALESCE(avatar_url, '') AS avatar_url, arma_id, COALESCE(arma_character, '') AS arma_character,
         role, is_banned, COALESCE(ban_reason, '') AS ban_reason, banned_by, banned_at, total_deployments, (");
    query.push(ATTENDANCE_RATE_SQL).push(
        ")::float8 AS attendance_rate, last_login_at,
         COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at,
         COALESCE(updated_at, '0001-01-01 00:00:00+00'::timestamptz) AS updated_at
         FROM users WHERE discord_id = $1 AND deleted_at IS NULL",
    );
    query
        .build_query_as::<User>()
        .bind(discord_id)
        .fetch_optional(pool)
        .await
}
