//! W6–W8: the staging missions deployed onto every fleet server, first on the terrain the
//! servers run (a `scenario_restart`), then across to Arland and back to Everon (each a
//! `host_restart`).
//!
//! **Role:** builds the three deployment steps: per server, the deployment's transition and
//! mission, its fleet command, its confirmation, and for a process restart the config switch
//! and the new process, or for a scenario restart the unchanged process.
//!
//! **Position:** called by `waves/mod.rs`; builds on `shared_probes.rs` and reads
//! `fleet_procedure/fleet_reads.rs`'s deployments, unit state and config scenario.
//!
//! **Signals & state:** none; builders and judges; measures each server's deployed scenario,
//! confirming generation and PID for the effects after it.
//!
//! **Invariants:** a deployment of another mission or with another transition contradicts its
//! effect; a `host_restart` naming the scenario of the terrain it leaves contradicts it, and it
//! must be confirmed by a session started after its own request; the unchanged process of a
//! `scenario_restart` is judged only after the deployment is confirmed; a config holds only
//! when it names the scenario the deployment row names.

use crate::error::Result;
use serde_json::Value;

use super::shared_probes::{
    first_request, measured_pid, new_process, pid_measurement, running_pid, server_effect,
    unit_probe, wave_step,
};
use super::wave_table::{
    CROSS_TERRAIN_WAVE, FleetWave, RESTART_WAVE, RETURN_WAVE, SAME_TERRAIN_WAVE,
};
use super::{FleetServer, WaveTargets};
use crate::fleet_procedure::fleet_cases::{RETURN_TO_ORIGIN_TERRAIN, per_server_case};
use crate::fleet_procedure::fleet_reads::{
    self, DeploymentRow, json_rows, newest_since, scenario_of_config, since_ms,
};
use crate::fleet_procedure::judge_mapping::server_id_measurement;
use crate::procedure_receipts::CaseName;
use crate::procedure_runner::step::{Probe, ProbeVerdict, RequestPredicate, Step};

/// The origin terrain of the fleet procedure.
pub(crate) const EVERON_TERRAIN: &str = "everon";
/// The terrain W7 crosses to.
pub(crate) const ARLAND_TERRAIN: &str = "arland";
/// The staging mission on Everon.
pub(crate) const EVERON_MISSION: &str = "TBD Staging Everon";
/// The staging mission on Arland.
pub(crate) const ARLAND_MISSION: &str = "TBD Staging Arland";

/// A deployment wave: where it deploys, what the servers run before it, and what the platform
/// must do.
struct Deployment {
    wave: &'static FleetWave,
    terrain: &'static str,
    origin_terrain: &'static str,
    mission: &'static str,
    /// `scenario_restart` or `host_restart`.
    transition: &'static str,
    /// The fleet action the transition runs: `load_mission` or `restart_with_mission`.
    command_action: &'static str,
    /// The wave whose measured PID the process is compared with.
    previous_process: &'static str,
}

const SAME_TERRAIN: Deployment = Deployment {
    wave: &SAME_TERRAIN_WAVE,
    terrain: EVERON_TERRAIN,
    origin_terrain: EVERON_TERRAIN,
    mission: EVERON_MISSION,
    transition: "scenario_restart",
    command_action: "load_mission",
    previous_process: RESTART_WAVE.step_id,
};

const CROSS_TERRAIN: Deployment = Deployment {
    wave: &CROSS_TERRAIN_WAVE,
    terrain: ARLAND_TERRAIN,
    origin_terrain: EVERON_TERRAIN,
    mission: ARLAND_MISSION,
    transition: "host_restart",
    command_action: "restart_with_mission",
    previous_process: SAME_TERRAIN_WAVE.step_id,
};

const RETURN_TO_ORIGIN: Deployment = Deployment {
    wave: &RETURN_WAVE,
    terrain: EVERON_TERRAIN,
    origin_terrain: ARLAND_TERRAIN,
    mission: EVERON_MISSION,
    transition: "host_restart",
    command_action: "restart_with_mission",
    previous_process: CROSS_TERRAIN_WAVE.step_id,
};

