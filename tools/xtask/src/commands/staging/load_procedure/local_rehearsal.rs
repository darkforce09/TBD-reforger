//! `staging load --rehearse-local`: the load path against the local stack, recording nothing.
//!
//! **Role:** seeds the committed population and fixture events into the local database with the
//! host tool, sends the keying refreshes from 127.0.0.2 to 127.0.0.6, checks their strict
//! buckets, runs the committed workload over a shortened window against the local API, judges
//! the member-load cases on its report against rehearsal thresholds scaled to that window, and
//! cleans the fixtures and the account file whatever happened.
//!
//! **Position:** called by `dispatch.rs`; reaches the local stack through a
//! [`RehearsalEnvironment`] ([`LocalStack`] live, a stub in the tests), and shares the judges,
//! the queries and the workstation seam with the recorded run.
//!
//! **Signals & state:** a private scratch folder under `target/staging/load/` holding the account
//! file for the rehearsal's length.
//!
//! **Invariants:** nothing is written under `target/api-readiness/`; the cleanup runs after any
//! seeding step ran; the account file is created by the host tool in a mode-700 folder and
//! removed with it; the ramp is the workload's sign-in minimum, so every client joins the member
//! load before the measured window opens; `concurrency` and `member_accounts` are judged against
//! rehearsal thresholds (every client, and the accounts the clients reach in the short window),
//! printed as such, while the recorded run keeps the acceptance; the game-operation cases are not
//! rehearsed, since no fleet runs locally.

use std::io::Write;
use std::net::{IpAddr, Ipv4Addr};
use std::os::unix::fs::DirBuilderExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, anyhow, ensure};
use developer_tools::staging_verification::load_generation::{
    FixtureEvent, LoadRunPlan, WorkloadPlan, reachable_member_accounts,
};
use process_runner::Run;

use super::committed_load_data::CommittedLoadData;
use super::load_preconditions::{keying_answers_hold, send_keying_refreshes, strict_buckets_hold};
use super::load_queries::{
    LOAD_FIXTURE_EVENTS, STRICT_RATE_LIMIT_BUCKETS, fixture_events, mission_ids,
};
use super::load_report_judges::{
    LoadThresholds, concurrency, member_accounts, p95_json_reads, p95_json_writes, refresh_pacing,
    sustained_rate, zero_unexpected_errors,
};
use super::workstation_load::{LiveWorkstation, WorkstationLoad};
use crate::commands::staging::remote_observers::database_reader::{
    CommittedQuery, READ_ONLY_SESSION_OPTIONS, validate,
};
use crate::commands::staging::remote_observers::remote_command::CommandOutput;
use crate::commands::staging::staging_settings::{STAGING_DATABASE, STAGING_DATABASE_USER};

/// The local API the rehearsal loads.
pub(crate) const REHEARSAL_ORIGIN: &str = "http://127.0.0.1:8080";
/// The measured window of a rehearsal, seconds; its ramp is the workload's sign-in minimum.
pub(crate) const REHEARSAL_MEASURED_SECONDS: f64 = 60.0;
/// The local database container of `cargo xtask db up`.
pub(crate) const LOCAL_DATABASE_CONTAINER: &str = "tbd_reforger_db";
/// The oldest live mission of the local database, which the rehearsal's fixture events attach.
pub(crate) const LOCAL_LIVE_MISSION: CommittedQuery = CommittedQuery {
    name: "rehearsal_live_mission",
    sql: "SELECT id FROM missions WHERE status = 'live' AND deleted_at IS NULL \
          ORDER BY created_at, id LIMIT 1",
    parameters: &[],
};

/// The loopback source addresses 127.0.0.2 to 127.0.0.6.
pub(crate) fn rehearsal_addresses() -> Vec<IpAddr> {
    (2..=6)
        .map(|last| IpAddr::V4(Ipv4Addr::new(127, 0, 0, last)))
        .collect()
}

