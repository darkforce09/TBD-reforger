//! W13 and W14 on the relay's fleet server: a Restart whose claim answer, then whose result
//! answer, the acknowledgement-dropping relay withholds from the host agent.
//!
//! **Role:** builds the two steps and their judges, deciding
//! `lost_acknowledgement_claim_response` and `lost_acknowledgement_result_response`.
//!
//! **Position:** called by `waves/mod.rs`; arms the relay through
//! `remote_actions/relay_control.rs` as each step's host action; reads `single_server_reads.rs`'
//! command leases, the relay's status, the host agent's and the game server's unit journals.
//!
//! **Signals & state:** none; builders and boxed judges. The request measures the restart's
//! command id, which the ledger and relay effects judge.
//!
//! **Invariants:** a lost claim answer holds only when the ledger requeued the restart at least
//! one claim lease after its request and it succeeded under fencing token 2; a lost result answer
//! only when the restart succeeded under fencing token 1 without a requeue and the agent's retried
//! report was refused `STALE_FENCING_TOKEN`, never accepted; both need the relay to have withheld
//! that command's answer under fencing token 1 and the game server unit to have started exactly
//! once.

use anyhow::Result;
use serde_json::Value;

use super::single_server_probes::{
    journal_probe, relay_withheld, single_server_step, single_start, step_effect, step_measurement,
};
use super::wave_table::{LOST_CLAIM_ANSWER_STEP, LOST_RESULT_ANSWER_STEP, SingleServerStep};
use super::{FleetServer, WaveTargets};
use crate::commands::staging::fleet_procedure::fleet_cases::{
    LOST_ACKNOWLEDGEMENT_CLAIM_RESPONSE, LOST_ACKNOWLEDGEMENT_RESULT_RESPONSE,
};
use crate::commands::staging::fleet_procedure::fleet_reads::{json_rows, since_ms};
use crate::commands::staging::fleet_procedure::single_server_reads::{self, LeaseRow};
use crate::commands::staging::procedure_runner::step::{
    Probe, ProbeVerdict, RequestPredicate, Step, StepContext, StepKind,
};
use crate::commands::staging::remote_actions::relay_control::{self, DropTarget};
use crate::commands::staging::remote_observers::unit_journal_reader::JournalLine;
use crate::verifications::api_readiness::operational_recording::CaseName;

/// The ledger's claim lease: a claim the executor never acknowledged lapses this long after it
/// was taken, and the reconciler returns the command to the queue.
pub(crate) const CLAIM_LEASE_MS: u64 = 30_000;

/// Judges a succeeded restart's ledger row: `Ok` with the summary, or `Err` with the reason.
type LedgerJudge = fn(&LeaseRow) -> std::result::Result<String, String>;

/// One lost answer: its step, what the relay withholds, and how the ledger must end.
struct LostAnswer {
    row: &'static SingleServerStep,
    target: DropTarget,
    /// The relay's word for the withheld exchange: `claim` or `result`.
    response: &'static str,
    case: &'static str,
    ledger: LedgerJudge,
}

const LOST_CLAIM_ANSWER: LostAnswer = LostAnswer {
    row: &LOST_CLAIM_ANSWER_STEP,
    target: DropTarget::ClaimResponse,
    response: "claim",
    case: LOST_ACKNOWLEDGEMENT_CLAIM_RESPONSE,
    ledger: requeued_under_second_token,
};

const LOST_RESULT_ANSWER: LostAnswer = LostAnswer {
    row: &LOST_RESULT_ANSWER_STEP,
    target: DropTarget::ResultResponse,
    response: "result",
    case: LOST_ACKNOWLEDGEMENT_RESULT_RESPONSE,
    ledger: succeeded_under_first_token,
};

/// The steps of W13 and W14.
pub(super) fn steps(targets: &WaveTargets) -> Result<Vec<Step>> {
    Ok(vec![
        lost_answer_step(targets, &LOST_CLAIM_ANSWER)?,
        lost_answer_step(targets, &LOST_RESULT_ANSWER)?,
    ])
}

fn lost_answer_step(targets: &WaveTargets, lost: &LostAnswer) -> Result<Step> {
    let row = lost.row;
    let server = targets.server(row.server)?;
    let case = CaseName::new(lost.case)?;
    let command_key = step_measurement(row.step_id, "command_id");
    let mut effects = vec![
        step_effect(
            row,
            server,
            "ledger",
            &format!("restart ledger shows the lost {} answer", lost.response),
            ledger_probe(targets, server, lost.ledger, command_key.clone()),
            &case,
        ),
        step_effect(
            row,
            server,
            "relay_drop",
            &format!("relay withheld the restart's {} answer", lost.response),
            relay_withheld(server.instance, lost.response, command_key.clone()),
            &case,
        ),
    ];
    if lost.target == DropTarget::ResultResponse {
        effects.push(step_effect(
            row,
            server,
            "retry_refused",
            "host agent's retried result refused STALE_FENCING_TOKEN",
            journal_probe(&server.host_agent_unit, retried_result_refused),
            &case,
        ));
    }
    effects.push(step_effect(
        row,
        server,
        "single_start",
        "game server unit started exactly once",
        single_start(server),
        &case,
    ));
    single_server_step(
        row,
        server,
        StepKind::HostAction(relay_control::arm(server.instance, lost.target)),
        Some(restart_request(targets, server, command_key)),
        effects,
    )
}

