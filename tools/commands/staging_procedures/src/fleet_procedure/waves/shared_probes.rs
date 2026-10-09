//! The step, request and effect builders every fleet wave uses: the wave's step, its request
//! (the first matching row since the step began), a server's command outcome, and a server's
//! game server process.
//!
//! **Role:** turns a [`FleetWave`] and the fleet servers into [`Step`]s, [`RequestPredicate`]s
//! and [`EffectPredicate`]s, and holds the judges that more than one wave shares.
//!
//! **Position:** called by `process_waves.rs`, `console_waves.rs`, `deployment_waves.rs` and
//! `identity_waves.rs`; reads through `fleet_procedure/fleet_reads.rs`; measures what
//! `judge_mapping.rs` and later waves read.
//!
//! **Signals & state:** none; builders and boxed judges.
//!
//! **Invariants:** a wave's request is satisfied at the earliest matching row since its step
//! began, so every server is held to at most the wave's deadline from its own request; a
//! server's effect judges only that server's newest row since the step began; a command that
//! ended in any state but `succeeded` contradicts its effect at once; a PID compared with an
//! earlier wave's must have been measured by that wave, or the comparison is refused.

use serde_json::Value;

use super::wave_table::FleetWave;
use super::{FleetServer, WaveTargets};
use crate::fleet_procedure::fleet_reads::{
    self, CommandRow, FleetRow, json_rows, newest_since, since_ms,
};
use crate::fleet_procedure::judge_mapping::server_id_measurement;
use crate::procedure_receipts::CaseName;
use crate::procedure_runner::step::{
    Deadline, EffectPredicate, Probe, ProbeVerdict, RequestPredicate, Satisfaction, Step,
    StepContext, StepId, StepKind,
};
use crate::remote_observers::unit_state_reader::{self, UnitState};

/// Accepts a command row as the one a wave waits for.
pub(super) type RowFilter = fn(&CommandRow) -> bool;
/// Judges a succeeded command's outcome, given the satisfaction its success already holds.
pub(super) type SuccessJudge = fn(&CommandRow, &FleetServer, Satisfaction) -> ProbeVerdict;

/// The measurement key of the PID `step_id` observed for `instance`.
pub(crate) fn pid_measurement(step_id: &str, instance: u16) -> String {
    format!("{step_id}.server{instance}.pid")
}

/// The wave's step: an orchestrator action in Server Control on every fleet server.
pub(super) fn wave_step(
    wave: &FleetWave,
    targets: &WaveTargets,
    request: RequestPredicate,
    effects: Vec<EffectPredicate>,
) -> crate::error::Result<Step> {
    Ok(Step {
        id: StepId::new(wave.step_id)?,
        kind: StepKind::ChromeAction,
        instruction: format!(
            "W{}: in Server Control, {} on {}; the harness waits until {} (deadline {} s from \
             the first request)",
            wave.number,
            wave.server_control_action,
            targets.server_list(),
            wave.awaited_effect,
            wave.deadline_seconds
        ),
        request: Some(request),
        effects,
    })
}

/// An effect of `wave` on `server`: its id is `server<N>_<id>` and its description names the
/// server.
pub(super) fn server_effect(
    wave: &FleetWave,
    server: &FleetServer,
    id: &str,
    description: &str,
    probe: Probe,
    case: CaseName,
) -> EffectPredicate {
    EffectPredicate {
        id: format!("server{}_{id}", server.instance),
        description: format!("{} {description}", server.name),
        probe,
        deadline: Deadline::from_request_row(wave.deadline_seconds),
        case,
    }
}

/// Satisfied at the earliest of `rows` requested since the step began.
pub(super) fn first_request<T: FleetRow>(
    rows: crate::error::Result<Vec<T>>,
    context: &StepContext<'_>,
    what: &str,
) -> ProbeVerdict {
    let rows = match rows {
        Ok(rows) => rows,
        Err(error) => return ProbeVerdict::Contradicted(format!("{error:#}")),
    };
    let since = since_ms(context);
    let requested: Vec<&T> = rows
        .iter()
        .filter(|row| row.requested_ms() >= since)
        .collect();
    match requested.iter().min_by_key(|row| row.requested_ms()) {
        None => ProbeVerdict::Pending(format!("no {what} requested since the step began")),
        Some(first) => ProbeVerdict::Satisfied(
            ProbeVerdict::satisfied(format!(
                "first {what} requested for {} ({} fleet rows so far)",
                first.server(),
                requested.len()
            ))
            .at(first.requested_ms()),
        ),
    }
}

/// The request of a command wave: the first `action` command for a fleet server since the step
/// began.
pub(super) fn command_request(targets: &WaveTargets, action: &'static str) -> RequestPredicate {
    let container = targets.database_container.clone();
    let what = format!("{action} command");
    RequestPredicate {
        description: format!("a {action} command for a fleet server"),
        probe: Probe::host(
            move |context| fleet_reads::commands_since(&container, action, context),
            move |text, context| first_request(json_rows::<CommandRow>(text), context, &what),
        ),
    }
}

/// What a command effect waits for: the action, the row it accepts, and the judge of a
/// succeeded command's outcome.
pub(super) struct CommandExpectation {
    pub action: &'static str,
    /// The effect's description after the server's name.
    pub description: String,
    pub accepts: RowFilter,
    pub on_success: SuccessJudge,
}

impl CommandExpectation {
    /// Any `action` command, whose success is the whole effect.
    pub(super) fn succeeded(action: &'static str) -> Self {
        Self {
            action,
            description: format!("{action} command succeeded"),
            accepts: |_| true,
            on_success: |_, _, satisfaction| ProbeVerdict::Satisfied(satisfaction),
        }
    }
}

