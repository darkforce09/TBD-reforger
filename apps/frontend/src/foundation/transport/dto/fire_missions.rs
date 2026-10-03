//! Fire missions: the save body, the stored mission and the save answer around the engine's
//! battery solution.
//!
//! **Role:** the wire shapes of the mortar calculator's fire missions — the
//! [`FireMissionSave`] body that pins a catalog version, the stored [`SavedFire`] row with its
//! [`FireMissionGun`]s, and the [`SavedFireMissionAnswer`] the save returns. The solution itself
//! ([`FireMissionSolution`] with its [`GunFireSolution`]s, [`ChargeSolution`] rows,
//! [`ImpactDispersion`], [`FireMissionFuze`] with its [`FuzeBurstAim`], and
//! [`CrestClearance`]) is the ballistics crates' own type, carried on the wire unchanged.
//!
//! [`GunFireSolution`]: fire_mission_planning::battery::GunFireSolution
//! [`ChargeSolution`]: ballistics_solver::ChargeSolution
//! [`FireMissionFuze`]: fire_mission_planning::fire_mission::FireMissionFuze
//! [`FuzeBurstAim`]: fire_mission_planning::fire_mission::FuzeBurstAim
//! [`CrestClearance`]: ballistics_solver::crest_clearance::CrestClearance
//! **Position:** deserialised from and serialised to the API's fire-mission routes by the mortar
//! page. The page solves with
//! [`fire_mission_planning::fire_mission::solve_fire_mission`] and posts
//! that solution unchanged as `client_solution`; the API re-solves with the same function, so the
//! solution types exist once, in the ballistics crates, which carry their `@contract` tags.
//! **Signals & state:** none — these are plain data.
//! **Invariants:** a null the API sends stays an explicit `null` when serialising; a key the
//! contract makes optional and non-nullable (the save body's `event_id`, `charge_rings`, `wind`,
//! `burst_height_m`) is absent, never `null`, when it has no value. A stored row whose
//! dispersion claims `verified_in_engine: true` fails the read, because the engine's
//! [`ImpactDispersion`] deserializer holds the contract's constant `false`. A stored row's legacy figures and its catalog-model fields are `Option`s with
//! a default, so a row stored before either existed still decodes, and `guns` defaults to empty
//! for the same reason.
//! @contract fire-mission.schema.json#/definitions/FireMissionSave
//! @contract fire-mission.schema.json#/definitions/MapPoint
//! @contract fire-mission.schema.json#/definitions/GunPosition
//! @contract fire-mission.schema.json#/definitions/HeightSource
//! @contract fire-mission.schema.json#/definitions/Wind
//! @contract fire-mission.schema.json#/definitions/FireMission
//! @contract fire-mission.schema.json#/definitions/FireMissionGun
//! @contract fire-mission.schema.json#/definitions/SavedFireMission
//! @contract fire-mission.schema.json#/definitions/FireMissionList

#[cfg(any(target_arch = "wasm32", test))]
use serde::{Deserialize, Serialize};

// The ballistics crates' battery solution and its dispersion, carried on the wire unchanged.
#[cfg(any(target_arch = "wasm32", test))]
use ballistics_solver::dispersion::ImpactDispersion;
#[cfg(any(target_arch = "wasm32", test))]
use fire_mission_planning::fire_mission::FireMissionSolution;

/// Where a height came from: sampled from the terrain elevation model, or typed by the operator.
#[cfg(any(target_arch = "wasm32", test))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HeightSource {
    Dem,
    Manual,
}

/// A point in map metres with its height and the height's source.
/// @contract fire-mission.schema.json#/definitions/MapPoint
#[cfg(any(target_arch = "wasm32", test))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapPoint {
    pub x: f64,
    pub y: f64,
    pub height_m: f64,
    pub height_source: HeightSource,
}

/// One gun of the battery as the save body names it.
/// @contract fire-mission.schema.json#/definitions/GunPosition
#[cfg(any(target_arch = "wasm32", test))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GunPosition {
    pub label: String,
    pub x: f64,
    pub y: f64,
    pub height_m: f64,
    pub height_source: HeightSource,
}

/// Constant surface wind: speed and the meteorological direction it blows from.
/// @contract fire-mission.schema.json#/definitions/Wind
#[cfg(any(target_arch = "wasm32", test))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Wind {
    pub speed_m_s: f64,
    /// Degrees clockwise from north, in `[0, 360)`.
    pub from_deg: f64,
}