/// Judges one deployment row of a server.
type DeploymentJudge = fn(&DeploymentRow, &Deployment, &FleetServer) -> ProbeVerdict;

/// The steps of W6, W7 and W8.
pub(super) fn steps(targets: &WaveTargets) -> Result<Vec<Step>> {
    Ok(vec![
        deployment_step(targets, &SAME_TERRAIN, |instance| {
            per_server_case(instance, "same_terrain")
        })?,
        deployment_step(targets, &CROSS_TERRAIN, |instance| {
            per_server_case(instance, "cross_terrain")
        })?,
        deployment_step(targets, &RETURN_TO_ORIGIN, |_| {
            CaseName::new(RETURN_TO_ORIGIN_TERRAIN)
        })?,
    ])
}

/// The measurement key of the scenario `step_id`'s deployment row names for `instance`.
fn scenario_measurement(step_id: &str, instance: u16) -> String {
    format!("{step_id}.server{instance}.scenario")
}

/// The measurement key of the generation that confirmed `step_id`'s deployment on `instance`.
fn confirmation_measurement(step_id: &str, instance: u16) -> String {
    format!("{step_id}.server{instance}.confirmed_generation")
}

fn deployment_step(
    targets: &WaveTargets,
    deployment: &'static Deployment,
    case_of: impl Fn(u16) -> Result<CaseName>,
) -> Result<Step> {
    let wave = deployment.wave;
    let mut effects = Vec::new();
    for server in &targets.servers {
        let case = case_of(server.instance)?;
        let row_effects: [(&str, String, DeploymentJudge); 3] = [
            (
                "transition",
                format!(
                    "deployment of {} is a {}",
                    deployment.mission, deployment.transition
                ),
                transition_verdict,
            ),
            (
                "command",
                format!("{} command succeeded", deployment.command_action),
                command_verdict,
            ),
            (
                "confirmed",
                "deployment confirmed".to_string(),
                confirmation_verdict,
            ),
        ];
        for (id, description, judge) in row_effects {
            let probe = deployment_probe(targets, server, deployment, judge);
            effects.push(server_effect(
                wave,
                server,
                id,
                &description,
                probe,
                case.clone(),
            ));
        }
        if deployment.transition == "host_restart" {
            effects.push(server_effect(
                wave,
                server,
                "config",
                &format!("server config names the {} scenario", deployment.terrain),
                config_probe(targets, server, deployment),
                case.clone(),
            ));
            effects.push(server_effect(
                wave,
                server,
                "process",
                "game server unit runs a new process",
                new_process(wave.step_id, server, Some(deployment.previous_process)),
                case,
            ));
        } else {
            effects.push(server_effect(
                wave,
                server,
                "process",
                "game server keeps its process",
                same_process(server, deployment),
                case,
            ));
        }
    }
    wave_step(
        wave,
        targets,
        deployment_request(targets, deployment),
        effects,
    )
}

/// The request of a deployment wave: the first deployment onto its terrain since the step began.
fn deployment_request(targets: &WaveTargets, deployment: &'static Deployment) -> RequestPredicate {
    let container = targets.database_container.clone();
    let what = format!("deployment of {}", deployment.mission);
    RequestPredicate {
        description: format!("a deployment of {} to a fleet server", deployment.mission),
        probe: Probe::host(
            move |context| {
                fleet_reads::deployments_since(
                    &container,
                    deployment.terrain,
                    deployment.origin_terrain,
                    context,
                )
            },
            move |text, context| first_request(json_rows::<DeploymentRow>(text), context, &what),
        ),
    }
}

