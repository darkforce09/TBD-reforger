//! Lock order: sorted Arma identities, sorted accounts, link codes, match/attendance rows, leaderboard.
//! Every identity change and telemetry transaction uses these same locks before resolving owners.
use sqlx::PgConnection;

pub async fn lock_identities(connection: &mut PgConnection, ids: &[&str]) -> sqlx::Result<()> {
    let mut ids = ids.to_vec();
    ids.sort_unstable();
    ids.dedup();
    for id in ids {
        sqlx::query(
            "INSERT INTO arma_identity_serialization(arma_id) VALUES ($1) ON CONFLICT DO NOTHING",
        )
        .bind(id)
        .execute(&mut *connection)
        .await?;
        sqlx::query(
            "SELECT arma_id FROM arma_identity_serialization WHERE arma_id = $1 FOR UPDATE",
        )
        .bind(id)
        .fetch_one(&mut *connection)
        .await?;
    }
    Ok(())
}

/// Account writers serialize without blocking registration foreign-key KEY SHARE checks.
/// The partial Arma ownership index keeps identity updates from upgrading to a referenced-key lock.
pub async fn lock_accounts(connection: &mut PgConnection, ids: &[String]) -> sqlx::Result<()> {
    let mut ids = ids.to_vec();
    ids.sort_unstable();
    ids.dedup();
    for id in ids {
        sqlx::query("SELECT discord_id FROM users WHERE discord_id = $1 FOR NO KEY UPDATE")
            .bind(&id)
            .fetch_optional(&mut *connection)
            .await?;
    }
    Ok(())
}
