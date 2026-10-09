//! The load procedure (`staging_load`): the synthetic population, the measured member load from
//! five source addresses, and the game operations measured beside it.
//!
//! **Role:** the [`StagingProcedure`] of the load check (its action and recovery lists, its
//! preconditions, its plan of ten cases, its fixture identities, its run and its observations),
//! the `seed-load` and `clean-load` actions, and the local rehearsal that records nothing.
//!
//! **Position:** selected by `staging_dispatch.rs` for `staging load --record --token-file <path>`,
//! `staging load --rehearse-local`, `staging seed-load`, `staging clean-load`,
//! `staging action-list load` and `staging preflight`; plans with `procedure_runner/`, runs its
//! own steps in `load_run.rs`, and runs the load engine (`tools/staging/staging_load_generator/`)
//! as the `staging-load` child process through `workstation_load.rs`.
//!
//! **Signals & state:** none held across commands; the procedure carries its token file, the
//! settings and repository root of a recorded run, and the workstation seam.
//!
//! **Invariants:** the committed workload and population are read from the repository the run
//! binds to; a recorded run needs its token file and settings or it stops, and its receipt then
//! fails naming why; the observations carry what the engine measured, or zeros and an
//! unreachable p95 when it measured nothing.

mod committed_load_data;
mod game_operations;
mod load_action_lists;
mod load_fixture_orchestration;
mod load_preconditions;
mod load_queries;
mod load_report_judges;
mod load_run;
mod load_steps;
mod local_rehearsal;
mod workstation_load;

#[cfg(test)]
#[path = "tests/load_test_support.rs"]
mod load_test_support;

#[cfg(test)]
#[path = "tests/load_data_and_judges.rs"]
mod load_data_and_judges_tests;

#[cfg(test)]
#[path = "tests/load_run.rs"]
mod load_run_tests;

#[cfg(test)]
#[path = "tests/local_rehearsal.rs"]
mod local_rehearsal_tests;

#[cfg(test)]
#[path = "tests/workstation_load.rs"]
mod workstation_load_tests;

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::error::{Result, ResultExt};
use serde_json::{Value, json};
use staging_load_plan::LoadReport;

use self::committed_load_data::{CommittedLoadData, POPULATION_FILE, WORKLOAD_FILE};
use self::load_queries::{FLEET_HEARTBEAT_CENSUS, STAGING_MISSION, heartbeat_census, mission_ids};
use self::load_report_judges::REPORT_MEASUREMENT;
use self::load_run::{HARDWARE_MEASUREMENT, LoadRunInputs, NETWORK_MEASUREMENT, run_load};
use self::workstation_load::{LiveWorkstation, WorkstationLoad};
use crate::operator_coordination::action_list::PlannedAction;
use crate::procedure_runner::procedure::{ProcedurePlan, ProcedureRun, StagingProcedure};
use crate::procedure_runner::runner::RunContext;
use crate::remote_observers::database_reader::select;
use crate::remote_observers::remote_command::HostCommandRunner;
use crate::staging_command::PlanOnly;
use crate::staging_settings::StagingSettings;
use crate::support_commands::preflight::PreflightCheck;
use api_readiness_checks::operational_recording::{FixtureManifest, Observations, StagingCheck};
use repository_root::find_repository_root;

/// The load procedure: the token file, the settings and root of a recorded run, and the
/// workstation that sends its requests.
#[derive(Clone)]
pub(crate) struct LoadProcedure {
    /// The refresh token file a recorded run reads once.
    pub token_file: Option<PathBuf>,
    /// The checkout whose committed load data the procedure reads; the working tree's when
    /// absent.
    pub repository_root: Option<PathBuf>,
    /// The settings of a recorded run.
    pub settings: Option<StagingSettings>,
    /// The keying refreshes and the member load.
    pub workstation: Arc<dyn WorkstationLoad>,
}

impl Default for LoadProcedure {
    fn default() -> Self {
        Self {
            token_file: None,
            repository_root: None,
            settings: None,
            workstation: Arc::new(LiveWorkstation),
        }
    }
}

impl LoadProcedure {
    /// The procedure of `staging load --record --token-file <path>`.
    pub(crate) fn recording(
        token_file: Option<PathBuf>,
        root: &Path,
        settings: &StagingSettings,
    ) -> Self {
        Self {
            token_file,
            repository_root: Some(root.to_path_buf()),
            settings: Some(settings.clone()),
            ..Self::default()
        }
    }

    fn data(&self) -> Result<CommittedLoadData> {
        let root = match &self.repository_root {
            Some(root) => root.clone(),
            None => find_repository_root()?,
        };
        CommittedLoadData::read(&root)
    }
}

impl StagingProcedure for LoadProcedure {
    fn check(&self) -> StagingCheck {
        StagingCheck::Load
    }

    fn action_list(&self, settings: &StagingSettings) -> Vec<PlannedAction> {
        load_action_lists::action_list(
            settings,
            self.data().ok().as_ref(),
            self.token_file.as_deref(),
        )
    }

