//! What the fleet run binds to before its first step, and the fleet's own preflight checks.
//!
//! **Role:** reads the fixture identities the manifest holds (the five servers' ids, the two
//! staging missions with their artifact digests, the fleet scenario rows, the Workshop version)
//! and builds the read-only preconditions `staging preflight` checks for the fleet run.
//!
//! **Position:** called by `FleetProcedure::fixture_identities` (recorded runs) and
//! `FleetProcedure::preflight_checks` (`staging preflight`); reads through `fleet_reads.rs`,
//! `console_log_reader` and `build_identity`.
//!
//! **Signals & state:** none; reads through the given host runner.
//!
//! **Invariants:** only reads; a Workshop version that cannot be read is recorded as
//! `unavailable (<why>)`, never guessed; the preflight holds only when each of the five servers
//! is registered and active once, both staging missions are live with an artifact, both fleet
//! scenario rows exist, and every server's open runtime session heartbeated within
//! [`RESTING_HEARTBEAT_SECONDS`] and runs an Everon artifact, which W6's same-terrain deployment
//! needs.

use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use serde_json::Value;

use super::fleet_reads::{FLEET_IDENTITIES, FLEET_RESTING_SESSIONS, json_rows};
use super::waves::deployment_waves::{
    ARLAND_MISSION, ARLAND_TERRAIN, EVERON_MISSION, EVERON_TERRAIN,
};
use crate::commands::staging::environment_identity::build_identity;
use crate::commands::staging::remote_observers::console_log_reader;
use crate::commands::staging::remote_observers::database_reader;
use crate::commands::staging::remote_observers::remote_command::{
    CommandOutput, HostCommandRunner,
};
use crate::commands::staging::staging_settings::StagingSettings;
use crate::commands::staging::support_commands::preflight::PreflightCheck;

/// The oldest heartbeat, in seconds, a resting fleet server's open session may carry.
pub(crate) const RESTING_HEARTBEAT_SECONDS: u64 = 60;

/// The instance whose console log names the Workshop version the fleet runs.
const WORKSHOP_VERSION_INSTANCE: u16 = 1;

/// The staging missions and the terrain each must be on.
const STAGING_MISSIONS: [(&str, &str); 2] = [
    (EVERON_MISSION, EVERON_TERRAIN),
    (ARLAND_MISSION, ARLAND_TERRAIN),
];

/// One row of [`FLEET_RESTING_SESSIONS`].
#[derive(Debug, Deserialize)]
struct RestingSession {
    server: String,
    generation: Option<u64>,
    heartbeat_age_seconds: Option<u64>,
    terrain: Option<String>,
    mission: Option<String>,
}

/// The manifest's fixture identities: `servers`, `missions` (with artifact digests),
/// `fleet_scenarios` and `workshop_version`.
pub(crate) fn read(settings: &StagingSettings, host: &mut dyn HostCommandRunner) -> Result<Value> {
    let command = database_reader::select(&settings.database_container, &FLEET_IDENTITIES, &[])?;
    let output = host.run(&command)?;
    ensure!(
        output.exit_code == 0,
        "the fleet identity read exited {}",
        output.exit_code
    );
    let mut identities: Value = serde_json::from_str(output.stdout.trim())
        .context("the fleet identity read is not one JSON object")?;
    let log = console_log_reader::newest(&settings.fleet_root(), WORKSHOP_VERSION_INSTANCE);
    let workshop_version = match host.run(&log) {
        Ok(answer) if answer.exit_code == 0 => console_log_reader::parse(&answer.stdout)
            .and_then(|log| build_identity::workshop_version(&log.text))
            .unwrap_or_else(|| "unavailable (the console log names no Workshop version)".into()),
        Ok(answer) => format!(
            "unavailable (the console log read exited {})",
            answer.exit_code
        ),
        Err(error) => format!("unavailable ({error:#})"),
    };
    identities
        .as_object_mut()
        .context("the fleet identity read is not one JSON object")?
        .insert("workshop_version".into(), Value::String(workshop_version));
    Ok(identities)
}

/// The fleet run's own preconditions.
pub(crate) fn preflight_checks(settings: &StagingSettings) -> Vec<PreflightCheck> {
    let names: Vec<String> = settings
        .fleet
        .instances()
        .iter()
        .map(|instance| instance.server_name())
        .collect();
    let container = &settings.database_container;
    let mut checks = Vec::new();
    match database_reader::select(container, &FLEET_IDENTITIES, &[]) {
        Ok(command) => {
            let expected = names.clone();
            checks.push(PreflightCheck::host(
                "fleet servers registered",
                command.clone(),
                move |output| servers_registered(output, &expected),
            ));
            checks.push(PreflightCheck::host(
                "fleet missions and scenarios",
                command,
                missions_ready,
            ));
        }
        Err(error) => {
            let why = format!("{error:#}");
            checks.push(PreflightCheck::local(
                "fleet servers registered",
                move || Err(why.clone()),
            ));
        }
    }
    match database_reader::select(container, &FLEET_RESTING_SESSIONS, &[]) {
        Ok(command) => checks.push(PreflightCheck::host(
            "fleet runs the Everon deployment",
            command,
            move |output| sessions_resting(output, &names),
        )),
        Err(error) => {
            let why = format!("{error:#}");
            checks.push(PreflightCheck::local(
                "fleet runs the Everon deployment",
                move || Err(why.clone()),
            ));
        }
    }
    checks
}

