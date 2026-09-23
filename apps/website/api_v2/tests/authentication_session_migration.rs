//! Upgrade a populated pre-session schema without changing identity or historical records.

use sqlx::{AssertSqlSafe, PgPool};
use std::borrow::Cow;
use uuid::Uuid;
use website_api::core::database;
mod common;

#[tokio::test]
async fn session_migration_preserves_accounts_and_invalidates_unverified_legacy_credentials() {
    let base = common::require_test_database_url().unwrap();
    let maintenance = database::connect(&base).await.unwrap();
    let mut url = url::Url::parse(&base).unwrap();
    let prefix: String = url
        .path()
        .trim_start_matches('/')
        .chars()
        .take(42)
        .collect();
    let suffix = Uuid::new_v4().simple().to_string();
    let name = format!("{prefix}_upgrade_{}_it", &suffix[..8]);
    assert!(name.len() <= 63 && name.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_'));
    url.set_path(&name);
    common::assert_test_database_url(url.as_str());
    sqlx::raw_sql(AssertSqlSafe(format!("CREATE DATABASE {name}")))
        .execute(&maintenance)
        .await
        .unwrap();
    let pool = database::connect(url.as_str()).await.unwrap();
    let result = upgrade(&pool).await;
    pool.close().await;
    sqlx::raw_sql(AssertSqlSafe(format!("DROP DATABASE {name}")))
        .execute(&maintenance)
        .await
        .unwrap();
    result.unwrap();
}

async fn upgrade(pool: &PgPool) -> anyhow::Result<()> {
    let all = sqlx::migrate!("./migrations");
    let baseline = sqlx::migrate::Migrator {
        migrations: Cow::Owned(
            all.migrations
                .iter()
                .filter(|m| m.version <= 26)
                .cloned()
                .collect(),
        ),
        ..sqlx::migrate::Migrator::DEFAULT
    };
    baseline.run(pool).await?;
    sqlx::query(
        "INSERT INTO users(discord_id, username, role, arma_id) VALUES
        ('legacy-member', 'Preserved member', 'admin', 'verified-arma'),
        ('000000000000000001', 'Dev Operator', 'admin', NULL)",
    )
    .execute(pool)
    .await?;
    let legacy = Uuid::new_v4();
    let development = Uuid::new_v4();
    for (id, actor, hash) in [
        (legacy, "legacy-member", "legacy-hash"),
        (development, "000000000000000001", "development-hash"),
    ] {
        sqlx::query("INSERT INTO refresh_tokens(id, discord_id, token_hash, expires_at) VALUES ($1, $2, $3, now() + interval '1 day')")
            .bind(id).bind(actor).bind(hash).execute(pool).await?;
    }
    sqlx::query("INSERT INTO audit_logs(actor_id, actor_name, action, message, target_type, target_id)
        VALUES ('legacy-member', 'Preserved member', 'historical.action', 'Preserved fact', 'user', 'legacy-member')")
        .execute(pool).await?;
    sqlx::query(
        "INSERT INTO identity_link_codes(code, discord_id, created_at, expires_at) VALUES
        ('100001','legacy-member',now()-interval '2 minutes',now()+interval '10 minutes'),
        ('100002','legacy-member',now()-interval '1 minute',now()+interval '10 minutes')",
    )
    .execute(pool)
    .await?;
    all.run(pool).await?;
    let pending: Vec<String> = sqlx::query_scalar(
        "SELECT code FROM identity_link_codes WHERE discord_id='legacy-member'
        AND consumed_at IS NULL AND cancelled_at IS NULL ORDER BY code",
    )
    .fetch_all(pool)
    .await?;
    anyhow::ensure!(
        pending == vec!["100002"],
        "upgrade did not retain exactly the latest pending code"
    );
    let older: (bool, Option<String>) = sqlx::query_as("SELECT cancelled_at IS NOT NULL, cancellation_reason FROM identity_link_codes WHERE code='100001'")
        .fetch_one(pool).await?;
    anyhow::ensure!(
        older == (true, Some("superseded".into())),
        "upgrade lost code cancellation history"
    );
    let preserved: (String, Option<String>) =
        sqlx::query_as("SELECT username, arma_id FROM users WHERE discord_id = 'legacy-member'")
            .fetch_one(pool)
            .await?;
    anyhow::ensure!(
        preserved == ("Preserved member".into(), Some("verified-arma".into())),
        "account facts changed"
    );
    let revoked: i64 = sqlx::query_scalar("SELECT count(*) FROM refresh_tokens r JOIN authentication_sessions s ON s.id = r.session_id
        WHERE r.id = ANY($1) AND r.revoked_at IS NOT NULL AND s.revoked_at IS NOT NULL")
        .bind(vec![legacy, development]).fetch_one(pool).await?;
    anyhow::ensure!(
        revoked == 2,
        "unknown-provenance credentials survived upgrade"
    );
    let snapshots: i64 = sqlx::query_scalar("SELECT count(*) FROM discord_membership_snapshots")
        .fetch_one(pool)
        .await?;
    anyhow::ensure!(snapshots == 0, "upgrade invented Discord membership");
    let history: i64 = sqlx::query_scalar("SELECT count(*) FROM audit_logs a JOIN audit_publication_pending p ON p.audit_id = a.id WHERE a.action = 'historical.action'")
        .fetch_one(pool).await?;
    anyhow::ensure!(
        history == 1,
        "historical audit was not preserved and queued"
    );
    Ok(())
}
