//! Discord role → web role sync.
//!
//! `sync_roles` runs in-request on OAuth login; `resync_all_roles` is what admin
//! `POST /admin/roles/sync` calls. The scheduled safety net that calls `resync_all_roles`
//! on a timer lives in [`crate::background_workers::discord_role_synchronizer`].

use sqlx::PgPool;

use crate::models::UserRole;

/// Reconcile a user's Discord role snowflakes into `user_discord_roles` (unmapped
/// ids are still stored so a later admin mapping + resync promotes them), then
/// resolve their web role. Defaults to enlisted when nothing maps.
pub async fn sync_roles(
    pool: &PgPool,
    discord_id: &str,
    role_ids: &[String],
) -> sqlx::Result<UserRole> {
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM user_discord_roles WHERE discord_id = $1")
        .bind(discord_id)
        .execute(&mut *tx)
        .await?;
    for rid in role_ids {
        sqlx::query(
            "INSERT INTO user_discord_roles (discord_id, discord_role_id, synced_at) \
             VALUES ($1, $2, now()) ON CONFLICT DO NOTHING",
        )
        .bind(discord_id)
        .bind(rid)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    resolve_role(pool, role_ids).await
}

/// Re-resolve every user's web role from their stored Discord roles against current
/// mappings (used after an admin remaps a role). Returns the number changed.
///
/// **T-372 — empty stored snowflakes are not "enlisted".** A user with zero
/// `user_discord_roles` rows has **no snapshot** (never OAuth'd, or promoted via
/// PATCH / admin seed) — the same statement as [`RoleSnapshot::Unavailable`] in
/// `handlers/oauth.rs` (T-185), not `Authoritative([])`. Skipping leaves their
/// web role alone. Only a non-empty stored list is an authoritative input for
/// [`resolve_role`].
pub async fn resync_all_roles(pool: &PgPool) -> sqlx::Result<i64> {
    let users: Vec<(String, UserRole)> =
        sqlx::query_as("SELECT discord_id, role FROM users WHERE deleted_at IS NULL")
            .fetch_all(pool)
            .await?;
    let mut updated = 0i64;
    for (discord_id, current) in users {
        let role_ids: Vec<String> = sqlx::query_scalar(
            "SELECT discord_role_id FROM user_discord_roles WHERE discord_id = $1",
        )
        .bind(&discord_id)
        .fetch_all(pool)
        .await?;
        // No snapshot ⇒ skip. Do not call resolve_role(empty) — that returns
        // Enlisted and is the admin-lockout path this ticket closes.
        let Some(ids) = resync_ids_from_snapshot(&role_ids) else {
            continue;
        };
        let role = resolve_role(pool, ids).await?;
        if role != current {
            sqlx::query("UPDATE users SET role = $1, updated_at = now() WHERE discord_id = $2")
                .bind(role)
                .bind(&discord_id)
                .execute(pool)
                .await?;
            updated += 1;
        }
    }
    Ok(updated)
}

/// Stored Discord snowflakes as a resync input — mirrors T-185 `RoleSnapshot`.
///
/// - `None` = **no snapshot** (empty table for this user). Must not demote.
/// - `Some(ids)` = authoritative stored list; may resolve to enlisted when nothing
///   maps (real OAuth answer that wrote zero mapped roles, or remap cleared them).
///
/// Do not `unwrap_or_default()` the `None` — that is exactly the T-372 bug.
fn resync_ids_from_snapshot(role_ids: &[String]) -> Option<&[String]> {
    if role_ids.is_empty() {
        None
    } else {
        Some(role_ids)
    }
}

/// The highest-priority mapped web role among the given Discord role ids, or
/// enlisted if none are mapped.
pub async fn resolve_role(pool: &PgPool, role_ids: &[String]) -> sqlx::Result<UserRole> {
    if role_ids.is_empty() {
        return Ok(UserRole::Enlisted);
    }
    let mapped: Option<Option<UserRole>> = sqlx::query_scalar(
        "SELECT mapped_role FROM discord_roles \
         WHERE discord_role_id = ANY($1) AND mapped_role IS NOT NULL \
         ORDER BY priority DESC LIMIT 1",
    )
    .bind(role_ids)
    .fetch_optional(pool)
    .await?;
    Ok(mapped.flatten().unwrap_or(UserRole::Enlisted))
}

#[cfg(test)]
#[path = "tests/role_sync.rs"]
mod tests;