/// The identity read's JSON object, or why it is not one.
fn identities(output: &CommandOutput) -> Result<Value, String> {
    if output.exit_code != 0 {
        return Err(format!("the database read exited {}", output.exit_code));
    }
    serde_json::from_str(output.stdout.trim())
        .map_err(|error| format!("the database read is not one JSON object: {error}"))
}

/// Every expected server name is registered and active exactly once.
fn servers_registered(output: &CommandOutput, expected: &[String]) -> Result<String, String> {
    let identities = identities(output)?;
    let registered: Vec<&str> = identities["servers"]
        .as_array()
        .map(|servers| {
            servers
                .iter()
                .filter_map(|server| server["name"].as_str())
                .collect()
        })
        .unwrap_or_default();
    let problems: Vec<String> = expected
        .iter()
        .filter_map(
            |name| match registered.iter().filter(|seen| *seen == name).count() {
                1 => None,
                0 => Some(format!("{name} is not registered and active")),
                count => Some(format!("{name} is registered {count} times")),
            },
        )
        .collect();
    match problems.is_empty() {
        true => Ok(format!(
            "{} fleet servers registered once each",
            expected.len()
        )),
        false => Err(problems.join("; ")),
    }
}

/// Both staging missions are live on their terrain with an artifact, and both terrains have a
/// fleet scenario row.
fn missions_ready(output: &CommandOutput) -> Result<String, String> {
    let identities = identities(output)?;
    let missions = identities["missions"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let scenarios = identities["fleet_scenarios"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let mut problems = Vec::new();
    for (title, terrain) in STAGING_MISSIONS {
        let named: Vec<&Value> = missions.iter().filter(|m| m["title"] == title).collect();
        match named.as_slice() {
            [mission] => {
                if mission["status"] != "live" || mission["terrain"] != terrain {
                    problems.push(format!(
                        "{title} is {} on {}, not live on {terrain}",
                        mission["status"], mission["terrain"]
                    ));
                }
                if mission["artifacts"].as_array().is_none_or(Vec::is_empty) {
                    problems.push(format!("{title} has no artifact"));
                }
            }
            [] => problems.push(format!("{title} does not exist")),
            _ => problems.push(format!("{title} exists {} times", named.len())),
        }
        if !scenarios.iter().any(|row| row["terrain_key"] == terrain) {
            problems.push(format!("no fleet scenario row for {terrain}"));
        }
    }
    match problems.is_empty() {
        true => Ok("both staging missions live with artifacts; both fleet scenarios set".into()),
        false => Err(problems.join("; ")),
    }
}

/// Every expected server's open runtime session is fresh and runs an Everon artifact.
fn sessions_resting(output: &CommandOutput, expected: &[String]) -> Result<String, String> {
    if output.exit_code != 0 {
        return Err(format!("the database read exited {}", output.exit_code));
    }
    let rows: Vec<RestingSession> =
        json_rows(&output.stdout).map_err(|error| format!("{error:#}"))?;
    let problems: Vec<String> = expected
        .iter()
        .filter_map(|name| {
            let Some(row) = rows.iter().find(|row| &row.server == name) else {
                return Some(format!("{name} is not registered and active"));
            };
            match (
                row.generation,
                row.heartbeat_age_seconds,
                row.terrain.as_deref(),
            ) {
                (None, _, _) => Some(format!("{name} has no open runtime session")),
                (Some(_), Some(age), _) if age > RESTING_HEARTBEAT_SECONDS => {
                    Some(format!("{name}'s session last heartbeated {age} s ago"))
                }
                (Some(_), _, Some(EVERON_TERRAIN)) => None,
                (Some(generation), _, terrain) => Some(format!(
                    "{name}'s generation {generation} runs {} on {}, not an Everon artifact",
                    row.mission.as_deref().unwrap_or("no deployed mission"),
                    terrain.unwrap_or("no terrain")
                )),
            }
        })
        .collect();
    match problems.is_empty() {
        true => Ok(format!(
            "{} fleet servers heartbeating on the Everon deployment",
            expected.len()
        )),
        false => Err(problems.join("; ")),
    }
}
