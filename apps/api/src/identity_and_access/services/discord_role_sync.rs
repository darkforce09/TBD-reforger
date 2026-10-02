//! Reapply configured TBD guild mappings to the roster's derived display role.

use super::account_authority::{load_account_authority, lock_account};
use crate::identity_and_access::models::user_account::UserRole;
use sqlx::PgPool;

/// Membership status is independent of the number of role rows in its verified snapshot.
fn role_for_membership(status: &str, mapped: UserRole) -> UserRole {
    if status == "member" {
        mapped
    } else {
        UserRole::Guest
    }
}

/// Each account update uses the same lock order as session and membership writers.
/// Actual authorization reads the verified snapshot directly, including freshness and overrides.
pub async fn resync_all_roles(pool: &PgPool, guild_id: &str) -> sqlx::Result<i64> {
    let users: Vec<String> = sqlx::query_scalar(
        "SELECT discord_id FROM users WHERE deleted_at IS NULL ORDER BY discord_id",
    )
    .fetch_all(pool)
    .await?;
    let mut updated = 0;
    for id in users {
        let mut tx = pool.begin().await?;
        if !lock_account(&mut tx, &id).await? {
            continue;
        }
        if let Some(account) = load_account_authority(&mut tx, &id, guild_id).await? {
            let role = role_for_membership(&account.membership_status, account.cached_role);
            updated += sqlx::query(
                "UPDATE users SET role = $2, updated_at = now()
                WHERE discord_id = $1 AND deleted_at IS NULL AND role <> $2",
            )
            .bind(&id)
            .bind(role)
            .execute(&mut *tx)
            .await?
            .rows_affected() as i64;
        }
        tx.commit().await?;
    }
    Ok(updated)
}

#[cfg(test)]
#[path = "tests/discord_role_sync.rs"]
mod tests;
