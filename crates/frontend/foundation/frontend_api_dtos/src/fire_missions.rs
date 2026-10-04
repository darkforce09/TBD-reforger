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

use serde::{Deserialize, Serialize};

// The ballistics crates' battery solution and its dispersion, carried on the wire unchanged.
use super::identifiers::{
    BallisticsCatalogId, BallisticsShellId, BallisticsWeaponId, EventId, FireMissionId,
};
use ballistics_solver::dispersion::ImpactDispersion;
use fire_mission_planning::fire_mission::FireMissionSolution;

/// Where a height came from: sampled from the terrain elevation model, or typed by the operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HeightSource {
    /// Read from the terrain's elevation model.
    Dem,
    /// Typed in by the user.
    Manual,
}

/// A point in map metres with its height and the height's source.
/// @contract fire-mission.schema.json#/definitions/MapPoint
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapPoint {
    /// Gun position, flat game-world metres.
    pub x: f64,
    /// Gun position northing, flat game-world metres.
    pub y: f64,
    /// Gun height, metres above the terrain datum.
    pub height_m: f64,
    /// Where `height_m` came from.
    pub height_source: HeightSource,
}

/// One gun of the battery as the save body names it.
/// @contract fire-mission.schema.json#/definitions/GunPosition
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GunPosition {
    /// Name of the gun shown on the plot and the fire orders.
    pub label: String,
    /// Gun position, flat game-world metres.
    pub x: f64,
    /// Gun position northing, flat game-world metres.
    pub y: f64,
    /// Gun height, metres above the terrain datum.
    pub height_m: f64,
    /// Where `height_m` came from.
    pub height_source: HeightSource,
}

/// Constant surface wind: speed and the meteorological direction it blows from.
/// @contract fire-mission.schema.json#/definitions/Wind
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Wind {
    /// The wind speed, in metres per second.
    pub speed_m_s: f64,
    /// Degrees clockwise from north, in `[0, 360)`.
    pub from_deg: f64,
}

/// `POST /api/v1/fire-missions` body: the solve inputs against a pinned catalog version and the
/// client's solution, which the API re-solves and compares.
/// @contract fire-mission.schema.json#/definitions/FireMissionSave
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FireMissionSave {
    /// The event the mission is saved against; absent outside an event.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_id: Option<EventId>,
    /// Catalog the mission is solved against.
    pub catalog_id: BallisticsCatalogId,
    /// Version of that catalog, 1 or more.
    pub catalog_version: u32,
    /// Launcher identifier in the catalog.
    pub weapon_id: BallisticsWeaponId,
    /// Shell identifier in the catalog.
    pub shell_id: BallisticsShellId,
    /// A charge chosen by the operator; absent takes each gun's recommended charge.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub charge_rings: Option<u32>,
    /// Where the shells must land.
    pub target: MapPoint,
    /// The guns; the first is the lead gun.
    pub guns: Vec<GunPosition>,
    /// The surface wind; absent or `null` is calm air.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wind: Option<Wind>,
    /// Burst height above the target for a time-fuzed shell, metres.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub burst_height_m: Option<f64>,
    /// The target's grid reference as the operator entered it.
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
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SavedFire {
    /// Fire mission id (uuid).
    pub id: FireMissionId,
    /// The event the mission was saved under; `None` (absent on the wire) outside an event.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_id: Option<EventId>,
    /// Discord id of the member who saved the mission.
    pub created_by: String,
    /// Display name of the weapon system fired.
    pub weapon_system: String,
    /// Firing position text; the lead gun's `x, y` metres on a catalog-model row.
    pub fp_grid: String,
    /// The target's grid reference as the operator entered it (trimmed).
    pub target_grid: String,
    /// Lead gun's range to the target, whole metres.
    pub distance_m: i64,
    /// Lead gun's azimuth, in degrees to one decimal.
    pub azimuth_deg: f64,
    /// Lead gun's elevation, whole mils.
    pub elevation_mils: i64,
    /// The four coordinates, as real numbers. `None` for a row written before the columns
    /// existed, whose only record of them is the `x, y` text of the two grid strings.
    #[serde(default)]
    pub fp_x: Option<f64>,
    /// Firing position northing, flat game-world metres (`double precision`).
    #[serde(default)]
    pub fp_y: Option<f64>,
    /// Target, flat game-world metres (`double precision`).
    #[serde(default)]
    pub tgt_x: Option<f64>,
    /// Target northing, flat game-world metres (`double precision`).
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
    pub catalog_id: Option<BallisticsCatalogId>,
    /// Pinned catalog version (`integer`, 1 or more).
    #[serde(default)]
    pub catalog_version: Option<u32>,
    /// Weapon system slug within the pinned catalog.
    #[serde(default)]
    pub weapon_id: Option<BallisticsWeaponId>,
    /// Shell slug within the pinned catalog.
    #[serde(default)]
    pub shell_id: Option<BallisticsShellId>,
    /// The charge the operator chose; `None` when each gun took its recommended charge.
    #[serde(default)]
    pub charge_rings: Option<u32>,
    /// Target height, metres above the terrain datum.
    #[serde(default)]
    pub target_height_m: Option<f64>,
    /// Where `target_height_m` came from; `None` on a row stored before catalogs.
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
    pub mils_per_circle: Option<u32>,
    /// The lead gun's stored spread; [`ImpactDispersion`]'s own deserializer refuses a
    /// `verified_in_engine` other than `false`.
    #[serde(default)]
    pub dispersion: Option<ImpactDispersion>,
    /// Revision of the solver that produced the stored solution.
    #[serde(default)]
    pub solver_revision: Option<String>,
    /// The stored guns with the server's solution for each, in battery order.
    #[serde(default)]
    pub guns: Vec<FireMissionGun>,
    /// When the mission was saved (RFC 3339 UTC on the wire).
    pub created_at: String,
}

/// One stored gun of a saved mission with the server's solution for it; the three solution
/// fields are `None` together, when no charge solved.
/// @contract fire-mission.schema.json#/definitions/FireMissionGun
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FireMissionGun {
    /// Position of the gun in the battery, from 0 (`smallint`).
    pub gun_index: u32,
    /// Name of the gun shown on the plot and the fire orders.
    pub label: String,
    /// Gun position, flat game-world metres.
    pub x: f64,
    /// Gun position northing, flat game-world metres.
    pub y: f64,
    /// Gun height, metres above the terrain datum.
    pub height_m: f64,
    /// Where `height_m` came from.
    pub height_source: HeightSource,
    /// Azimuth in the weapon's mils.
    pub azimuth_mils: f64,
    /// Elevation in the weapon's convention.
    pub elevation_mils: Option<f64>,
    /// The charge this gun fires (`smallint`).
    pub charge_rings: Option<u32>,
    /// Seconds to splash.
    pub time_of_flight_s: Option<f64>,
}

/// `POST /api/v1/fire-missions` answer: the server's solution and the stored mission. The mortar
/// page decodes the save with this type and the golden round trip holds the same type.
/// @contract fire-mission.schema.json#/definitions/SavedFireMission
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedFireMissionAnswer {
    /// The firing solution the API computed for the saved fire mission.
    pub solution: FireMissionSolution,
    /// The stored fire mission.
    pub fire_mission: SavedFire,
}