    fn recovery_action_list(&self, _settings: &StagingSettings) -> Vec<PlannedAction> {
        load_action_lists::recovery_action_list(self.token_file.as_deref())
    }

    fn preflight_checks(&self, settings: &StagingSettings) -> Vec<PreflightCheck> {
        let addresses = self
            .data()
            .ok()
            .map(|data| data.workload.source_address_count);
        load_action_lists::preflight_checks(settings, addresses, &self.workstation)
    }

    fn plan(&self, settings: &StagingSettings) -> Result<ProcedurePlan> {
        load_steps::load_plan(&self.data()?, settings)
    }

    fn fixture_identities(
        &self,
        settings: &StagingSettings,
        host: &mut dyn HostCommandRunner,
    ) -> Result<Value> {
        let data = self.data()?;
        let title = data.population.fixture_events.mission_title.clone();
        let mission = host.run(&select(
            &settings.database_container,
            &STAGING_MISSION,
            &[("title", title.clone())],
        )?)?;
        let census = host.run(&select(
            &settings.database_container,
            &FLEET_HEARTBEAT_CENSUS,
            &[],
        )?)?;
        let servers: Vec<Value> = heartbeat_census(&census.stdout)
            .iter()
            .map(|row| json!({ "name": row.server_name, "server_id": row.server_id }))
            .collect();
        Ok(json!({
            "workload_sha256": data.workload_sha256,
            "committed_files": [WORKLOAD_FILE, POPULATION_FILE],
            "population": data.population,
            "target_origin": settings.load_target_origin,
            "source_addresses": settings.load_source_addresses,
            "staging_mission": { "title": title, "ids": mission_ids(&mission.stdout), "read_exit": mission.exit_code },
            "fleet_servers": servers,
        }))
    }

    fn run<'a>(&self, plan: &'a ProcedurePlan, context: RunContext<'a>) -> Result<ProcedureRun> {
        let settings = self
            .settings
            .as_ref()
            .context("a recorded load run needs the staging settings")?;
        let token_file = self
            .token_file
            .as_deref()
            .context("a recorded load run needs --token-file")?;
        let data = self.data()?;
        let inputs = LoadRunInputs {
            data: &data,
            settings,
            token_file,
            workstation: &self.workstation,
        };
        run_load(plan, context, &inputs)
    }

    fn observations(&self, run: &ProcedureRun, _manifest: &FixtureManifest) -> Observations {
        let report: Option<LoadReport> = run
            .measurements
            .get(REPORT_MEASUREMENT)
            .and_then(|value| serde_json::from_value(value.clone()).ok());
        let text = |key: &str| {
            run.measurements
                .get(key)
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string()
        };
        let workload_sha256 = self
            .data()
            .map(|data| data.workload_sha256)
            .unwrap_or_default();
        match report {
            Some(report) => Observations::Load {
                duration_seconds: report.measured_seconds as u64,
                member_accounts: report.member_accounts,
                minimum_concurrent_clients: u64::from(report.minimum_concurrent_clients),
                completed_requests: report.completed_requests,
                unexpected_errors: report.unexpected_errors.total,
                json_read_count: report.json_reads.completed,
                json_write_count: report.json_writes.completed,
                p95_json_read_ms: report.json_reads.p95_milliseconds.unwrap_or(f64::MAX),
                p95_json_write_ms: report.json_writes.p95_milliseconds.unwrap_or(f64::MAX),
                workload_sha256,
                hardware: text(HARDWARE_MEASUREMENT),
                network: text(NETWORK_MEASUREMENT),
            },
            None => Observations::Load {
                duration_seconds: 0,
                member_accounts: 0,
                minimum_concurrent_clients: 0,
                completed_requests: 0,
                unexpected_errors: 0,
                json_read_count: 0,
                json_write_count: 0,
                p95_json_read_ms: f64::MAX,
                p95_json_write_ms: f64::MAX,
                workload_sha256,
                hardware: text(HARDWARE_MEASUREMENT),
                network: text(NETWORK_MEASUREMENT),
            },
        }
    }
}

/// `staging seed-load --token-file <path>`.
pub(crate) fn seed_load(
    root: &Path,
    settings: &StagingSettings,
    token_file: &Path,
    plan: &PlanOnly,
    host: &mut dyn HostCommandRunner,
    output: &mut dyn Write,
) -> Result<u8> {
    let data = CommittedLoadData::read(root)?;
    load_fixture_orchestration::seed_load(settings, &data, token_file, plan, host, output)
}

/// `staging clean-load`.
pub(crate) fn clean_load(
    settings: &StagingSettings,
    plan: &PlanOnly,
    host: &mut dyn HostCommandRunner,
    output: &mut dyn Write,
) -> Result<u8> {
    load_fixture_orchestration::clean_load(settings, plan, host, output)
}

/// `staging load --rehearse-local`: the load path against the local stack, recording nothing.
pub(crate) fn rehearse_local(root: &Path) -> Result<u8> {
    let data = CommittedLoadData::read(root)?;
    let mut stack = local_rehearsal::LocalStack::new(root);
    local_rehearsal::rehearse(&mut stack, &data, &mut std::io::stdout())
}
