//! A launcher of the ballistics catalog.
//!
//! @contract ballistics-catalog.schema.json#/definitions/WeaponSystem

use serde::{Deserialize, Serialize};

use crate::ids::{ShellId, WeaponId};

use crate::angular_units::{AngularUnitsError, MilsConvention, degrees_to_radians};

/// One launcher: its sight convention, elevation limits, muzzle speed factor, muzzle dispersion
/// and the shells it fires.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WeaponSystem {
    /// Lowercase slug naming the launcher inside its catalog.
    pub weapon_id: WeaponId,
    /// Human-readable launcher name.
    pub display_name: String,
    /// Enfusion GUID of the launcher prefab: 16 uppercase hexadecimal digits.
    pub prefab_guid: String,
    /// Bore diameter in millimetres.
    pub caliber_mm: f64,
    /// Mils in a full circle in this launcher's sight convention.
    pub mils_per_circle: u32,
    /// Lowest elevation the launcher lays, degrees above the horizontal.
    pub elevation_min_deg: f64,
    /// Highest elevation the launcher lays, degrees above the horizontal.
    pub elevation_max_deg: f64,
    /// Launcher factor on a shell's muzzle speed.
    pub muzzle_init_speed_coef: f64,
    /// Diameter in metres of the uniform dispersion disc at [`Self::dispersion_range_m`].
    pub dispersion_diameter_m: f64,
    /// Range in metres at which [`Self::dispersion_diameter_m`] is measured.
    pub dispersion_range_m: f64,
    /// Slugs of the catalog shells this launcher fires.
    pub shell_ids: Vec<ShellId>,
}

impl WeaponSystem {
    /// The launcher's sight convention.
    ///
    /// # Errors
    ///
    /// [`AngularUnitsError::ZeroMilsPerCircle`] when the catalog declares zero mils.
    pub fn mils_convention(&self) -> Result<MilsConvention, AngularUnitsError> {
        MilsConvention::new(self.mils_per_circle)
    }

    /// Whether `shell_id` is one of the shells this launcher fires.
    pub fn fires_shell(&self, shell_id: &ShellId) -> bool {
        self.shell_ids.iter().any(|listed| listed == shell_id)
    }

    /// The elevation limits `(min, max)` in radians.
    pub fn elevation_limits_rad(&self) -> (f64, f64) {
        (
            degrees_to_radians(self.elevation_min_deg),
            degrees_to_radians(self.elevation_max_deg),
        )
    }
}