/// What the rehearsal reaches on this workstation.
pub(crate) trait RehearsalEnvironment {
    /// Runs `staging-fixtures <arguments>` against the local database.
    fn host_tool(&mut self, arguments: &[String]) -> Result<CommandOutput>;
    /// Runs a committed read against the local database and returns its rows' text.
    fn read(&mut self, query: &CommittedQuery) -> Result<String>;
    /// The keying refresh and the member load.
    fn workstation(&self) -> Arc<dyn WorkstationLoad>;
    /// A new private folder for the account file.
    fn scratch_folder(&mut self) -> Result<PathBuf>;
}

/// Rehearses the load path; returns 0 when every rehearsed check held.
pub(crate) fn rehearse(
    environment: &mut dyn RehearsalEnvironment,
    data: &CommittedLoadData,
    output: &mut dyn Write,
) -> Result<u8> {
    writeln!(
        output,
        "rehearsal: the load path against {REHEARSAL_ORIGIN}; records nothing"
    )?;
    let scratch = environment.scratch_folder()?;
    let account_file = scratch.join("load-accounts.json");
    let outcome = seed_and_load(environment, data, &account_file, output);
    let cleaned = clean(environment, output);
    let removed = std::fs::remove_dir_all(&scratch);
    let passed = match outcome {
        Ok(passed) => passed,
        Err(error) => {
            writeln!(output, "rehearsal: stopped: {error:#}")?;
            false
        }
    };
    writeln!(
        output,
        "rehearsal: game_servers_heartbeating and game_operations_measured are not rehearsed: no fleet runs locally"
    )?;
    let cleaned = cleaned? && removed.is_ok();
    writeln!(
        output,
        "rehearsal: {}",
        if passed && cleaned { "PASS" } else { "FAIL" }
    )?;
    Ok(u8::from(!(passed && cleaned)))
}

fn seed_and_load(
    environment: &mut dyn RehearsalEnvironment,
    data: &CommittedLoadData,
    account_file: &Path,
    output: &mut dyn Write,
) -> Result<bool> {
    let missions = mission_ids(&environment.read(&LOCAL_LIVE_MISSION)?);
    let mission = missions
        .first()
        .context("the local database holds no live mission for the fixture events")?;
    let population = &data.population;
    let seeding = [
        vec![
            "seed-load-population".to_string(),
            "--accounts".into(),
            population.accounts.to_string(),
            "--role".into(),
            population.discord_role.clone(),
            "--account-file".into(),
            account_file.display().to_string(),
            "--id-base".into(),
            population.id_base.clone(),
        ],
        vec![
            "seed-load-fixture-events".to_string(),
            "--mission".into(),
            mission.clone(),
        ],
    ];
    for arguments in seeding {
        ensure!(
            run_host_tool(environment, arguments, output)?,
            "the local seeding failed"
        );
    }
    let events: Vec<FixtureEvent> = fixture_events(&environment.read(&LOAD_FIXTURE_EVENTS)?)
        .into_iter()
        .map(|event| FixtureEvent {
            event_id: event.event_id,
            event_mission_id: event.event_mission_id,
            mission_id: event.mission_id,
            slot_ids: event.slot_ids,
        })
        .collect();
    ensure!(
        events.len() == population.fixture_events.count as usize,
        "the local database holds {} fixture events, not {}",
        events.len(),
        population.fixture_events.count
    );
    let addresses = rehearsal_addresses();
    let workstation = environment.workstation();
    let mut passed = true;
    let keying = keying_answers_hold(
        &send_keying_refreshes(&workstation, REHEARSAL_ORIGIN, &addresses),
        &addresses,
    );
    passed &= verdict(output, "keying refreshes", keying)?;
    let buckets = strict_buckets_hold(
        &environment.read(&STRICT_RATE_LIMIT_BUCKETS)?,
        &addresses,
        0,
    );
    passed &= verdict(output, "strict buckets", buckets)?;
    let workload = rehearsal_workload(&data.workload);
    let limits = rehearsal_thresholds(&workload)?;
    writeln!(
        output,
        "rehearsal: member load for a {} s ramp (the shortest that keeps {} clients signing in \
         from {} addresses at 80 % of the auth ceiling) and {} measured s",
        workload.ramp_seconds,
        workload.clients,
        workload.source_address_count,
        workload.measured_seconds
    )?;
    writeln!(
        output,
        "rehearsal: rehearsal thresholds for this window, not the acceptance: concurrency at least \
         {} clients, member_accounts at least {} (the accounts {} clients reach in {} s); the \
         recorded run keeps the acceptance of {} clients and {} member accounts",
        limits.concurrent_clients,
        limits.member_accounts,
        workload.clients,
        workload.ramp_seconds + workload.measured_seconds,
        LoadThresholds::ACCEPTANCE.concurrent_clients,
        LoadThresholds::ACCEPTANCE.member_accounts
    )?;
    let plan = LoadRunPlan {
        workload,
        target_origin: REHEARSAL_ORIGIN.to_string(),
        source_addresses: addresses,
        account_file: account_file.to_path_buf(),
        fixture_events: events,
    };
    let report = workstation.member_load(&plan)?;
    let judges: [(&str, super::load_report_judges::ReportJudge); 7] = [
        ("sustained_rate", sustained_rate),
        ("concurrency", concurrency),
        ("member_accounts", member_accounts),
        ("zero_unexpected_errors", zero_unexpected_errors),
        ("p95_json_reads", p95_json_reads),
        ("p95_json_writes", p95_json_writes),
        ("refresh_paced", refresh_pacing),
    ];
    for (case, judge) in judges {
        passed &= verdict(output, case, judge(&report, &limits))?;
    }
    Ok(passed)
}

