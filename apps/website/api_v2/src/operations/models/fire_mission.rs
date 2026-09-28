//! The saved fire mission: its legacy single-tube record, its catalog-model inputs and its guns.
//!
//! **Role:** the row and wire shape of `fire_missions` and `fire_mission_guns`.
//!
//! **Position:** operations models; written by the fire-mission save, read by the per-event
//! fire-mission list. The catalog-model columns reference a stored
//! [`crate::operations::models::ballistics_catalog::BallisticsCatalogSummary`] version.
//!
//! **Signals & state:** none; plain data.
//!
//! **Invariants:** the catalog-model inputs (`catalog_id` through `solver_revision`) are either
//! all absent — a row stored before catalogs — or carry every required input; the database check
//! `fire_missions_catalog_model_all_or_none` enforces it. `guns` is empty on the former. A
//! nullable column is an `Option` that serialises as `null`, never as an invented zero.
//!
//! @contract fire-mission.schema.json#/definitions/FireMission
//! @contract fire-mission.schema.json#/definitions/FireMissionGun
//! @contract fire-mission.schema.json#/definitions/HeightSource

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::core::wire_format::rfc3339_utc;

/// Where a height came from: sampled from the terrain's elevation raster, or typed by the
/// operator. Stored as `text` constrained to the two values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "text", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum HeightSource {
    /// Sampled from the terrain's elevation raster.
    Dem,
    /// Entered by the operator.
    Manual,
}

/// A saved fire mission.
///
/// # Why the nullable fields are `Option`
///
/// `None` states a fact about the row, never a measurement. The map coordinates, `azimuth_mils`,
/// `charge` and `time_of_flight_s` are `None` on rows stored before they were recorded. The
/// catalog-model fields are `None` on rows stored before catalogs; on a catalog-model row the
/// optional inputs (`charge_rings`, the wind pair, `burst_height_m`, `fuze_time_s`, `dispersion`)
/// are `None` when the operator gave none or the solution has none. A `0.0` time of flight or
/// charge `0` would be a plausible, wrong, unfalsifiable number, so no field defaults to zero.
///
/// Every nullable field serialises **unconditionally**, as `null` rather than as an absent key,
/// except `event_id`, whose absence is the real state of a mission outside an event.
///
/// `weapon_system` keeps the legacy weapon names, including ones no catalog solves.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct FireMission {
    pub id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub event_id: Option<Uuid>,
    pub created_by: String,
    pub weapon_system: String,
    pub fp_grid: String,
    pub target_grid: String,
    pub distance_m: i64,
    /// `numeric(5,1)` — queries must `CAST(azimuth_deg AS double precision)`.
    pub azimuth_deg: f64,
    pub elevation_mils: i64,
    /// Firing position, flat game-world metres (`double precision`).
    #[serde(default)]
    pub fp_x: Option<f64>,
    #[serde(default)]
    pub fp_y: Option<f64>,
    /// Target, flat game-world metres (`double precision`).
    #[serde(default)]
    pub tgt_x: Option<f64>,
    #[serde(default)]
    pub tgt_y: Option<f64>,
    /// The sight setting — `azimuth_deg` is the human-readable echo, this is what is dialled.
    #[serde(default)]
    pub azimuth_mils: Option<i64>,
    /// The propellant ring the crew sets on the round. An elevation without it is half a fire
    /// order.
    #[serde(default)]
    pub charge: Option<i64>,
    /// Seconds to splash.
    #[serde(default)]
    pub time_of_flight_s: Option<f64>,
    /// Slug of the pinned catalog.
    #[serde(default)]
    pub catalog_id: Option<String>,
    /// Pinned catalog version (`integer`, 1 or more).
    #[serde(default)]
    pub catalog_version: Option<i32>,
    /// Weapon system slug within the pinned catalog.
    #[serde(default)]
    pub weapon_id: Option<String>,
    /// Shell slug within the pinned catalog.
    #[serde(default)]
    pub shell_id: Option<String>,
    /// The charge the operator chose (`smallint`); `None` takes each gun's recommended charge.
    #[serde(default)]
    pub charge_rings: Option<i16>,
    /// Target height, metres above the terrain datum.
    #[serde(default)]
    pub target_height_m: Option<f64>,
    #[serde(default)]
    pub target_height_source: Option<HeightSource>,
    /// Constant wind speed, metres per second; set together with `wind_from_deg`.
    #[serde(default)]
    pub wind_speed_m_s: Option<f64>,
    /// Meteorological wind direction, degrees clockwise from north, in `[0, 360)`.
    #[serde(default)]
    pub wind_from_deg: Option<f64>,
    /// Burst height above the target for a time-fuzed shell, metres.
    #[serde(default)]
    pub burst_height_m: Option<f64>,
    /// The lead gun's fuze setting, seconds.
    #[serde(default)]
    pub fuze_time_s: Option<f64>,
    /// Mils per full circle in the weapon's convention (`integer`, 1 or more).
    #[serde(default)]
    pub mils_per_circle: Option<i32>,
    /// The lead gun's dispersion (`jsonb`, the schema's `Dispersion` object).
    #[serde(default)]
    pub dispersion: Option<serde_json::Value>,
    /// Revision of the solver that produced the stored solution.
    #[serde(default)]
    pub solver_revision: Option<String>,
    /// The mission's guns in `gun_index` order, read from `fire_mission_guns` by a second query;
    /// empty on a row stored before catalogs.
    #[sqlx(skip)]
    #[serde(default)]
    pub guns: Vec<FireMissionGun>,
    #[serde(with = "rfc3339_utc")]
    pub created_at: DateTime<Utc>,
}

/// One stored gun of a saved fire mission with the server's solution for it. The three solution
/// fields are `None` together, when no charge solved; the database check
/// `fire_mission_guns_solution_all_or_none` enforces it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, sqlx::FromRow)]
pub struct FireMissionGun {
    /// Position of the gun in the battery, from 0 (`smallint`).
    pub gun_index: i16,
    pub label: String,
    /// Gun position, flat game-world metres.
    pub x: f64,
    pub y: f64,
    /// Gun height, metres above the terrain datum.
    pub height_m: f64,
    pub height_source: HeightSource,
    /// Azimuth in the weapon's convention, in `[0, mils_per_circle)`.
    pub azimuth_mils: f64,
    /// Elevation in the weapon's convention.
    pub elevation_mils: Option<f64>,
    /// The charge this gun fires (`smallint`).
    pub charge_rings: Option<i16>,
    /// Seconds to splash.
    pub time_of_flight_s: Option<f64>,
}
