//! The statements of the fire-mission tables: the transactional insert of a re-solved save and
//! the per-event read with every mission's guns.
//!
//! **Role:** writes one catalog-model fire mission (its `fire_missions` row and one
//! `fire_mission_guns` row per gun) in one transaction, and reads an event's fire missions,
//! legacy and catalog-model alike, each with its guns.
//!
//! **Position:** operations services; called by
//! [`crate::operations::handlers::fire_missions`] after
//! [`crate::operations::services::fire_mission_resolve::resolve_fire_mission`] accepted the save.
//! Rows decode into [`crate::operations::models::fire_mission::FireMission`].
//!
//! **Signals & state:** none; each function borrows the caller's pool.
//!
//! **Invariants:**
//! - The mission row and its gun rows commit together or not at all.
//! - Every stored number is the server's re-solve: the per-gun columns hold each gun's fired
//!   charge ([`crate::operations::services::fire_mission_resolve::fired_charge`]), the
//!   single-tube summary columns (`distance_m`, `azimuth_deg`, `elevation_mils`, `azimuth_mils`,
//!   `charge`, `time_of_flight_s`) hold the lead gun's, and `fp_grid` holds the lead gun's
//!   position as the `x, y` metre text the legacy rows use.
//! - A foreign-key violation on the event or the catalog answers 404 naming it, never 500.
//! - The insert's `RETURNING` and the list's `SELECT` project one column list, so the two
//!   routes decode the identical shape.

use std::collections::BTreeMap;

use sqlx::PgPool;
use uuid::Uuid;
use website_map_engine::data::scenario::ballistics::fire_mission::{
    FireMissionInputs, HeightSource as InputHeightSource,
};

use crate::core::database::postgres_errors::violated_constraint;
use crate::core::error_handling::api_error::ApiError;
use crate::operations::models::fire_mission::{FireMission, FireMissionGun};
use crate::operations::services::fire_mission_resolve::{ResolvedFireMission, fired_charge};

/// Every column of `fire_missions` a [`FireMission`] decodes, spelled once.
///
/// A macro rather than a `const &str`, because sqlx takes `SqlSafeStr`, implemented for
/// `&'static str` only: expanding to a literal inside `concat!` keeps both statements on that
/// path. `azimuth_deg` is `numeric(5,1)` and casts to `float8`; `created_at` is nullable in the
/// shipped schema and is `COALESCE`d for the non-optional model field.
macro_rules! fire_mission_columns {
    () => {
        "id, event_id, created_by, weapon_system, fp_grid, target_grid, \
         distance_m, azimuth_deg::float8 AS azimuth_deg, elevation_mils, fp_x, fp_y, tgt_x, tgt_y, \
         azimuth_mils, charge, time_of_flight_s, catalog_id, catalog_version, weapon_id, shell_id, \
         charge_rings, target_height_m, target_height_source, wind_speed_m_s, wind_from_deg, \
         burst_height_m, fuze_time_s, mils_per_circle, dispersion, solver_revision, \
         COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at"
    };
}

/// Every column of `fire_mission_guns` a [`FireMissionGun`] decodes.
macro_rules! fire_mission_gun_columns {
    () => {
        "gun_index, label, x, y, height_m, height_source, azimuth_mils, elevation_mils, \
         charge_rings, time_of_flight_s"
    };
}

/// A re-solved save ready to store.
#[derive(Debug, Clone, Copy)]
pub struct NewFireMission<'a> {
    /// The event the mission belongs to; `None` outside an event.
    pub event_id: Option<Uuid>,
    /// Discord id of the author.
    pub created_by: &'a str,
    /// The client's target grid reference, trimmed.
    pub target_grid: &'a str,
    /// The save's inputs.
    pub inputs: &'a FireMissionInputs,
    /// The accepted re-solve.
    pub resolved: &'a ResolvedFireMission,
}

/// The text a height source is stored as.
fn height_source_text(source: InputHeightSource) -> &'static str {
    match source {
        InputHeightSource::Dem => "dem",
        InputHeightSource::Manual => "manual",
    }
}

