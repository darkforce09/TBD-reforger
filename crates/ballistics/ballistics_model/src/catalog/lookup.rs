//! Typed lookups into a ballistics catalog and the flight parameters of one firing.
//!
//! **Role:** finds a weapon, a shell and a charge by identifier, refuses unknown or mismatched
//! combinations with a typed [`CatalogLookupError`], and derives the flight model inputs of one
//! weapon + shell + charge.
//!
//! **Position:** the `ballistics_model` crate's `catalog` module; the firing solver, calibration, dispersion
//! and fuze computations call [`BallisticsCatalog::resolve_firing`] and hand its
//! [`ResolvedFiring`] to [`crate::flight_model`].
//!
//! **Signals & state:** none; borrows the catalog read-only.
//!
//! **Invariants:**
//! - Refusal order is fixed: unknown weapon, then unknown shell, then a shell the weapon does not
//!   fire, then a ring count the shell does not accept.
//! - Muzzle speed is `init_speed_m_s × charge init_speed_coef × weapon muzzle_init_speed_coef`,
//!   multiplied in that order.
//! - [`FlightParameters`] take the catalog gravity, the shell's mass, air drag, wind influence
//!   and lifetime, and [`DEFAULT_INTEGRATION_STEP_S`]; the side air-drag scale never enters them
//!   because the flight model's drag is isotropic.

use thiserror::Error;

use super::{BallisticsCatalog, Charge, Shell, WeaponSystem};
use crate::flight_model::{DEFAULT_INTEGRATION_STEP_S, FlightParameters};
use crate::ids::{ShellId, WeaponId};

/// Why a catalog has no firing for the requested weapon, shell and charge.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CatalogLookupError {
    /// No weapon of the catalog has this identifier.
    #[error("unknown weapon `{weapon_id}`")]
    UnknownWeapon {
        /// The identifier that was asked for.
        weapon_id: WeaponId,
    },
    /// No shell of the catalog has this identifier.
    #[error("unknown shell `{shell_id}`")]
    UnknownShell {
        /// The identifier that was asked for.
        shell_id: ShellId,
    },
    /// The shell exists but the weapon does not fire it.
    #[error("weapon `{weapon_id}` does not fire shell `{shell_id}`")]
    IncompatibleShell {
        /// The weapon that was asked for.
        weapon_id: WeaponId,
        /// The shell it does not fire.
        shell_id: ShellId,
    },
    /// The shell accepts no charge with this ring count.
    #[error("shell `{shell_id}` has no charge of {rings} rings")]
    UnknownRing {
        /// The shell that was asked for.
        shell_id: ShellId,
        /// The ring count it does not accept.
        rings: u32,
    },
}

/// One weapon firing one shell at one charge, with the flight model inputs it implies.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResolvedFiring<'catalog> {
    /// The launcher.
    pub weapon: &'catalog WeaponSystem,
    /// The shell.
    pub shell: &'catalog Shell,
    /// The charge.
    pub charge: &'catalog Charge,
    /// The shell's physical constants in the catalog's world.
    pub flight_parameters: FlightParameters,
    /// Muzzle speed in metres per second.
    pub muzzle_speed_m_s: f64,
}

impl BallisticsCatalog {
    /// The weapon with identifier `weapon_id`.
    ///
    /// # Errors
    ///
    /// [`CatalogLookupError::UnknownWeapon`].
    pub fn weapon(&self, weapon_id: &WeaponId) -> Result<&WeaponSystem, CatalogLookupError> {
        self.weapons
            .iter()
            .find(|weapon| weapon.weapon_id == *weapon_id)
            .ok_or_else(|| CatalogLookupError::UnknownWeapon {
                weapon_id: weapon_id.clone(),
            })
    }

    /// The shell with identifier `shell_id`.
    ///
    /// # Errors
    ///
    /// [`CatalogLookupError::UnknownShell`].
    pub fn shell(&self, shell_id: &ShellId) -> Result<&Shell, CatalogLookupError> {
        self.shells
            .iter()
            .find(|shell| shell.shell_id == *shell_id)
            .ok_or_else(|| CatalogLookupError::UnknownShell {
                shell_id: shell_id.clone(),
            })
    }

    /// The weapon and a shell it fires.
    ///
    /// # Errors
    ///
    /// [`CatalogLookupError::UnknownWeapon`], [`CatalogLookupError::UnknownShell`] or
    /// [`CatalogLookupError::IncompatibleShell`], in that order.
    pub fn weapon_and_shell(
        &self,
        weapon_id: &WeaponId,
        shell_id: &ShellId,
    ) -> Result<(&WeaponSystem, &Shell), CatalogLookupError> {
        let weapon = self.weapon(weapon_id)?;
        let shell = self.shell(shell_id)?;
        if !weapon.fires_shell(shell_id) {
            return Err(CatalogLookupError::IncompatibleShell {
                weapon_id: weapon_id.clone(),
                shell_id: shell_id.clone(),
            });
        }
        Ok((weapon, shell))
    }

    /// The weapon firing the shell at the charge of `rings` rings, with its flight parameters
    /// and muzzle speed.
    ///
    /// # Errors
    ///
    /// Any refusal of [`Self::weapon_and_shell`], then [`CatalogLookupError::UnknownRing`].
    pub fn resolve_firing(
        &self,
        weapon_id: &WeaponId,
        shell_id: &ShellId,
        rings: u32,
    ) -> Result<ResolvedFiring<'_>, CatalogLookupError> {
        let (weapon, shell) = self.weapon_and_shell(weapon_id, shell_id)?;
        let charge = shell
            .charge(rings)
            .ok_or_else(|| CatalogLookupError::UnknownRing {
                shell_id: shell_id.clone(),
                rings,
            })?;
        Ok(ResolvedFiring {
            weapon,
            shell,
            charge,
            flight_parameters: flight_parameters(self.gravity_m_s2, shell),
            muzzle_speed_m_s: muzzle_speed_m_s(weapon, shell, charge),
        })
    }
}

/// The flight model inputs of `shell` under `gravity_m_s2`, stepped at
/// [`DEFAULT_INTEGRATION_STEP_S`]. [`Shell::side_air_drag_scale`] has no field here.
pub fn flight_parameters(gravity_m_s2: f64, shell: &Shell) -> FlightParameters {
    FlightParameters {
        gravity_m_s2,
        mass_kg: shell.mass_kg,
        air_drag: shell.air_drag,
        wind_influence_multiplier: shell.wind_influence_multiplier,
        time_to_live_s: shell.time_to_live_s,
        integration_step_s: DEFAULT_INTEGRATION_STEP_S,
    }
}

/// Muzzle speed in metres per second of `shell` fired by `weapon` at `charge`.
pub fn muzzle_speed_m_s(weapon: &WeaponSystem, shell: &Shell, charge: &Charge) -> f64 {
    shell.init_speed_m_s * charge.init_speed_coef * weapon.muzzle_init_speed_coef
}

#[cfg(test)]
#[path = "tests/lookup.rs"]
mod tests;
