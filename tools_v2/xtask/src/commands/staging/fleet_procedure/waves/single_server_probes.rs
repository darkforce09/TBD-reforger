//! The step, effect and probe builders the single-server waves W9–W14 share: the step of a
//! wave-table row, its effects, the unit journal since the request, the game server unit's
//! starts, the relay's status, and the browser text matching.
//!
//! **Role:** turns a [`SingleServerStep`] and its server into a [`Step`] and
//! [`EffectPredicate`]s, and holds the probes more than one of W9–W14 uses.
//!
//! **Position:** called by `identity_waves.rs`, `credential_waves.rs` and
//! `lost_acknowledgement_waves.rs`; reads through `remote_observers/unit_journal_reader.rs` and
//! `remote_actions/relay_control.rs`.
//!
//! **Signals & state:** none; builders and boxed judges.
//!
//! **Invariants:** a journal read starts at the step's request row (the host's clock), less the
//! clock allowance, or at the step's start when no request was observed; a single start holds only
//! once [`DUPLICATE_START_WINDOW_MS`] has passed since the unit's first start, and a second start
//! contradicts it whenever it shows; a relay drop counts only for the command the step's request
//! measured.

use serde::Deserialize;
use serde_json::Value;

use super::FleetServer;
use super::wave_table::SingleServerStep;
use crate::commands::staging::fleet_procedure::fleet_reads::REQUEST_CLOCK_TOLERANCE_MS;
use crate::commands::staging::procedure_runner::step::{
    EffectPredicate, Probe, ProbeVerdict, RequestPredicate, Step, StepContext, StepId, StepKind,
};
use crate::commands::staging::remote_actions::relay_control;
use crate::commands::staging::remote_observers::unit_journal_reader::{self, JournalLine};
use crate::verifications::api_readiness::operational_recording::CaseName;

/// How long after the unit's first start a second execution of the same command would have
/// started it again: a second claim follows the first within the claim lease (30 s), a
/// reconciliation pass (5 s) and a claim poll (5 s), and the margin covers the restart itself.
pub(crate) const DUPLICATE_START_WINDOW_MS: u64 = 60_000;

/// The step of `row` on `server`.
pub(super) fn single_server_step(
    row: &SingleServerStep,
    server: &FleetServer,
    kind: StepKind,
    request: Option<RequestPredicate>,
    effects: Vec<EffectPredicate>,
) -> anyhow::Result<Step> {
    Ok(Step {
        id: StepId::new(row.step_id)?,
        kind,
        instruction: format!(
            "W{}: {}; the harness waits until {} (deadline {} s from {})",
            row.wave,
            row.action_on(&server.name),
            row.awaited_on(&server.name),
            row.deadline_seconds,
            row.anchor()
        ),
        request,
        effects,
    })
}

/// An effect of `row`'s step on `server`, held to the row's deadline.
pub(super) fn step_effect(
    row: &SingleServerStep,
    server: &FleetServer,
    id: &str,
    description: &str,
    probe: Probe,
    case: &CaseName,
) -> EffectPredicate {
    EffectPredicate {
        id: id.to_string(),
        description: format!("{} {description}", server.name),
        probe,
        deadline: row.deadline(),
        case: case.clone(),
    }
}

/// The measurement key of `what` for `step_id`: `<step>.<what>`.
pub(crate) fn step_measurement(step_id: &str, what: &str) -> String {
    format!("{step_id}.{what}")
}

/// The whole second a journal read of the step of `context` starts at: the request row's time
/// (the host's clock) or the step's start, less the clock allowance.
pub(super) fn journal_since_seconds(context: &StepContext<'_>) -> u64 {
    context
        .request_unix_ms
        .unwrap_or(context.step_started_unix_ms)
        .saturating_sub(REQUEST_CLOCK_TOLERANCE_MS)
        / 1000
}