/// A probe of `server`'s newest deployment onto the wave's terrain since the step began.
fn deployment_probe(
    targets: &WaveTargets,
    server: &FleetServer,
    deployment: &'static Deployment,
    judge: DeploymentJudge,
) -> Probe {
    let container = targets.database_container.clone();
    let target = server.clone();
    Probe::host(
        move |context| {
            fleet_reads::deployments_since(
                &container,
                deployment.terrain,
                deployment.origin_terrain,
                context,
            )
        },
        move |text, context| match json_rows::<DeploymentRow>(text) {
            Err(error) => ProbeVerdict::Contradicted(format!("{error:#}")),
            Ok(rows) => match newest_since(&rows, &target.name, since_ms(context)) {
                None => ProbeVerdict::Pending(format!(
                    "no deployment of {} to {} yet",
                    deployment.mission, target.name
                )),
                Some(row) => judge(row, deployment, &target),
            },
        },
    )
}

/// Holds when the deployment deploys the wave's mission with the wave's transition; measures the
/// server's id and the scenario the deployment names.
fn transition_verdict(
    row: &DeploymentRow,
    deployment: &Deployment,
    server: &FleetServer,
) -> ProbeVerdict {
    if row.mission != deployment.mission {
        return ProbeVerdict::Contradicted(format!(
            "deployment {} deploys {:?}, not {:?}",
            row.deployment_id, row.mission, deployment.mission
        ));
    }
    if row.transition != deployment.transition {
        let why = if deployment.transition == "scenario_restart" {
            format!(
                ": the server's open session runs no {} artifact, so the platform restarts the \
                 process",
                deployment.terrain
            )
        } else {
            String::new()
        };
        return ProbeVerdict::Contradicted(format!(
            "deployment {} is a {}, not a {}{why}",
            row.deployment_id, row.transition, deployment.transition
        ));
    }
    if deployment.transition == "host_restart"
        && row.origin_scenario_id.as_deref() == Some(row.scenario_id.as_str())
    {
        return ProbeVerdict::Contradicted(format!(
            "deployment {} names {}, the scenario of the {} terrain it leaves",
            row.deployment_id, row.scenario_id, deployment.origin_terrain
        ));
    }
    ProbeVerdict::Satisfied(
        ProbeVerdict::satisfied(format!(
            "deployment {} of {} (artifact {}) is a {} to {}",
            row.deployment_id, row.mission, row.artifact_digest, row.transition, row.scenario_id
        ))
        .at(row.requested_ms)
        .measure(
            server_id_measurement(server.instance),
            row.server_id.clone(),
        )
        .measure(
            scenario_measurement(deployment.wave.step_id, server.instance),
            row.scenario_id.clone(),
        ),
    )
}

/// Holds when the deployment's fleet command is the transition's action and succeeded.
fn command_verdict(row: &DeploymentRow, deployment: &Deployment, _: &FleetServer) -> ProbeVerdict {
    if row.command_action != deployment.command_action {
        return ProbeVerdict::Contradicted(format!(
            "deployment {} runs {}, not {}",
            row.deployment_id, row.command_action, deployment.command_action
        ));
    }
    match row.command_state.as_str() {
        "succeeded" => ProbeVerdict::Satisfied(
            ProbeVerdict::satisfied(format!(
                "{} command of deployment {} succeeded",
                row.command_action, row.deployment_id
            ))
            .at(row.command_finished_ms.unwrap_or(row.requested_ms)),
        ),
        "queued" | "claimed" | "executing" => ProbeVerdict::Pending(format!(
            "{} command of deployment {} is {}",
            row.command_action, row.deployment_id, row.command_state
        )),
        state => ProbeVerdict::Contradicted(format!(
            "{} command of deployment {} ended {state}: {}",
            row.command_action,
            row.deployment_id,
            row.command_failure_reason
                .as_deref()
                .unwrap_or("no failure reason recorded")
        )),
    }
}

