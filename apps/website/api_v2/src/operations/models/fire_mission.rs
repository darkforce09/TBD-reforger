//! The saved mortar firing solution model.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::core::wire_format::rfc3339_utc;

/// Saved mortar firing solution from the Mortar Calculator.
///
/// # The seven `Option` fields, and why every one of them is an `Option`
///
/// Migration `0020_fire_missions_solution.sql` adds the four coordinates
/// [`website_map_engine::data::scenario::ballistics::solve_fire_mission`] is given and the three
/// numbers it computes that the table had nowhere to put. Every one is **nullable with no
/// default**, so every one is an `Option` here.
///
/// `None` means exactly one thing and it is true: **this row predates migration `0020`.** It is
/// not "zero", and the distinction is the whole point — a `0.0` time of flight is a plausible,
/// wrong, unfalsifiable number, and `charge` 0 is a real ring on every tube in the charge table.
/// Typing these as `f64`/`i64` with `#[serde(default)]` puts exactly those fabrications on the
/// wire for every fire mission saved before the migration ran.
///
/// They serialise **unconditionally**, as `null` rather than as an absent key (unlike `event_id`,
/// whose absence is a real state). A reader that receives `"time_of_flight_s": null` has been told
/// the field exists and this row has none; a reader that receives nothing cannot tell that from a
/// server too old to have the column.
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
    /// Firing position, flat game-world metres (`double precision`). `None` on rows written before
    /// migration `0020`.
    #[serde(default)]
    pub fp_x: Option<f64>,
    #[serde(default)]
    pub fp_y: Option<f64>,
    /// Target, flat game-world metres (`double precision`). `None` on rows written before
    /// migration `0020`.
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
    #[serde(with = "rfc3339_utc")]
    pub created_at: DateTime<Utc>,
}
