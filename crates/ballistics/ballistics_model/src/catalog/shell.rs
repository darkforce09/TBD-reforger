//! A shell of the ballistics catalog, its charges and its time fuze.
//!
//! @contract ballistics-catalog.schema.json#/definitions/Shell
//! @contract ballistics-catalog.schema.json#/definitions/Charge
//! @contract ballistics-catalog.schema.json#/definitions/TimeFuze

use serde::{Deserialize, Serialize};

use crate::ids::ShellId;

/// One shell's flight parameters, as the game's shell movement component declares them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Shell {
    /// Lowercase slug naming the shell inside its catalog.
    pub shell_id: ShellId,
    /// Human-readable shell name.
    pub display_name: String,
    /// Enfusion GUID of the shell prefab: 16 uppercase hexadecimal digits.
    pub prefab_guid: String,
    /// What the shell does at the target.
    pub role: ShellRole,
    /// Base muzzle speed in metres per second before charge and launcher factors.
    pub init_speed_m_s: f64,
    /// Half-width of the uniform muzzle speed spread, in the game's unit.
    pub init_speed_variation: f64,
    /// Mass in kilograms of the shell movement component; the drag term divides by it.
    pub mass_kg: f64,
    /// Quadratic drag coefficient.
    pub air_drag: f64,
    /// The game's side air-drag scale. The flight model's drag is isotropic, so this value is
    /// carried for provenance and never enters
    /// [`crate::flight_model::FlightParameters`].
    pub side_air_drag_scale: f64,
    /// Share of the wind velocity the air-relative velocity subtracts.
    pub wind_influence_multiplier: f64,
    /// Factor on the launcher's dispersion disc.
    pub dispersion_multiplier: f64,
    /// Seconds the shell exists after firing.
    pub time_to_live_s: f64,
    /// The range card's standard dispersion in metres.
    pub standard_dispersion_m: f64,
    /// Every charge the shell accepts; rings are unique and exactly one is the default.
    pub charges: Vec<Charge>,
    /// The settable time fuze, when the shell carries one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time_fuze: Option<TimeFuze>,
}

/// What a shell does at the target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShellRole {
    /// High explosive.
    He,
    /// Smoke screen.
    Smoke,
    /// Illumination flare.
    Illumination,
    /// Inert practice round.
    Practice,
    /// Any other effect.
    Other,
}

/// One charge: its propellant ring count and its factor on the shell's muzzle speed.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Charge {
    /// Number of propellant rings.
    pub rings: u32,
    /// Factor this charge applies to [`Shell::init_speed_m_s`].
    pub init_speed_coef: f64,
    /// Whether the game loads this charge by default.
    pub is_default: bool,
}

/// A settable time fuze: its window and factory setting, in seconds after firing.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimeFuze {
    /// Shortest settable time.
    pub min_s: f64,
    /// Longest settable time.
    pub max_s: f64,
    /// Factory setting inside the window.
    pub default_s: f64,
}

impl Shell {
    /// The charge with `rings` propellant rings, if the shell accepts one.
    pub fn charge(&self, rings: u32) -> Option<&Charge> {
        self.charges.iter().find(|charge| charge.rings == rings)
    }

    /// The charge the game loads by default, if the catalog marks one.
    pub fn default_charge(&self) -> Option<&Charge> {
        self.charges.iter().find(|charge| charge.is_default)
    }
}
