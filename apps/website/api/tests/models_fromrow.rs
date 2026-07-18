//! Phase 2 gate — the sqlx `FromRow` decode path is correct for the tricky types:
//! Postgres ENUM → Rust enum, `timestamptz` → `DateTime<Utc>`, `bigint` → `i64`,
//! `numeric` → `f64` (via `::float8` cast), and `jsonb` → RawValue passthrough.
//!
//! Skips unless `MIGRATE_TEST_DATABASE_URL` points at a migrated DB. NOTE: the Rust
//! app sets `created_at`/`updated_at` explicitly on INSERT (GORM did this app-side;
//! the columns have no DB default) — the inserts below mirror that.

use uuid::Uuid;
use website_api::db;
use website_api::models::{MissionVersion, User, UserRole};

#[tokio::test]
async fn fromrow_decodes_enum_numeric_timestamp_jsonb() {
    let Ok(url) = std::env::var("MIGRATE_TEST_DATABASE_URL") else {
        eprintln!("skip: MIGRATE_TEST_DATABASE_URL unset");
        return;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");

    let did = format!("frt-{}", Uuid::new_v4());

    // --- User: enum (role), bigint (total_deployments), numeric cast (attendance_rate) ---
    // Mirrors the app: non-pointer string columns get '' (never NULL); created/updated set app-side.
    sqlx::query(
        "INSERT INTO users (discord_id, username, discord_handle, avatar_url, arma_character, \
         role, is_banned, ban_reason, total_deployments, attendance_rate, created_at, updated_at) \
         VALUES ($1, 'FromRow Fran', '', '', '', 'admin', false, '', 7, 94.5, now(), now())",
    )
    .bind(&did)
    .execute(&pool)
    .await
    .expect("insert user");

    let u: User = sqlx::query_as(
        "SELECT discord_id, username, discord_handle, avatar_url, arma_id, arma_character, \
         role, is_banned, ban_reason, banned_by, banned_at, total_deployments, \
         attendance_rate::float8 AS attendance_rate, last_login_at, created_at, updated_at \
         FROM users WHERE discord_id = $1",
    )
    .bind(&did)
    .fetch_one(&pool)
    .await
    .expect("decode user");
    assert_eq!(u.role, UserRole::Admin);
    assert_eq!(u.total_deployments, 7);
    assert!((u.attendance_rate - 94.5).abs() < 1e-9, "numeric->f64 cast");

    // --- MissionVersion: jsonb passthrough (Postgres-normalized bytes, no reformat) ---
    let mid: Uuid = sqlx::query_scalar(
        "INSERT INTO missions (title, author_id, terrain, game_mode, max_players, status, created_at, updated_at) \
         VALUES ('t', $1, 'everon', 'pve_coop', 10, 'draft', now(), now()) RETURNING id",
    )
    .bind(&did)
    .fetch_one(&pool)
    .await
    .expect("insert mission");

    sqlx::query(
        "INSERT INTO mission_versions (mission_id, semver, json_payload, editor_notes, created_by, created_at) \
         VALUES ($1, '0.1.0', '{\"b\": 2, \"a\": 1}'::jsonb, '', $2, now())",
    )
    .bind(mid)
    .bind(&did)
    .execute(&pool)
    .await
    .expect("insert version");

    let mv: MissionVersion = sqlx::query_as(
        "SELECT id, mission_id, semver, json_payload, editor_notes, created_by, created_at \
         FROM mission_versions WHERE mission_id = $1",
    )
    .bind(mid)
    .fetch_one(&pool)
    .await
    .expect("decode version");
    let v = serde_json::to_value(&mv).expect("serialize");
    assert_eq!(v["json_payload"]["a"], serde_json::json!(1));
    assert_eq!(v["json_payload"]["b"], serde_json::json!(2));

    // cleanup
    let _ = sqlx::query("DELETE FROM mission_versions WHERE mission_id = $1")
        .bind(mid)
        .execute(&pool)
        .await;
    let _ = sqlx::query("DELETE FROM missions WHERE id = $1")
        .bind(mid)
        .execute(&pool)
        .await;
    let _ = sqlx::query("DELETE FROM users WHERE discord_id = $1")
        .bind(&did)
        .execute(&pool)
        .await;
}