/// Holds when the deployment is confirmed, for a `host_restart` by a session started after its
/// request; measures the confirming generation.
fn confirmation_verdict(
    row: &DeploymentRow,
    deployment: &Deployment,
    server: &FleetServer,
) -> ProbeVerdict {
    match row.state.as_str() {
        "confirmed" => {
            let generation = row.confirmed_generation.unwrap_or_default();
            let fresh = row
                .confirmed_started_ms
                .is_some_and(|started| started >= row.requested_ms);
            if deployment.transition == "host_restart" && !fresh {
                return ProbeVerdict::Contradicted(format!(
                    "deployment {} was confirmed by generation {generation}, a session started \
                     before the deployment was requested",
                    row.deployment_id
                ));
            }
            ProbeVerdict::Satisfied(
                ProbeVerdict::satisfied(format!(
                    "deployment {} confirmed by runtime-session generation {generation}",
                    row.deployment_id
                ))
                .at(row.finished_ms.unwrap_or(row.requested_ms))
                .measure(
                    confirmation_measurement(deployment.wave.step_id, server.instance),
                    generation,
                ),
            )
        }
        "requested" => {
            ProbeVerdict::Pending(format!("deployment {} is requested", row.deployment_id))
        }
        state => ProbeVerdict::Contradicted(format!(
            "deployment {} ended {state}: {}",
            row.deployment_id,
            row.failure_reason
                .as_deref()
                .unwrap_or("no failure reason recorded")
        )),
    }
}

/// Holds when `server`'s config names the scenario this wave's deployment row named.
fn config_probe(
    targets: &WaveTargets,
    server: &FleetServer,
    deployment: &'static Deployment,
) -> Probe {
    let fleet_root = targets.fleet_root.clone();
    let instance = server.instance;
    Probe::host(
        move |_| Ok(fleet_reads::configured_scenario(&fleet_root, instance)),
        move |text, context| {
            let key = scenario_measurement(deployment.wave.step_id, instance);
            let Some(expected) = context.measurements.get(&key).and_then(Value::as_str) else {
                return ProbeVerdict::Pending(
                    "waiting for the deployment row that names the scenario".into(),
                );
            };
            match scenario_of_config(text) {
                None => ProbeVerdict::Pending("the server config names no scenario".into()),
                Some(scenario) if scenario == expected => ProbeVerdict::Satisfied(
                    ProbeVerdict::satisfied(format!("the server config names {scenario}")),
                ),
                Some(scenario) => {
                    ProbeVerdict::Pending(format!("the server config still names {scenario}"))
                }
            }
        },
    )
}

/// Holds, once the deployment is confirmed, when `server` still runs the process
/// `previous_process` measured; measures it again for the next wave.
fn same_process(server: &FleetServer, deployment: &'static Deployment) -> Probe {
    let target = server.clone();
    unit_probe(server, move |state, context| {
        let step_id = deployment.wave.step_id;
        let confirmation = confirmation_measurement(step_id, target.instance);
        if !context.measurements.contains_key(&confirmation) {
            return ProbeVerdict::Pending("waiting for the deployment's confirmation".into());
        }
        let previous_step = deployment.previous_process;
        match (
            measured_pid(context, previous_step, target.instance),
            running_pid(state),
        ) {
            (None, _) => ProbeVerdict::Contradicted(format!(
                "{previous_step} measured no process of {}, so an unchanged one cannot be shown",
                target.name
            )),
            (Some(_), None) => ProbeVerdict::Contradicted(format!(
                "{} is {}/{} after the deployment was confirmed",
                state.unit, state.active_state, state.sub_state
            )),
            (Some(previous), Some(pid)) if previous == u64::from(pid) => ProbeVerdict::Satisfied(
                ProbeVerdict::satisfied(format!("{} still runs PID {pid}", state.unit))
                    .measure(pid_measurement(step_id, target.instance), pid),
            ),
            (Some(previous), Some(pid)) => ProbeVerdict::Contradicted(format!(
                "{} runs PID {pid}, not PID {previous} that {previous_step} measured",
                state.unit
            )),
        }
    })
}
