//! The staging fleet instance a debug command's `--instance N` names.
//!
//! **Role:** selects instance N of the fleet `cargo xtask deploy staging` deploys, as that
//! command's own [`FleetInstance`], and names why `--instance` can name no instance. Ports, folders
//! and unit names come from [`crate::commands::deploy::staging::fleet_instances`], which owns those
//! rules.
//!
//! **Position:** read by `mod remote-logs` ([`crate::commands::debug::remote_logs`]) and
//! `debug direct-join` ([`crate::commands::debug::direct_join`]); the settings come from
//! `deploy.env` through [`crate::core::deploy_environment`] and are parsed by the deploy's
//! [`FleetSettings`].
//!
//! **Signals & state:** none; pure values.
//!
//! **Invariants:** a fleet the deploy refuses (a malformed `TBD_FLEET_*` setting, an agent origin
//! it does not accept, a broken port rule) is refused here too, so a debug command never reads a
//! port or folder the deploy would not have used; N outside 1 to `TBD_FLEET_INSTANCES` is its own
//! refusal and takes precedence over a broken port rule; the profile a debug command reads is the
//! `-profile` folder of the instance's game server unit, `profile/` in the instance's folder.

use std::fmt;

use crate::commands::deploy::staging::fleet_instances::{
    FLEET_ROOT_UNDER_HOME, FleetInstance, FleetSettings, InstanceFolder,
};
use crate::core::deploy_environment::DeployEnvironment;

/// Why `--instance N` names no instance.
#[derive(Debug)]
pub(crate) enum InstanceSelectionError {
    /// The deploy file's fleet settings are refused, the way `cargo xtask deploy staging` refuses
    /// them; the text names the setting or the port rule.
    Setting(String),
    /// N is outside 1 to the fleet's instance count.
    OutOfRange {
        /// The number `--instance` gave.
        number: u16,
        /// `TBD_FLEET_INSTANCES`.
        count: u16,
    },
}

impl fmt::Display for InstanceSelectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Setting(problem) => write!(formatter, "{problem}"),
            Self::OutOfRange { number, count } => write!(
                formatter,
                "--instance {number} names no instance; the fleet runs instances 1 to {count}"
            ),
        }
    }
}

/// Instance `number` of the fleet `environment` describes, judged by the deploy's
/// [`FleetSettings::from_environment`] and [`FleetSettings::check_port_rules`].
pub(crate) fn select_fleet_instance(
    environment: &DeployEnvironment,
    number: u16,
) -> Result<FleetInstance, InstanceSelectionError> {
    let fleet = FleetSettings::from_environment(environment)
        .map_err(|error| InstanceSelectionError::Setting(error.to_string()))?;
    let instance = fleet
        .instances()
        .into_iter()
        .find(|instance| instance.number == number)
        .ok_or(InstanceSelectionError::OutOfRange {
            number,
            count: fleet.instance_count,
        })?;
    fleet.check_port_rules().map_err(|problem| {
        InstanceSelectionError::Setting(format!("{}: {problem}", environment.path().display()))
    })?;
    Ok(instance)
}

/// The instance's `-profile` folder relative to the deploy user's home:
/// `tbd/fleet/instance-N/profile` ([`InstanceFolder::profile`]).
pub(crate) fn profile_under_home(instance: &FleetInstance) -> String {
    InstanceFolder::under(FLEET_ROOT_UNDER_HOME, instance.number).profile()
}

#[cfg(test)]
#[path = "tests/staging_fleet_instance/tests.rs"]
mod tests;
