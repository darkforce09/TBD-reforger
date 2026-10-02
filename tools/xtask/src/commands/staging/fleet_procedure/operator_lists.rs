//! The numbered lists the operator approves for the fleet run: its real actions, and the
//! actions that put the host back after a stopped run.
//!
//! **Role:** builds the fleet's [`PlannedAction`]s from the wave table (W1–W8 one action per
//! wave, W9–W14 one per step, each harness step with the exact host command the plan runs), and
//! its recovery actions: disarm the relay, start any game server left stopped, deploy Everon back
//! onto any server left on another terrain, and finish a credential rotation left midway.
//!
//! **Position:** called by `FleetProcedure::action_list` and
//! `FleetProcedure::recovery_action_list`; printed by `staging action-list fleet
//! [--recovery]`.
//!
//! **Signals & state:** none; pure builders over the settings.
//!
//! **Invariants:** each W1–W8 wave is one numbered action naming every server it touches, and
//! each W9–W14 step one naming its server and actor, with the awaited effect and the deadline, in
//! the order the plan runs them; a host command shown is the plan's own; the relay disarm names
//! the exact host command; nothing here runs a command.

use super::waves::deployment_waves::EVERON_MISSION;
use super::waves::wave_table::{FLEET_WAVES, SINGLE_SERVER_STEPS, SingleServerStep, WaveServer};
use super::waves::{WaveTargets, steps};
use crate::commands::staging::operator_coordination::action_list::PlannedAction;
use crate::commands::staging::procedure_runner::step::{Step, StepKind};
use crate::commands::staging::remote_actions::relay_control;
use crate::commands::staging::staging_settings::StagingSettings;

/// The actor of every Server Control action.
const ORCHESTRATOR: &str = "orchestrator (browser)";
/// The actor of a command run on the host by hand.
const OPERATOR: &str = "operator";

/// One numbered action per wave, in the order the plan runs them.
pub(crate) fn action_list(settings: &StagingSettings) -> Vec<PlannedAction> {
    let servers: Vec<String> = settings
        .fleet
        .instances()
        .iter()
        .map(|instance| instance.server_name())
        .collect();
    let mut actions: Vec<PlannedAction> = FLEET_WAVES
        .iter()
        .map(|wave| {
            let summary = format!(
                "W{}: {} on every fleet server in Server Control",
                wave.number, wave.server_control_action
            );
            let action = servers.iter().fold(
                PlannedAction::new(ORCHESTRATOR, summary),
                |action, server| {
                    action.detail(format!("{} on {server}", wave.server_control_action))
                },
            );
            action.detail(format!(
                "awaited: {} (deadline {} s from the wave's first request)",
                wave.awaited_effect, wave.deadline_seconds
            ))
        })
        .collect();
    let plan = WaveTargets::from_settings(settings)
        .and_then(|targets| steps(&targets, settings))
        .map_err(|error| format!("{error:#}"));
    actions.extend(
        SINGLE_SERVER_STEPS
            .iter()
            .map(|step| single_server_action(settings, step, &plan)),
    );
    actions
}

/// The name of the fleet server `which` names in `settings`.
fn server_name(settings: &StagingSettings, which: WaveServer) -> String {
    let instance = match which {
        WaveServer::First => Some(1),
        WaveServer::Second => Some(2),
        WaveServer::Relay => settings.relay_unit().map(|(instance, _)| instance),
    };
    match instance {
        Some(number) => format!("TBD Staging {number}"),
        None => "<the relay instance: TBD_FLEET_RELAY_INSTANCE unset>".to_string(),
    }
}

/// One W9–W14 step as a numbered action; a harness step shows the host command the plan runs,
/// or why the plan cannot be built.
fn single_server_action(
    settings: &StagingSettings,
    step: &SingleServerStep,
    plan: &Result<Vec<Step>, String>,
) -> PlannedAction {
    let server = server_name(settings, step.server);
    let action = PlannedAction::new(
        step.actor,
        format!("W{}: {}", step.wave, step.action_on(&server)),
    );
    let action = match plan {
        Err(why) => action.detail(format!("the plan cannot be built: {why}")),
        Ok(planned) => match planned
            .iter()
            .find(|planned| planned.id.as_str() == step.step_id)
            .map(|planned| &planned.kind)
        {
            Some(StepKind::HostAction(command)) => {
                let action = action.detail(format!("on the host: {}", command.command_line));
                command.stdin.iter().flat_map(|script| script.lines()).fold(
                    action,
                    |action, line| match line.trim() {
                        "" => action,
                        _ => action.detail(format!("  {line}")),
                    },
                )
            }
            _ => action,
        },
    };
    action.detail(format!(
        "awaited: {} (deadline {} s from {})",
        step.awaited_on(&server),
        step.deadline_seconds,
        step.anchor()
    ))
}

/// The actions that put the host back after a fleet run stopped early.
pub(crate) fn recovery_action_list(settings: &StagingSettings) -> Vec<PlannedAction> {
    let instances = settings.fleet.instances();
    let mut actions = Vec::new();
    if let Some((instance, unit)) = settings.relay_unit() {
        actions.push(
            PlannedAction::new(
                OPERATOR,
                format!("Disarm the acknowledgement-dropping relay of instance {instance}"),
            )
            .detail(format!(
                "on the host: {}",
                relay_control::disarm(instance).command_line
            ))
            .detail(format!(
                "then `cargo xtask staging status` shows {unit} disarmed"
            )),
        );
    }
    let start = instances.iter().fold(
        PlannedAction::new(
            ORCHESTRATOR,
            "Start every fleet server whose game server unit was left stopped",
        )
        .detail("`cargo xtask staging status` names each inactive game server unit"),
        |action, instance| {
            action.detail(format!(
                "{} inactive: Start on {} in Server Control",
                instance.game_server_unit(),
                instance.server_name()
            ))
        },
    );
    actions.push(
        start.detail("then `cargo xtask staging status` shows every game server unit active"),
    );
    let origin = instances.iter().fold(
        PlannedAction::new(
            ORCHESTRATOR,
            format!("Deploy {EVERON_MISSION} on every fleet server left on another terrain"),
        )
        .detail("`cargo xtask staging preflight` names each server whose session runs no Everon artifact"),
        |action, instance| {
            action.detail(format!(
                "{} named: Deploy the mission \"{EVERON_MISSION}\" on it in Server Control",
                instance.server_name()
            ))
        },
    );
    actions.push(
        origin.detail(
            "then `cargo xtask staging preflight` shows the fleet on the Everon deployment",
        ),
    );
    actions.push(
        PlannedAction::new(
            OPERATOR,
            "Finish a credential rotation W11 or W12 left midway",
        )
        .detail(
            "Server Control, Credentials, on TBD Staging 1 (host_agent) and TBD Staging 2 \
                 (mod_runtime): two unrevoked credentials of one program, or a revoked one its \
                 program still holds, mark a rotation left midway",
        )
        .detail(
            "host_agent: `cargo xtask staging rotate-credential --instance 1 --executor \
                 host_agent --promote`, then on the host `systemctl --user restart \
                 fleet_host_agent@1.service`",
        )
        .detail(
            "mod_runtime: `cargo xtask staging rotate-credential --instance 2 --executor \
                 mod_runtime --promote`, then `cargo xtask deploy staging`, which writes every \
                 profile from its live credential file and restarts every game server",
        )
        .detail(
            "then Revoke the older credential of that program in Server Control, Credentials, \
                 and `cargo xtask staging status` shows every unit active",
        ),
    );
    actions
}