/// A probe of `server`'s restart commands since the step began, judged by `judge`.
fn leases_probe(
    targets: &WaveTargets,
    server: &FleetServer,
    judge: impl Fn(&[LeaseRow], &StepContext<'_>) -> ProbeVerdict + 'static,
) -> Probe {
    let container = targets.database_container.clone();
    let name = server.name.clone();
    Probe::host(
        move |context| {
            single_server_reads::command_leases_since(&container, &name, "restart", context)
        },
        move |text, context| match json_rows::<LeaseRow>(text) {
            Err(error) => ProbeVerdict::Contradicted(format!("{error:#}")),
            Ok(rows) => judge(&rows, context),
        },
    )
}

/// The first restart of `server` since the step began; measures its command id.
fn restart_request(
    targets: &WaveTargets,
    server: &FleetServer,
    command_key: String,
) -> RequestPredicate {
    let name = server.name.clone();
    RequestPredicate {
        description: format!("a restart command for {name}"),
        probe: leases_probe(targets, server, move |rows, context| {
            let since = since_ms(context);
            match rows
                .iter()
                .filter(|row| row.requested_ms >= since)
                .min_by_key(|row| row.requested_ms)
            {
                None => ProbeVerdict::Pending(format!(
                    "no restart command for {name} requested since the step began"
                )),
                Some(first) => ProbeVerdict::Satisfied(
                    ProbeVerdict::satisfied(format!(
                        "restart command {} requested for {name}",
                        first.command_id
                    ))
                    .at(first.requested_ms)
                    .measure(command_key.clone(), first.command_id.clone()),
                ),
            }
        }),
    }
}

/// The restart the request measured: pending while it is queued, claimed or executing,
/// contradicted once it ended in any state but `succeeded`, and otherwise judged by `judge` at
/// its finish.
fn ledger_probe(
    targets: &WaveTargets,
    server: &FleetServer,
    judge: LedgerJudge,
    command_key: String,
) -> Probe {
    leases_probe(targets, server, move |rows, context| {
        let Some(command) = context
            .measurements
            .get(&command_key)
            .and_then(Value::as_str)
        else {
            return ProbeVerdict::Pending("the restart command is not known yet".into());
        };
        let Some(row) = rows.iter().find(|row| row.command_id == command) else {
            return ProbeVerdict::Pending(format!("restart command {command} is not in the read"));
        };
        match row.state.as_str() {
            "queued" | "claimed" | "executing" => ProbeVerdict::Pending(format!(
                "restart command {command} is {} under fencing token {} after {} attempt(s)",
                row.state, row.fencing_token, row.attempts
            )),
            "succeeded" => {
                let finished = row.finished_ms.unwrap_or(row.requested_ms);
                match judge(row) {
                    Ok(summary) => {
                        ProbeVerdict::Satisfied(ProbeVerdict::satisfied(summary).at(finished))
                    }
                    Err(why) => ProbeVerdict::Contradicted(why),
                }
            }
            state => ProbeVerdict::Contradicted(format!(
                "restart command {command} ended {state}: {}",
                row.failure_reason
                    .as_deref()
                    .unwrap_or("no failure reason recorded")
            )),
        }
    })
}

/// W13's restart: the ledger requeued it at least one claim lease after its request, and the
/// second claim, which ran it, carried fencing token 2.
fn requeued_under_second_token(row: &LeaseRow) -> std::result::Result<String, String> {
    let Some(requeued) = row.requeued_ms else {
        return Err(format!(
            "restart command {} succeeded without a lease-lapse requeue, so no claim answer was \
             lost",
            row.command_id
        ));
    };
    let after = requeued.saturating_sub(row.requested_ms);
    if after < CLAIM_LEASE_MS {
        return Err(format!(
            "restart command {} was requeued {after} ms after its request, sooner than the \
             {CLAIM_LEASE_MS} ms claim lease",
            row.command_id
        ));
    }
    if row.fencing_token != 2 {
        return Err(format!(
            "restart command {} succeeded under fencing token {}, not 2",
            row.command_id, row.fencing_token
        ));
    }
    Ok(format!(
        "restart command {} was requeued {after} ms after its request by a lapsed lease and \
         succeeded under fencing token 2 after {} attempt(s)",
        row.command_id, row.attempts
    ))
}

/// W14's restart: it succeeded under the first claim's fencing token 1, without a requeue, so
/// the ledger kept the result the relay's withheld answer acknowledged.
fn succeeded_under_first_token(row: &LeaseRow) -> std::result::Result<String, String> {
    if let Some(requeued) = row.requeued_ms {
        return Err(format!(
            "restart command {} was requeued at {requeued}, so its claim, not its result, went \
             unacknowledged",
            row.command_id
        ));
    }
    if row.fencing_token != 1 {
        return Err(format!(
            "restart command {} succeeded under fencing token {}, not 1",
            row.command_id, row.fencing_token
        ));
    }
    Ok(format!(
        "restart command {} succeeded under fencing token 1 without a requeue and stays succeeded",
        row.command_id
    ))
}

/// Holds when the host agent logged its retried result refused `STALE_FENCING_TOKEN`; a result
/// report the API accepted contradicts it.
fn retried_result_refused(lines: &[JournalLine], _: &StepContext<'_>) -> ProbeVerdict {
    if let Some(accepted) = lines
        .iter()
        .find(|line| line.text.contains("result reported"))
    {
        return ProbeVerdict::Contradicted(format!(
            "the API accepted a result report at {}: {}",
            accepted.unix_ms,
            accepted.text.trim()
        ));
    }
    match lines.iter().find(|line| {
        line.text.contains("STALE_FENCING_TOKEN") && line.text.contains("result is not reported")
    }) {
        Some(line) => ProbeVerdict::Satisfied(
            ProbeVerdict::satisfied(format!(
                "the retried result was refused at {}: {}",
                line.unix_ms,
                line.text.trim()
            ))
            .at(line.unix_ms),
        ),
        None => ProbeVerdict::Pending(
            "no result report refused STALE_FENCING_TOKEN in the host agent's journal yet".into(),
        ),
    }
}
