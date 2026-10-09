//! The fire-mission and ballistics-catalog rows of the null-tolerance seed.
//!
//! **Role:** seeds one legacy single-tube fire mission, one catalog-model fire mission with one
//! gun, and the catalog version the catalog-model row pins; names their blast rows and the swept
//! catalog routes.
//!
//! **Position:** called by [`super::database_fixtures::seed`] and
//! [`super::route_sweep::route_sweep`]; the rows it names are NULLed by
//! [`super::database_fixtures::blast_nulls`] like every other seeded table.
//!
//! **Signals & state:** writes to the shared integration database. The catalog version is
//! stored once under a fixed id and reused on every run, because `ballistics_catalogs` refuses
//! `UPDATE` and `DELETE` by trigger: a catalog per run could never be cleaned up.
//!
//! **Invariants:** both fire missions are created by [`super::NULL_UID`] and attached to the
//! seeded event, and `fire_missions.event_id` is in [`super::REACHABILITY_KEEP`], so
//! `GET /api/v1/events/{id}/fire-missions` still reaches both rows after the blast. The
//! catalog-model row and its gun set every nullable column, so the blast drives each one from a
//! stored value to `NULL` rather than leaving it at `NULL` all along.

use sqlx::PgPool;
use uuid::Uuid;

use super::{NULL_UID, SweepCaller};

/// Slug of the catalog version the catalog-model fire mission pins.
pub(crate) const NULL_CATALOG_ID: &str = "null-tolerance";

/// Version of [`NULL_CATALOG_ID`] the seed stores.
pub(crate) const NULL_CATALOG_VERSION: i32 = 1;

/// Stores the pinned catalog version (once), both fire missions and the gun, attached to
/// `event`; returns their `(table, WHERE clause)` blast rows.
///
/// # Panics
///
/// On any database error, naming the statement.
pub(crate) async fn seed_fire_missions(pool: &PgPool, event: Uuid) -> Vec<(&'static str, String)> {
    let catalog_sha256 = "99".repeat(32);
    sqlx::query(
        "INSERT INTO ballistics_catalogs (catalog_id, catalog_version, title, game_build, \
         export_generation_id, catalog_sha256, calibration_sha256, catalog_document, \
         calibration_document, validation_report, uploaded_by) \
         VALUES ($1, $2, 'Null tolerance mortars', '1.0.0', '0000000000000099', $3, $3, \
                 '{\"schema_version\": 1}', '{\"schema_version\": 1}', \
                 '{\"accepted\": true, \"cases\": 0, \"failures\": []}', $4) \
         ON CONFLICT (catalog_id, catalog_version) DO NOTHING",
    )
    .bind(NULL_CATALOG_ID)
    .bind(NULL_CATALOG_VERSION)
    .bind(&catalog_sha256)
    .bind(NULL_UID)
    .execute(pool)
    .await
    .expect("seed: the pinned ballistics catalog version");

    sqlx::query(
        "INSERT INTO fire_missions (event_id, created_by, weapon_system, fp_grid, target_grid, \
         distance_m, azimuth_deg, elevation_mils, created_at) \
         VALUES ($1, $2, 'm252', '012345', '054321', 1000, 90.0, 800, now())",
    )
    .bind(event)
    .bind(NULL_UID)
    .execute(pool)
    .await
    .expect("seed: the legacy single-tube fire mission");

    let catalog_model_mission: Uuid = sqlx::query_scalar(
        "INSERT INTO fire_missions (event_id, created_by, weapon_system, fp_grid, target_grid, \
         distance_m, azimuth_deg, elevation_mils, created_at, fp_x, fp_y, tgt_x, tgt_y, \
         azimuth_mils, charge, time_of_flight_s, catalog_id, catalog_version, weapon_id, \
         shell_id, charge_rings, target_height_m, target_height_source, wind_speed_m_s, \
         wind_from_deg, burst_height_m, fuze_time_s, mils_per_circle, dispersion, \
         solver_revision) \
         VALUES ($1, $2, 'M252 81mm', '1000, 2000', '1000, 3000', 1000, 0.0, 1300, now(), \
                 1000, 2000, 1000, 3000, 0, 2, 20.5, $3, $4, 'm252', 'm821_he', 2, 12.5, 'dem', \
                 3.0, 270.0, 10.0, 19.8, 6400, \
                 '{\"range_probable_error_m\": 8.1, \"deflection_probable_error_m\": 4.2, \
                   \"ellipse_semi_major_m\": 9.5, \"ellipse_semi_minor_m\": 4.9, \
                   \"ellipse_orientation_deg\": 0.0, \"standard_dispersion_m\": 12.0, \
                   \"verified_in_engine\": false}', \
                 'rk4-v1') \
         RETURNING id",
    )
    .bind(event)
    .bind(NULL_UID)
    .bind(NULL_CATALOG_ID)
    .bind(NULL_CATALOG_VERSION)
    .fetch_one(pool)
    .await
    .expect("seed: the catalog-model fire mission");

    sqlx::query(
        "INSERT INTO fire_mission_guns (fire_mission_id, gun_index, label, x, y, height_m, \
         height_source, azimuth_mils, elevation_mils, charge_rings, time_of_flight_s) \
         VALUES ($1, 0, 'Gun 1', 1000, 2000, 12.0, 'dem', 0, 1300, 2, 20.5)",
    )
    .bind(catalog_model_mission)
    .execute(pool)
    .await
    .expect("seed: the catalog-model fire mission's gun");

    // Keyed off `created_by` and `fire_mission_id`, both NOT NULL, so neither blast clears the
    // column its own WHERE clause matches on.
    vec![
        ("fire_missions", format!("created_by = '{NULL_UID}'")),
        (
            "fire_mission_guns",
            format!(
                "fire_mission_id IN (SELECT id FROM fire_missions WHERE created_by = '{NULL_UID}')"
            ),
        ),
    ]
}

/// The [`super::route_sweep::route_sweep`] entries of the two public catalog reads: the
/// summary list and the pinned version's document.
pub(crate) fn ballistics_catalog_sweep() -> Vec<(&'static str, String, SweepCaller)> {
    vec![
        (
            "/ballistics-catalogs",
            "/api/v1/ballistics-catalogs".into(),
            SweepCaller::Member,
        ),
        (
            "/ballistics-catalogs/{catalogId}/versions/{version}",
            format!(
                "/api/v1/ballistics-catalogs/{NULL_CATALOG_ID}/versions/{NULL_CATALOG_VERSION}"
            ),
            SweepCaller::Member,
        ),
    ]
}