/// A value that fits `smallint`, or a 400 naming `field`.
fn smallint(value: u32, field: &str) -> Result<i16, ApiError> {
    i16::try_from(value).map_err(|_| ApiError::bad_request(format!("{field} is out of range")))
}

/// Maps a foreign-key violation of the insert to the 404 of the parent it names.
fn parent_not_found(error: sqlx::Error) -> ApiError {
    match violated_constraint(&error) {
        Some("fire_missions_event_id_fkey") => ApiError::not_found("event not found"),
        Some("fire_missions_catalog_fkey") => {
            ApiError::not_found("ballistics catalog version not found")
        }
        _ => error.into(),
    }
}

/// Stores `new` and its guns in one transaction and answers the stored mission.
///
/// # Errors
///
/// 400 for a charge or gun index beyond `smallint`; 404 when the event or the catalog version
/// vanished before the commit; 500 for any other database failure.
pub async fn insert_fire_mission(
    pool: &PgPool,
    new: NewFireMission<'_>,
) -> Result<FireMission, ApiError> {
    let inputs = new.inputs;
    let solution = &new.resolved.solution;
    let lead_gun = &solution.guns[0];
    let lead_input = &inputs.guns[0];
    let lead = fired_charge(lead_gun, inputs.charge_rings);
    // `resolve_fire_mission` refuses a save whose lead gun fires no solving charge.
    let (Some(lead_elevation), Some(lead_rings)) = (lead.elevation_mils, lead.charge_rings) else {
        return Err(ApiError::internal("internal error"));
    };
    let operator_rings = inputs
        .charge_rings
        .map(|rings| smallint(rings, "charge_rings"))
        .transpose()?;
    let catalog_version = i32::try_from(inputs.catalog_version)
        .map_err(|_| ApiError::not_found("ballistics catalog version not found"))?;
    let mils_per_circle = i32::try_from(new.resolved.mils_per_circle)
        .map_err(|_| ApiError::internal("internal error"))?;
    let dispersion = solution
        .dispersion
        .map(serde_json::to_value)
        .transpose()
        .map_err(|_| ApiError::internal("internal error"))?;
    let fuze_time_s = solution.fuze.and_then(|fuze| fuze.time_s);

    let mut transaction = pool.begin().await?;
    let mut mission: FireMission = sqlx::query_as(concat!(
        "INSERT INTO fire_missions \
         (event_id, created_by, weapon_system, fp_grid, target_grid, distance_m, azimuth_deg, \
          elevation_mils, fp_x, fp_y, tgt_x, tgt_y, azimuth_mils, charge, time_of_flight_s, \
          catalog_id, catalog_version, weapon_id, shell_id, charge_rings, target_height_m, \
          target_height_source, wind_speed_m_s, wind_from_deg, burst_height_m, fuze_time_s, \
          mils_per_circle, dispersion, solver_revision, created_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7::float8::numeric, $8, $9, $10, $11, $12, $13, $14, \
                 $15, $16, $17, $18, $19, $20, $21, $22, $23, $24, $25, $26, $27, $28, $29, now()) \
         RETURNING ",
        fire_mission_columns!()
    ))
    .bind(new.event_id)
    .bind(new.created_by)
    .bind(&new.resolved.weapon_display_name)
    .bind(format!("{}, {}", lead_input.x, lead_input.y))
    .bind(new.target_grid)
    .bind(lead_gun.distance_m.round() as i64)
    .bind(lead_gun.azimuth_deg)
    .bind(lead_elevation.round() as i64)
    .bind(lead_input.x)
    .bind(lead_input.y)
    .bind(inputs.target.x)
    .bind(inputs.target.y)
    .bind(lead.azimuth_mils.round() as i64)
    .bind(i64::from(lead_rings))
    .bind(lead.time_of_flight_s)
    .bind(&inputs.catalog_id)
    .bind(catalog_version)
    .bind(&inputs.weapon_id)
    .bind(&inputs.shell_id)
    .bind(operator_rings)
    .bind(inputs.target.height_m)
    .bind(height_source_text(inputs.target.height_source))
    .bind(inputs.wind.map(|wind| wind.speed_m_s))
    .bind(inputs.wind.map(|wind| wind.from_deg))
    .bind(inputs.burst_height_m)
    .bind(fuze_time_s)
    .bind(mils_per_circle)
    .bind(dispersion)
    .bind(&solution.solver_revision)
    .fetch_one(&mut *transaction)
    .await
    .map_err(parent_not_found)?;

    let mut gun_indexes = Vec::with_capacity(solution.guns.len());
    let mut labels = Vec::with_capacity(solution.guns.len());
    let (mut xs, mut ys, mut heights) = (Vec::new(), Vec::new(), Vec::new());
    let mut sources = Vec::with_capacity(solution.guns.len());
    let (mut azimuths, mut elevations, mut rings, mut flight_times) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for (index, (gun, input)) in solution.guns.iter().zip(&inputs.guns).enumerate() {
        let fired = fired_charge(gun, inputs.charge_rings);
        let index = u32::try_from(index).unwrap_or(u32::MAX);
        gun_indexes.push(smallint(index, "gun index")?);
        labels.push(input.label.clone());
        xs.push(input.x);
        ys.push(input.y);
        heights.push(input.height_m);
        sources.push(height_source_text(input.height_source));
        azimuths.push(fired.azimuth_mils);
        elevations.push(fired.elevation_mils);
        rings.push(
            fired
                .charge_rings
                .map(|value| smallint(value, "charge_rings"))
                .transpose()?,
        );
        flight_times.push(fired.time_of_flight_s);
    }
    let mut guns: Vec<FireMissionGun> = sqlx::query_as(concat!(
        "INSERT INTO fire_mission_guns (fire_mission_id, ",
        fire_mission_gun_columns!(),
        ") SELECT $1, * FROM UNNEST($2::smallint[], $3::text[], $4::float8[], $5::float8[], \
           $6::float8[], $7::text[], $8::float8[], $9::float8[], $10::smallint[], $11::float8[]) \
         RETURNING ",
        fire_mission_gun_columns!()
    ))
    .bind(mission.id)
    .bind(&gun_indexes)
    .bind(&labels)
    .bind(&xs)
    .bind(&ys)
    .bind(&heights)
    .bind(&sources)
    .bind(&azimuths)
    .bind(&elevations)
    .bind(&rings)
    .bind(&flight_times)
    .fetch_all(&mut *transaction)
    .await?;
    transaction.commit().await.map_err(parent_not_found)?;
    guns.sort_by_key(|gun| gun.gun_index);
    mission.guns = guns;
    Ok(mission)
}

