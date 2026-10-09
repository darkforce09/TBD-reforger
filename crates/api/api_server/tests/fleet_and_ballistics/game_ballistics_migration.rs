//! Migration `0060_game_ballistics_catalogs_and_fire_mission_inputs.sql` on the binary's database
//! migrated to head: catalog rows refuse UPDATE and DELETE, and complete catalog-model rows decode
//! while their guns cascade with the fire mission.

use crate::common;

use api_operations::models::fire_mission::{FireMission, FireMissionGun, HeightSource};
use sqlx::PgPool;
use uuid::Uuid;

const AUTHOR: &str = "game-ballistics-migration-author";

async fn provisioned() -> PgPool {
    let url = common::require_test_database_url()
        .expect("TEST_DATABASE_URL is unset: this suite needs a test database");
    let pool = api_database::connect(&url).await.expect("connect");
    sqlx::query(
        "INSERT INTO users(discord_id, username, role) VALUES ($1, 'Ballistics author', 'admin') \
         ON CONFLICT (discord_id) DO NOTHING",
    )
    .bind(AUTHOR)
    .execute(&pool)
    .await
    .expect("seed author");
    pool
}

/// Stores one catalog version under a fresh slug and returns `(catalog_id, sha256)`.
async fn store_catalog(pool: &PgPool) -> (String, String) {
    let suffix = Uuid::new_v4().simple().to_string();
    let catalog_id = format!("test-{}", &suffix[..12]);
    let sha = format!("{suffix}{suffix}");
    sqlx::query(
        "INSERT INTO ballistics_catalogs (catalog_id, catalog_version, title, game_build, \
         export_generation_id, catalog_sha256, calibration_sha256, catalog_document, \
         calibration_document, validation_report, uploaded_by) \
         VALUES ($1, 1, 'Test mortars', '1.6.0.119', '5E4F3D2C1B0A9988', $2, $2, \
                 '{\"schema_version\": 1}', '{\"schema_version\": 1}', \
                 '{\"accepted\": true, \"cases\": 0, \"failures\": []}', $3)",
    )
    .bind(&catalog_id)
    .bind(&sha)
    .bind(AUTHOR)
    .execute(pool)
    .await
    .expect("store a catalog version");
    (catalog_id, sha)
}

fn sqlstate_and_constraint(error: &sqlx::Error) -> (String, Option<String>) {
    let database_error = error
        .as_database_error()
        .unwrap_or_else(|| panic!("expected a database error, got {error:?}"));
    (
        database_error
            .code()
            .map(|c| c.into_owned())
            .unwrap_or_default(),
        database_error.constraint().map(str::to_owned),
    )
}

/// A legacy row's columns with the catalog-model columns bound as given; `$1` event, `$2`
/// catalog id, `$3` catalog version.
const INSERT_MODEL_ROW: &str = "INSERT INTO fire_missions (event_id, created_by, weapon_system, \
     fp_grid, target_grid, distance_m, azimuth_deg, elevation_mils, created_at, catalog_id, \
     catalog_version, weapon_id, shell_id, target_height_m, target_height_source, \
     mils_per_circle, solver_revision, dispersion) \
     VALUES ($1, 'game-ballistics-migration-author', 'M252 81mm', '1000, 2000', '2200, 1800', \
             1217, 99.5, 1315, now(), $2, $3, 'm252', 'm821_he', 12.5, 'dem', 6400, 'rk4-v1', \
             '{\"standard_dispersion_m\": 12}') RETURNING id";

