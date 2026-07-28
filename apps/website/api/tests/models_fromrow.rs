//! Phase 2 gate — the sqlx `FromRow` decode path is correct for the tricky types:
//! Postgres ENUM → Rust enum, `timestamptz` → `DateTime<Utc>`, `bigint` → `i64`,
//! `numeric` → `f64` (via `::float8` cast), and `jsonb` → RawValue passthrough.
//!
//! Skips unless `TEST_DATABASE_URL` is set. T-558: no longer shares
//! `MIGRATE_TEST_DATABASE_URL` with `db_migrate.rs` — [`common::require_test_database_url`]
//! gives this binary its own `<base>_models_fromrow_it` database.
//!
//! NOTE: the Rust app sets `created_at`/`updated_at` explicitly on INSERT (GORM did this
//! app-side; the columns have no DB default) — the inserts below mirror that.
//!
//! Also hosts the T-376 sparse-reimport Class-R (moved out of
//! `src/services/registry_import.rs` at T-558 so the DB consumer goes through the common
//! guard and the t542 `src/` scan stays green).

mod common;

use uuid::Uuid;
use website_api::db;
use website_api::models::{MissionVersion, User, UserRole};
use website_api::services::registry_import::import_items;

#[tokio::test]
async fn fromrow_decodes_enum_numeric_timestamp_jsonb() {
    let Some(url) = common::require_test_database_url() else {
        eprintln!("skip: TEST_DATABASE_URL unset");
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

/// Class-R (T-376 / T-558): sparse re-import must not NULL populated Option columns.
///
/// Lived in `src/services/registry_import.rs` as a lib `#[tokio::test]` that read
/// `TEST_DATABASE_URL` raw — invisible to the tests-only t542 scan and pointed at the
/// operator base. Moved here so the consumer uses the common per-binary guard.
#[tokio::test]
async fn sparse_reimport_preserves_option_columns() {
    let Some(url) = common::require_test_database_url() else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");

    const MP: &str = "00000000-0000-4000-a000-000000003377";
    const RN: &str = "{DEADBEEF00003761}Prefabs/Clothing/T376_ClassR_Vest.et";

    let rich = format!(
        r#"{{
  "registryItemsVersion": "2",
  "modpackId": "{MP}",
  "generatedAt": "2026-07-27T00:00:00Z",
  "addons": [{{ "guid": "5EB744C5F42E0800", "name": "ArmaReforger", "title": "Arma Reforger", "vanilla": true }}],
  "items": [{{
    "resource_name": "{RN}",
    "display_name": "  T376 ClassR Vest  ",
    "category": "  NATO/Vest  ",
    "kind": "gear_vest",
    "abstract": false,
    "arsenal_type": "VEST",
    "weight_kg": 2.5,
    "volume_cm3": 400.0,
    "max_weight_kg": 15.0,
    "max_volume_cm3": 2000.0,
    "addon": "ArmaReforger",
    "cargo_grid_w": 4,
    "cargo_grid_h": 6,
    "icon_url": "items/t376.png"
  }}]
}}"#
    )
    .into_bytes();

    let sparse = format!(
        r#"{{
  "registryItemsVersion": "2",
  "modpackId": "{MP}",
  "generatedAt": "2026-07-27T00:00:00Z",
  "addons": [{{ "guid": "5EB744C5F42E0800", "name": "ArmaReforger", "title": "Arma Reforger", "vanilla": true }}],
  "items": [{{
    "resource_name": "{RN}",
    "display_name": "T376 ClassR Vest Renamed",
    "category": "NATO/Vest",
    "kind": "gear_vest"
  }}]
}}"#
    )
    .into_bytes();

    let mp = Uuid::parse_str(MP).unwrap();
    for q in [
        "DELETE FROM registry_items WHERE modpack_id = $1",
        "DELETE FROM modpacks WHERE id = $1",
    ] {
        sqlx::query(q).bind(mp).execute(&pool).await.expect("clean");
    }

    let c1 = import_items(&pool, &rich, Some(mp), false)
        .await
        .expect("rich");
    assert_eq!((c1.inserted, c1.updated), (1, 0));

    #[derive(sqlx::FromRow)]
    struct Row {
        display_name: String,
        category: String,
        weight_kg: Option<f64>,
        volume_cm3: Option<f64>,
        max_weight_kg: Option<f64>,
        max_volume_cm3: Option<f64>,
        addon: Option<String>,
        arsenal_type: Option<String>,
        abstract_: Option<bool>,
        cargo_grid_w: Option<i32>,
        cargo_grid_h: Option<i32>,
        icon_url: String,
    }

    let after_rich: Row = sqlx::query_as(
        "SELECT display_name, category, weight_kg, volume_cm3, max_weight_kg, max_volume_cm3, \
         addon, arsenal_type, \"abstract\" AS abstract_, cargo_grid_w, cargo_grid_h, \
         COALESCE(icon_url, '') AS icon_url \
         FROM registry_items WHERE modpack_id = $1 AND resource_name = $2",
    )
    .bind(mp)
    .bind(RN)
    .fetch_one(&pool)
    .await
    .expect("after rich");
    assert_eq!(
        after_rich.display_name, "T376 ClassR Vest",
        "trim display_name"
    );
    assert_eq!(after_rich.category, "NATO/Vest", "trim category");
    assert_eq!(after_rich.weight_kg, Some(2.5));
    assert_eq!(after_rich.icon_url, "items/t376.png");

    let c2 = import_items(&pool, &sparse, Some(mp), false)
        .await
        .expect("sparse");
    // display_name change forces the UPDATE path; Option absences must not NULL columns.
    assert_eq!(c2.inserted, 0);
    assert_eq!(c2.updated, 1, "display_name rename must update the row");

    let after_sparse: Row = sqlx::query_as(
        "SELECT display_name, category, weight_kg, volume_cm3, max_weight_kg, max_volume_cm3, \
         addon, arsenal_type, \"abstract\" AS abstract_, cargo_grid_w, cargo_grid_h, \
         COALESCE(icon_url, '') AS icon_url \
         FROM registry_items WHERE modpack_id = $1 AND resource_name = $2",
    )
    .bind(mp)
    .bind(RN)
    .fetch_one(&pool)
    .await
    .expect("after sparse");

    assert_eq!(after_sparse.display_name, "T376 ClassR Vest Renamed");
    assert_eq!(after_sparse.weight_kg, Some(2.5), "weight_kg preserved");
    assert_eq!(after_sparse.volume_cm3, Some(400.0), "volume_cm3 preserved");
    assert_eq!(
        after_sparse.max_weight_kg,
        Some(15.0),
        "max_weight_kg preserved"
    );
    assert_eq!(
        after_sparse.max_volume_cm3,
        Some(2000.0),
        "max_volume_cm3 preserved"
    );
    assert_eq!(after_sparse.addon.as_deref(), Some("ArmaReforger"));
    assert_eq!(after_sparse.arsenal_type.as_deref(), Some("VEST"));
    assert_eq!(after_sparse.abstract_, Some(false));
    assert_eq!(after_sparse.cargo_grid_w, Some(4));
    assert_eq!(after_sparse.cargo_grid_h, Some(6));
    assert_eq!(
        after_sparse.icon_url, "items/t376.png",
        "icon_url still never updated"
    );
}
