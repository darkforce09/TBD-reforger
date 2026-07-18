//! Phase 1 gate — the sqlx migration runner reproduces the full schema.
//!
//! Skips unless `MIGRATE_TEST_DATABASE_URL` points at a **fresh** database (mirrors
//! the Go `t.Skip` on missing `TEST_DATABASE_URL`). The byte-level parity vs the Go
//! schema is proven separately by the G2 `pg_dump` diff; this proves sqlx's runner
//! applies the frozen migration end-to-end and lands the expected object counts.

use website_api::db;

#[tokio::test]
async fn migrate_creates_full_schema() {
    let Ok(url) = std::env::var("MIGRATE_TEST_DATABASE_URL") else {
        eprintln!("skip: MIGRATE_TEST_DATABASE_URL unset");
        return;
    };

    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");

    // 30 base tables (29 Go-parity + registry_compat, T-068.9) + the sqlx
    // `_sqlx_migrations` bookkeeping table.
    let tables: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM information_schema.tables \
         WHERE table_schema = 'public' AND table_type = 'BASE TABLE'",
    )
    .fetch_one(&pool)
    .await
    .expect("count tables");
    assert!(tables >= 30, "expected >= 30 base tables, got {tables}");

    // 12 Postgres enum types.
    let enums: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM pg_type t \
         JOIN pg_namespace n ON n.oid = t.typnamespace \
         WHERE t.typtype = 'e' AND n.nspname = 'public'",
    )
    .fetch_one(&pool)
    .await
    .expect("count enums");
    assert_eq!(enums, 12, "expected 12 enum types, got {enums}");

    // The leaderboard materialized view.
    let matviews: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM pg_matviews WHERE schemaname = 'public' \
         AND matviewname = 'leaderboard_totals'",
    )
    .fetch_one(&pool)
    .await
    .expect("count matviews");
    assert_eq!(matviews, 1, "expected leaderboard_totals matview");

    // Idempotent: a second run is a no-op (already-applied migration).
    db::migrate(&pool).await.expect("migrate idempotent");
}
