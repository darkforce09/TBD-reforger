//! The fleet procedure (`staging_fleet`): the waves of fleet commands on the five staging
//! instances and the effects each must show.
//!
//! **Role:** the [`StagingProcedure`] of the fleet check: its action and recovery lists, its
//! preconditions, its step table and declared cases, its fixture identities and the observations
//! its receipt carries.
//!
//! **Position:** selected by `dispatch.rs` for `staging fleet --record`,
//! `staging action-list fleet` and `staging preflight`; runs on `procedure_runner/`; the waves
//! are in `waves/`, the reads in `fleet_reads.rs` and `single_server_reads.rs`, the receipt's
//! observations in `judge_mapping.rs`.
//!
//! **Signals & state:** none; the procedure is a stateless value.
//!
//! **Invariants:** the run stops itself at [`FLEET_HARD_STOP_SECONDS`], inside the check's
//! 7,200 s timeout; every declared case is decided by at least one wave's effect; the
//! observations cite the manifest written beside the receipt, whatever the verdict.

mod fixture_identities;
mod fleet_cases;
mod fleet_reads;
mod judge_mapping;
mod operator_lists;
mod single_server_reads;
mod waves;

use anyhow::Result;
use serde_json::Value;

use crate::commands::staging::operator_coordination::action_list::PlannedAction;
use crate::commands::staging::procedure_runner::procedure::{
    ProcedurePlan, ProcedureRun, StagingProcedure,
};
use crate::commands::staging::remote_observers::remote_command::HostCommandRunner;
use crate::commands::staging::staging_settings::StagingSettings;
use crate::commands::staging::support_commands::preflight::PreflightCheck;
use crate::verifications::api_readiness::operational_recording::{
    FixtureManifest, Observations, StagingCheck,
};

/// Seconds after the run's start at which the fleet run stops itself.
pub(crate) const FLEET_HARD_STOP_SECONDS: u64 = 6_900;

/// Seconds between two polls of a pending fleet probe.
pub(crate) const FLEET_POLL_INTERVAL_SECONDS: u64 = 5;

/// The fleet procedure.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct FleetProcedure;

impl StagingProcedure for FleetProcedure {
    fn check(&self) -> StagingCheck {
        StagingCheck::Fleet
    }

    fn action_list(&self, settings: &StagingSettings) -> Vec<PlannedAction> {
        operator_lists::action_list(settings)
    }

    fn recovery_action_list(&self, settings: &StagingSettings) -> Vec<PlannedAction> {
        operator_lists::recovery_action_list(settings)
    }

    fn preflight_checks(&self, settings: &StagingSettings) -> Vec<PreflightCheck> {
        fixture_identities::preflight_checks(settings)
    }

    fn plan(&self, settings: &StagingSettings) -> Result<ProcedurePlan> {
        let targets = waves::WaveTargets::from_settings(settings)?;
        Ok(ProcedurePlan {
            declared_cases: fleet_cases::declared_cases()?,
            steps: waves::steps(&targets, settings)?,
            hard_stop_seconds: FLEET_HARD_STOP_SECONDS,
            poll_interval_seconds: FLEET_POLL_INTERVAL_SECONDS,
        })
    }

    fn fixture_identities(
        &self,
        settings: &StagingSettings,
        host: &mut dyn HostCommandRunner,
    ) -> Result<Value> {
        fixture_identities::read(settings, host)
    }

    fn observations(&self, run: &ProcedureRun, manifest: &FixtureManifest) -> Observations {
        judge_mapping::fleet_observations(run, manifest)
    }
}

#[cfg(test)]
#[path = "tests/recorded_fleet.rs"]
mod recorded_fleet;

#[cfg(test)]
#[path = "tests/fleet_waves.rs"]
mod fleet_waves_tests;

#[cfg(test)]
#[path = "tests/fleet_receipt.rs"]
mod fleet_receipt_tests;

#[cfg(test)]
#[path = "tests/recorded_single_server.rs"]
mod recorded_single_server;

#[cfg(test)]
#[path = "tests/single_server_waves.rs"]
mod single_server_waves_tests;