/// The fire missions of `event`, oldest first, each with its guns in battery order.
///
/// # Errors
///
/// The database error of either read.
pub async fn list_event_fire_missions(
    pool: &PgPool,
    event: Uuid,
) -> sqlx::Result<Vec<FireMission>> {
    let mut missions: Vec<FireMission> = sqlx::query_as(concat!(
        "SELECT ",
        fire_mission_columns!(),
        " FROM fire_missions WHERE event_id = $1 ORDER BY created_at ASC, id ASC"
    ))
    .bind(event)
    .fetch_all(pool)
    .await?;
    let ids: Vec<Uuid> = missions.iter().map(|mission| mission.id).collect();
    #[derive(sqlx::FromRow)]
    struct OwnedGun {
        fire_mission_id: Uuid,
        #[sqlx(flatten)]
        gun: FireMissionGun,
    }
    let rows: Vec<OwnedGun> = sqlx::query_as(concat!(
        "SELECT fire_mission_id, ",
        fire_mission_gun_columns!(),
        " FROM fire_mission_guns WHERE fire_mission_id = ANY($1) \
         ORDER BY fire_mission_id, gun_index"
    ))
    .bind(&ids)
    .fetch_all(pool)
    .await?;
    let mut by_mission: BTreeMap<Uuid, Vec<FireMissionGun>> = BTreeMap::new();
    for row in rows {
        by_mission
            .entry(row.fire_mission_id)
            .or_default()
            .push(row.gun);
    }
    for mission in &mut missions {
        mission.guns = by_mission.remove(&mission.id).unwrap_or_default();
    }
    Ok(missions)
}