/// A probe of `unit`'s journal since the step's request, judged by `judge`.
pub(super) fn journal_probe(
    unit: &str,
    judge: impl Fn(&[JournalLine], &StepContext<'_>) -> ProbeVerdict + 'static,
) -> Probe {
    let unit = unit.to_string();
    Probe::host(
        move |context| {
            Ok(unit_journal_reader::since(
                &unit,
                journal_since_seconds(context),
            ))
        },
        move |text, context| judge(&unit_journal_reader::parse(text), context),
    )
}

/// Whether a journal line is systemd starting the unit.
fn is_unit_start(line: &JournalLine) -> bool {
    line.text.contains("systemd[") && line.text.contains(": Started ")
}

/// `server`'s game server unit started exactly once since the request: pending until
/// [`DUPLICATE_START_WINDOW_MS`] has passed since its first start, and contradicted by a second
/// start whenever it shows.
pub(super) fn single_start(server: &FleetServer) -> Probe {
    let unit = server.unit.clone();
    journal_probe(&server.unit, move |lines, context| {
        let starts: Vec<&JournalLine> = lines.iter().filter(|line| is_unit_start(line)).collect();
        match starts.as_slice() {
            [] => ProbeVerdict::Pending(format!("{unit} has not started since the request")),
            [start] => {
                let settled = start.unix_ms.saturating_add(DUPLICATE_START_WINDOW_MS);
                if context.observed_unix_ms < settled {
                    ProbeVerdict::Pending(format!(
                        "{unit} started at {}; a second start would show by {settled}",
                        start.unix_ms
                    ))
                } else {
                    ProbeVerdict::Satisfied(ProbeVerdict::satisfied(format!(
                        "{unit} started once, at {}, and not again within {} ms",
                        start.unix_ms, DUPLICATE_START_WINDOW_MS
                    )))
                }
            }
            starts => {
                let times: Vec<String> =
                    starts.iter().map(|line| line.unix_ms.to_string()).collect();
                ProbeVerdict::Contradicted(format!(
                    "{unit} started {} times since the request (at {})",
                    starts.len(),
                    times.join(", ")
                ))
            }
        }
    })
}

/// The relay status fields the drop effects judge.
#[derive(Debug, Deserialize)]
struct RelayStatus {
    arming: String,
    drop_count: u64,
    last_drop: Option<RelayDrop>,
}

/// The relay's most recent withheld answer.
#[derive(Debug, Deserialize)]
struct RelayDrop {
    /// `claim` or `result`.
    response: String,
    command_id: Option<String>,
    fencing_token: Option<i64>,
}

/// The relay of `instance` withheld one `response` answer (`claim` or `result`) for the command
/// measured under `command_key`, carrying fencing token 1, and is disarmed again.
pub(super) fn relay_withheld(instance: u16, response: &'static str, command_key: String) -> Probe {
    Probe::host(
        move |_| Ok(relay_control::status(instance)),
        move |text, context| {
            let status: RelayStatus = match serde_json::from_str(text.trim()) {
                Ok(status) => status,
                Err(error) => {
                    return ProbeVerdict::Contradicted(format!(
                        "the relay status is not the JSON `control status` prints: {error}"
                    ));
                }
            };
            let Some(command) = context
                .measurements
                .get(&command_key)
                .and_then(Value::as_str)
            else {
                return ProbeVerdict::Pending("the step's command is not known yet".into());
            };
            let Some(drop) = status.last_drop.filter(|_| status.arming == "disarmed") else {
                return ProbeVerdict::Pending(format!(
                    "the relay is {} with {} drop(s) and none spent on this arming",
                    status.arming, status.drop_count
                ));
            };
            if drop.response != response || drop.command_id.as_deref() != Some(command) {
                return ProbeVerdict::Contradicted(format!(
                    "the relay withheld the {} answer of command {}, not the {response} answer of \
                     command {command}",
                    drop.response,
                    drop.command_id.as_deref().unwrap_or("(unnamed)")
                ));
            }
            match drop.fencing_token {
                Some(1) => ProbeVerdict::Satisfied(ProbeVerdict::satisfied(format!(
                    "the relay withheld the {response} answer of command {command} under fencing \
                     token 1 and is disarmed ({} drop(s) in all)",
                    status.drop_count
                ))),
                other => ProbeVerdict::Contradicted(format!(
                    "the withheld {response} answer of command {command} carried fencing token \
                     {other:?}, not 1"
                )),
            }
        },
    )
}

/// Browser text with escapes and white space removed, so JSON pairs match however the Chrome
/// tool quoted them.
pub(super) fn compact(text: &str) -> String {
    text.chars()
        .filter(|character| !character.is_whitespace() && *character != '\\')
        .collect()
}