#[tokio::test]
async fn game_ballistics_migration_catalog_rows_refuse_update_and_delete() {
    let pool = provisioned().await;
    let (catalog_id, sha) = store_catalog(&pool).await;

    let update =
        sqlx::query("UPDATE ballistics_catalogs SET title = 'Rewritten' WHERE catalog_id = $1")
            .bind(&catalog_id)
            .execute(&pool)
            .await
            .expect_err("an update of a stored catalog version is refused");
    assert_eq!(sqlstate_and_constraint(&update).0, "23514", "{update:?}");
    assert!(
        update
            .to_string()
            .contains("ballistics catalogs are immutable"),
        "{update}"
    );

    let delete = sqlx::query("DELETE FROM ballistics_catalogs WHERE catalog_id = $1")
        .bind(&catalog_id)
        .execute(&pool)
        .await
        .expect_err("a delete of a stored catalog version is refused");
    assert_eq!(sqlstate_and_constraint(&delete).0, "23514", "{delete:?}");

    let (title, stored_sha): (String, String) = sqlx::query_as(
        "SELECT title, catalog_sha256 FROM ballistics_catalogs WHERE catalog_id = $1",
    )
    .bind(&catalog_id)
    .fetch_one(&pool)
    .await
    .expect("the catalog version is still stored");
    assert_eq!(
        (title.as_str(), stored_sha.as_str()),
        ("Test mortars", sha.as_str())
    );

    let duplicate_sha = sqlx::query(
        "INSERT INTO ballistics_catalogs (catalog_id, catalog_version, title, game_build, \
         export_generation_id, catalog_sha256, calibration_sha256, catalog_document, \
         calibration_document, validation_report, uploaded_by) \
         VALUES ($1, 2, 'Same bytes', '1.6.0.119', '5E4F3D2C1B0A9988', $2, $2, '{}', '{}', '{}', \
                 $3)",
    )
    .bind(&catalog_id)
    .bind(&sha)
    .bind(AUTHOR)
    .execute(&pool)
    .await
    .expect_err("the same catalog bytes cannot be stored under a second version");
    assert_eq!(
        sqlstate_and_constraint(&duplicate_sha),
        (
            "23505".to_owned(),
            Some("ballistics_catalogs_catalog_id_catalog_sha256_key".to_owned())
        )
    );
}

#[tokio::test]
async fn game_ballistics_migration_complete_rows_decode_and_guns_cascade() {
    let pool = provisioned().await;
    let (catalog_id, _) = store_catalog(&pool).await;
    let id: Uuid = sqlx::query_scalar(INSERT_MODEL_ROW)
        .bind(None::<Uuid>)
        .bind(&catalog_id)
        .bind(1_i32)
        .fetch_one(&pool)
        .await
        .expect("a complete catalog-model row is stored");
    sqlx::query(
        "INSERT INTO fire_mission_guns (fire_mission_id, gun_index, label, x, y, height_m, \
         height_source, azimuth_mils, elevation_mils, charge_rings, time_of_flight_s) \
         VALUES ($1, 0, 'Gun 1', 1000, 2000, 35.5, 'dem', 1611.2, 1203.4, 2, 21.7), \
                ($1, 1, 'Gun 2', 1010, 2000, 36, 'manual', 1609.9, NULL, NULL, NULL)",
    )
    .bind(id)
    .execute(&pool)
    .await
    .expect("store two guns");

    let mission: FireMission = sqlx::query_as(
        "SELECT id, event_id, created_by, weapon_system, fp_grid, target_grid, distance_m, \
         azimuth_deg::float8 AS azimuth_deg, elevation_mils, fp_x, fp_y, tgt_x, tgt_y, \
         azimuth_mils, charge, time_of_flight_s, catalog_id, catalog_version, weapon_id, \
         shell_id, charge_rings, target_height_m, target_height_source, wind_speed_m_s, \
         wind_from_deg, burst_height_m, fuze_time_s, mils_per_circle, dispersion, \
         solver_revision, created_at FROM fire_missions WHERE id = $1",
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .expect("the row decodes into FireMission");
    assert_eq!(
        mission.catalog_id.as_ref().map(|id| id.as_str()),
        Some(catalog_id.as_str())
    );
    assert_eq!(mission.target_height_source, Some(HeightSource::Dem));
    assert_eq!(mission.mils_per_circle, Some(6400));
    assert!(mission.guns.is_empty());

    let guns: Vec<FireMissionGun> = sqlx::query_as(
        "SELECT gun_index, label, x, y, height_m, height_source, azimuth_mils, elevation_mils, \
         charge_rings, time_of_flight_s FROM fire_mission_guns WHERE fire_mission_id = $1 \
         ORDER BY gun_index",
    )
    .bind(id)
    .fetch_all(&pool)
    .await
    .expect("the guns decode into FireMissionGun");
    assert_eq!(guns.len(), 2);
    assert_eq!(
        (guns[0].height_source, guns[0].charge_rings),
        (HeightSource::Dem, Some(2))
    );
    assert_eq!(
        (guns[1].height_source, guns[1].elevation_mils),
        (HeightSource::Manual, None)
    );

    sqlx::query("DELETE FROM fire_missions WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .expect("delete the mission");
    let remaining: i64 =
        sqlx::query_scalar("SELECT count(*) FROM fire_mission_guns WHERE fire_mission_id = $1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .expect("count guns");
    assert_eq!(remaining, 0, "deleting a fire mission deletes its guns");
}