/// `POST /api/v1/fire-missions` body: the solve inputs against a pinned catalog version and the
/// client's solution, which the API re-solves and compares.
/// @contract fire-mission.schema.json#/definitions/FireMissionSave
#[cfg(any(target_arch = "wasm32", test))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FireMissionSave {
    /// The event the mission is saved against; absent outside an event.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
    pub catalog_id: String,
    pub catalog_version: u32,
    pub weapon_id: String,
    pub shell_id: String,
    /// A charge chosen by the operator; absent takes each gun's recommended charge.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub charge_rings: Option<u32>,
    pub target: MapPoint,
    pub guns: Vec<GunPosition>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wind: Option<Wind>,
    /// Burst height above the target for a time-fuzed shell, metres.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub burst_height_m: Option<f64>,
    pub target_grid: String,
    /// The engine's solution of these inputs, as the page solved it.
    pub client_solution: FireMissionSolution,
}

/// One saved fire mission, as the per-event list and the save answer carry it.
///
/// **Typed, and with no default on anything the backend marks required.** A renamed column fails
/// the decode and the saved list goes to its error state, instead of rendering a confident `0 m`.
///
/// `event_id` is absent for a fire mission saved with no event. The four coordinates and the
/// charge, `azimuth_mils` and `time_of_flight_s` are `null` on a row stored before they were
/// recorded. The catalog-model fields (`catalog_id` through `solver_revision`) are all `null` on
/// a row stored before catalogs and all set on a row stored with one; `guns` is empty on the
/// former. `weapon_system` keeps legacy weapon names, including ones no catalog solves.
/// @contract fire-mission.schema.json#/definitions/FireMission
#[cfg(any(target_arch = "wasm32", test))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SavedFire {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
    pub created_by: String,
    pub weapon_system: String,
    pub fp_grid: String,
    pub target_grid: String,
    pub distance_m: i64,
    pub azimuth_deg: f64,
    pub elevation_mils: i64,
    /// The four coordinates, as real numbers. `None` for a row written before the columns
    /// existed, whose only record of them is the `x, y` text of the two grid strings.
    #[serde(default)]
    pub fp_x: Option<f64>,
    #[serde(default)]
    pub fp_y: Option<f64>,
    #[serde(default)]
    pub tgt_x: Option<f64>,
    #[serde(default)]
    pub tgt_y: Option<f64>,
    #[serde(default)]
    pub azimuth_mils: Option<i64>,
    #[serde(default)]
    pub charge: Option<i64>,
    #[serde(default)]
    pub time_of_flight_s: Option<f64>,
    #[serde(default)]
    pub catalog_id: Option<String>,
    #[serde(default)]
    pub catalog_version: Option<u32>,
    #[serde(default)]
    pub weapon_id: Option<String>,
    #[serde(default)]
    pub shell_id: Option<String>,
    /// The charge the operator chose; `None` when each gun took its recommended charge.
    #[serde(default)]
    pub charge_rings: Option<u32>,
    #[serde(default)]
    pub target_height_m: Option<f64>,
    #[serde(default)]
    pub target_height_source: Option<HeightSource>,
    #[serde(default)]
    pub wind_speed_m_s: Option<f64>,
    #[serde(default)]
    pub wind_from_deg: Option<f64>,
    #[serde(default)]
    pub burst_height_m: Option<f64>,
    #[serde(default)]
    pub fuze_time_s: Option<f64>,
    #[serde(default)]
    pub mils_per_circle: Option<u32>,
    /// The lead gun's stored spread; [`ImpactDispersion`]'s own deserializer refuses a
    /// `verified_in_engine` other than `false`.
    #[serde(default)]
    pub dispersion: Option<ImpactDispersion>,
    #[serde(default)]
    pub solver_revision: Option<String>,
    /// The stored guns with the server's solution for each, in battery order.
    #[serde(default)]
    pub guns: Vec<FireMissionGun>,
    pub created_at: String,
}

/// One stored gun of a saved mission with the server's solution for it; the three solution
/// fields are `None` together, when no charge solved.
/// @contract fire-mission.schema.json#/definitions/FireMissionGun
#[cfg(any(target_arch = "wasm32", test))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FireMissionGun {
    pub gun_index: u32,
    pub label: String,
    pub x: f64,
    pub y: f64,
    pub height_m: f64,
    pub height_source: HeightSource,
    /// Azimuth in the weapon's mils.
    pub azimuth_mils: f64,
    pub elevation_mils: Option<f64>,
    pub charge_rings: Option<u32>,
    pub time_of_flight_s: Option<f64>,
}

/// `POST /api/v1/fire-missions` answer: the server's solution and the stored mission. The mortar
/// page decodes the save with this type and the golden round trip holds the same type.
/// @contract fire-mission.schema.json#/definitions/SavedFireMission
#[cfg(any(target_arch = "wasm32", test))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedFireMissionAnswer {
    pub solution: FireMissionSolution,
    pub fire_mission: SavedFire,
}