/// The committed workload over the rehearsal's window: the measured window shortened to
/// [`REHEARSAL_MEASURED_SECONDS`] and the ramp set to the workload's sign-in minimum.
pub(crate) fn rehearsal_workload(committed: &WorkloadPlan) -> WorkloadPlan {
    let mut workload = committed.clone();
    workload.ramp_seconds = workload.minimum_ramp_seconds();
    workload.measured_seconds = REHEARSAL_MEASURED_SECONDS;
    workload
}

/// The thresholds a rehearsal of `workload` is judged on: the acceptance over its shortened
/// window, with `concurrency` scaled to its clients and `member_accounts` to the accounts they
/// reach while the window is open, each capped at the acceptance.
pub(crate) fn rehearsal_thresholds(workload: &WorkloadPlan) -> Result<LoadThresholds> {
    let reachable = reachable_member_accounts(workload)?;
    Ok(LoadThresholds {
        concurrent_clients: workload
            .clients
            .min(LoadThresholds::ACCEPTANCE.concurrent_clients),
        ..LoadThresholds::rehearsal(workload.measured_seconds, reachable)
    })
}

fn clean(environment: &mut dyn RehearsalEnvironment, output: &mut dyn Write) -> Result<bool> {
    let mut cleaned = true;
    for subcommand in ["clean-load-fixture-events", "clean-load-population"] {
        cleaned &= run_host_tool(environment, vec![subcommand.to_string()], output)?;
    }
    Ok(cleaned)
}

fn run_host_tool(
    environment: &mut dyn RehearsalEnvironment,
    mut arguments: Vec<String>,
    output: &mut dyn Write,
) -> Result<bool> {
    arguments.extend([
        "--confirm-database".to_string(),
        STAGING_DATABASE.to_string(),
        "--apply".to_string(),
    ]);
    let answer = environment.host_tool(&arguments)?;
    output.write_all(answer.stdout.as_bytes())?;
    if answer.exit_code != 0 {
        output.write_all(answer.stderr.as_bytes())?;
        writeln!(
            output,
            "rehearsal: staging-fixtures {} exited {}",
            arguments[0], answer.exit_code
        )?;
    }
    Ok(answer.exit_code == 0)
}