/// `server`'s newest command since the step began that `expected` accepts has succeeded, and
/// its outcome satisfies `expected`.
pub(super) fn command_effect(
    wave: &FleetWave,
    targets: &WaveTargets,
    server: &FleetServer,
    expected: CommandExpectation,
    case: CaseName,
) -> EffectPredicate {
    let CommandExpectation {
        action,
        description,
        accepts,
        on_success,
    } = expected;
    let probe = command_probe(targets, server, action, accepts, on_success);
    server_effect(wave, server, "command", &description, probe, case)
}

/// A probe of `server`'s newest `action` command since the step began that `accepts` takes:
/// pending while it runs, contradicted once it ended in any state but `succeeded`, and otherwise
/// judged by `on_success`.
pub(super) fn command_probe(
    targets: &WaveTargets,
    server: &FleetServer,
    action: &'static str,
    accepts: RowFilter,
    on_success: SuccessJudge,
) -> Probe {
    let container = targets.database_container.clone();
    let target = server.clone();
    Probe::host(
        move |context| fleet_reads::commands_since(&container, action, context),
        move |text, context| {
            let rows: Vec<CommandRow> = match json_rows(text) {
                Ok(rows) => rows.into_iter().filter(accepts).collect(),
                Err(error) => return ProbeVerdict::Contradicted(format!("{error:#}")),
            };
            match newest_since(&rows, &target.name, since_ms(context)) {
                None => {
                    ProbeVerdict::Pending(format!("no {action} command for {} yet", target.name))
                }
                Some(row) => command_verdict(row, action, &target, on_success),
            }
        },
    )
}

/// [`command_effect`] for any `action` command whose success is the whole effect.
pub(super) fn command_succeeded(
    wave: &FleetWave,
    targets: &WaveTargets,
    server: &FleetServer,
    action: &'static str,
    case: CaseName,
) -> EffectPredicate {
    command_effect(
        wave,
        targets,
        server,
        CommandExpectation::succeeded(action),
        case,
    )
}

/// The verdict on one command row: pending while it runs, contradicted once it ended in any
/// state but `succeeded`, and otherwise `on_success`'s verdict on a satisfaction that holds the
/// finish time and measures the server's id.
fn command_verdict(
    row: &CommandRow,
    action: &str,
    server: &FleetServer,
    on_success: SuccessJudge,
) -> ProbeVerdict {
    match row.state.as_str() {
        "succeeded" => {
            let satisfaction =
                ProbeVerdict::satisfied(format!("{action} command {} succeeded", row.command_id))
                    .at(row.finished_ms.unwrap_or(row.requested_ms))
                    .measure(
                        server_id_measurement(server.instance),
                        row.server_id.clone(),
                    );
            on_success(row, server, satisfaction)
        }
        "queued" | "claimed" | "executing" => ProbeVerdict::Pending(format!(
            "{action} command {} is {}",
            row.command_id, row.state
        )),
        state => ProbeVerdict::Contradicted(format!(
            "{action} command {} ended {state}: {}",
            row.command_id,
            row.failure_reason
                .as_deref()
                .unwrap_or("no failure reason recorded")
        )),
    }
}

/// A probe of `server`'s game server unit, judged by `judge` once the read names the unit.
pub(super) fn unit_probe(
    server: &FleetServer,
    judge: impl Fn(&UnitState, &StepContext<'_>) -> ProbeVerdict + 'static,
) -> Probe {
    let unit = server.unit.clone();
    let read_unit = unit.clone();
    Probe::host(
        move |_| Ok(fleet_reads::game_server_unit(&read_unit)),
        move |text, context| match unit_state_reader::parse(text).get(&unit) {
            None => ProbeVerdict::Pending(format!("the unit read does not name {unit}")),
            Some(state) => judge(state, context),
        },
    )
}

/// `state`'s main PID when the unit is active.
pub(super) fn running_pid(state: &UnitState) -> Option<u32> {
    state.main_pid.filter(|_| state.active_state == "active")
}

/// The PID `step_id` measured for `instance`, if it did.
pub(super) fn measured_pid(context: &StepContext<'_>, step_id: &str, instance: u16) -> Option<u64> {
    context
        .measurements
        .get(&pid_measurement(step_id, instance))
        .and_then(Value::as_u64)
}

/// `server`'s unit runs a process, other than the one `previous_step` measured when one is
/// named; measures its PID for `step_id`.
pub(super) fn new_process(
    step_id: &'static str,
    server: &FleetServer,
    previous_step: Option<&'static str>,
) -> Probe {
    let target = server.clone();
    unit_probe(server, move |state, context| {
        let Some(pid) = running_pid(state) else {
            return ProbeVerdict::Pending(format!(
                "{} is {}/{}",
                state.unit, state.active_state, state.sub_state
            ));
        };
        if let Some(previous_step) = previous_step {
            match measured_pid(context, previous_step, target.instance) {
                None => {
                    return ProbeVerdict::Contradicted(format!(
                        "{previous_step} measured no process of {}, so a new one cannot be told \
                         apart",
                        target.name
                    ));
                }
                Some(previous) if previous == u64::from(pid) => {
                    return ProbeVerdict::Pending(format!(
                        "{} still runs PID {pid}, the process {previous_step} measured",
                        state.unit
                    ));
                }
                Some(_) => {}
            }
        }
        ProbeVerdict::Satisfied(
            ProbeVerdict::satisfied(format!("{} active with PID {pid}", state.unit))
                .measure(pid_measurement(step_id, target.instance), pid),
        )
    })
}