fn verdict(output: &mut dyn Write, check: &str, outcome: Result<String, String>) -> Result<bool> {
    match &outcome {
        Ok(summary) => writeln!(output, "rehearsal: {check} ... ok ({summary})")?,
        Err(why) => writeln!(output, "rehearsal: {check} ... FAILED ({why})")?,
    }
    Ok(outcome.is_ok())
}

/// The live local stack: the host tool through `cargo run` in the API crate, the database
/// through the local container, `curl` and the load engine.
pub(crate) struct LocalStack {
    root: PathBuf,
    runtime: String,
}

impl LocalStack {
    /// The local stack of the checkout at `root`, reached with `podman`, else `docker`.
    pub(crate) fn new(root: &Path) -> Self {
        let podman = Run::new("podman")
            .arg("--version")
            .timeout(Duration::from_secs(10))
            .output();
        let runtime = if podman.is_ok_and(|answer| answer.code == 0) {
            "podman"
        } else {
            "docker"
        };
        Self {
            root: root.to_path_buf(),
            runtime: runtime.to_string(),
        }
    }
}

impl RehearsalEnvironment for LocalStack {
    fn host_tool(&mut self, arguments: &[String]) -> Result<CommandOutput> {
        let mut words: Vec<String> = [
            "run",
            "--quiet",
            "--release",
            "-p",
            "api",
            "--bin",
            "staging-fixtures",
            "--",
        ]
        .iter()
        .map(ToString::to_string)
        .collect();
        words.extend(arguments.iter().cloned());
        let answer = Run::new("cargo")
            .args(&words)
            // From the checkout root, as on the staging host: the tool reads the API env file at
            // `apps/api/.env` relative to its working directory.
            .cwd(&self.root)
            .timeout(Duration::from_secs(1_800))
            .output()
            .map_err(|not_run| {
                anyhow!("staging-fixtures did not run through cargo: {not_run:?}")
            })?;
        Ok(CommandOutput {
            exit_code: answer.code,
            stdout: answer.stdout,
            stderr: answer.stderr,
        })
    }

    fn read(&mut self, query: &CommittedQuery) -> Result<String> {
        validate(query)?;
        ensure!(
            query.parameters.is_empty(),
            "the rehearsal binds no query parameter"
        );
        let answer = Run::new(&self.runtime)
            .args([
                "exec",
                "-i",
                "-e",
                READ_ONLY_SESSION_OPTIONS,
                LOCAL_DATABASE_CONTAINER,
                "psql",
                "-X",
                "-A",
                "-t",
                "-q",
                "-v",
                "ON_ERROR_STOP=1",
                "-U",
                STAGING_DATABASE_USER,
                "-d",
                STAGING_DATABASE,
            ])
            .stdin(format!("{};\n", query.sql))
            .timeout(Duration::from_secs(60))
            .output()
            .map_err(|not_run| {
                anyhow!(
                    "reading {} from the local database did not run: {not_run:?}",
                    query.name
                )
            })?;
        ensure!(
            answer.code == 0,
            "{} exited {}: {}",
            query.name,
            answer.code,
            answer.stderr.trim()
        );
        Ok(answer.stdout)
    }

    fn workstation(&self) -> Arc<dyn WorkstationLoad> {
        Arc::new(LiveWorkstation)
    }

    fn scratch_folder(&mut self) -> Result<PathBuf> {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| since.as_millis());
        let folder = self
            .root
            .join(format!("target/staging/load/rehearsal-{stamp}"));
        std::fs::create_dir_all(folder.parent().context("the scratch folder has a parent")?)?;
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(&folder)
            .with_context(|| format!("creating {}", folder.display()))?;
        Ok(folder)
    }
}
