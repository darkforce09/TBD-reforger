//! W9 and W10 on the first fleet server: the operator's Arma identity linked through
//! `#tbd link`, the operator kicked by a fleet command, and a kick naming an ended runtime
//! session refused.
//!
//! **Role:** builds the steps of W9 (`identity_link`) and W10 (`kick`,
//! `rejection_ended_session_kick`) and their judges.
//!
//! **Position:** called by `waves/mod.rs`; reads `single_server_reads.rs`' link row,
//! `fleet_reads.rs`' commands, the server's `console.log` and the step's browser inbox entry;
//! measures the linked Arma id W10 compares the kick with.
//!
//! **Signals & state:** none; builders and boxed judges.
//!
//! **Invariants:** the link holds only when the code issued since the step began was consumed,
//! the account carries the Arma id the code did, an `identity.link` audit row followed, the
//! server's listing since the step began names that id and the saved link status reads it
//! linked; the kick holds only when its command succeeded against the linked id and the game
//! runtime logged that command's kick; the ended-session kick holds only when the saved answer is
//! 409 `RUNTIME_SESSION_ENDED` and no kick but W10's first was recorded since the step began,
//! judged once that answer was read.

use anyhow::Result;
use serde_json::Value;

use super::console_waves::listed_arma_ids;
use super::shared_probes::{command_probe, first_request};
use super::single_server_probes::{compact, single_server_step, step_effect, step_measurement};
use super::wave_table::{ENDED_SESSION_KICK_STEP, IDENTITY_LINK_STEP, KICK_STEP};
use super::{FleetServer, WaveTargets};
use crate::commands::staging::fleet_procedure::fleet_cases::{
    IDENTITY_LINK, KICK, REJECTION_ENDED_SESSION_KICK,
};
use crate::commands::staging::fleet_procedure::fleet_reads::{
    self, CommandRow, json_rows, since_ms,
};
use crate::commands::staging::fleet_procedure::judge_mapping::player_listing_measurement;
use crate::commands::staging::fleet_procedure::single_server_reads::{self, IdentityLinkRow};
use crate::commands::staging::procedure_runner::step::{
    Probe, ProbeVerdict, RequestPredicate, Satisfaction, Step, StepContext, StepKind,
};
use crate::commands::staging::remote_observers::console_log_reader;
use crate::verifications::api_readiness::operational_recording::CaseName;

/// The code the API's refusal of a command against an ended runtime session carries.
const RUNTIME_SESSION_ENDED: &str = "RUNTIME_SESSION_ENDED";
/// The start of the line the game runtime logs for a fleet kick, before the command id.
const KICK_LOG_LINE: &str = "[TBD][Fleet] kicked command=";

/// The measurement key of the Arma id W9 linked to the operator's account.
pub(crate) fn linked_arma_id_key() -> String {
    step_measurement(IDENTITY_LINK_STEP.step_id, "linked_arma_id")
}

fn listed_arma_ids_key() -> String {
    step_measurement(IDENTITY_LINK_STEP.step_id, "listed_arma_ids")
}

fn kick_command_key() -> String {
    step_measurement(KICK_STEP.step_id, "command_id")
}

fn kicked_arma_id_key() -> String {
    step_measurement(KICK_STEP.step_id, "kicked_arma_id")
}

fn refusal_read_key() -> String {
    step_measurement(ENDED_SESSION_KICK_STEP.step_id, "refusal_read_ms")
}

/// The text value the run measured under `key`.
fn measured_text(context: &StepContext<'_>, key: &str) -> Option<String> {
    context
        .measurements
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
}

/// The steps of W9 and W10.
pub(super) fn steps(targets: &WaveTargets) -> Result<Vec<Step>> {
    let server = targets.server(IDENTITY_LINK_STEP.server)?;
    Ok(vec![
        identity_link(targets, server)?,
        kick(targets, server)?,
        ended_session_kick(targets, server)?,
    ])
}

fn identity_link(targets: &WaveTargets, server: &FleetServer) -> Result<Step> {
    let row = &IDENTITY_LINK_STEP;
    let case = CaseName::new(IDENTITY_LINK)?;
    let request = RequestPredicate {
        description: "a link code issued to the operator".into(),
        probe: link_probe(targets, |link, _| match link.code_created_ms {
            None => ProbeVerdict::Pending("no link code issued since the step began".into()),
            Some(created) => ProbeVerdict::Satisfied(
                ProbeVerdict::satisfied(format!("link code issued at {created}")).at(created),
            ),
        }),
    };
    let effects = vec![
        step_effect(
            row,
            server,
            "code_consumed",
            "link code consumed and the account linked to the Arma id it carried",
            link_probe(targets, code_consumed),
            &case,
        ),
        step_effect(
            row,
            server,
            "link_audit",
            "`identity.link` audit row recorded",
            link_probe(targets, link_audited),
            &case,
        ),
        step_effect(
            row,
            server,
            "player_listing",
            "listing recorded while the operator is connected",
            command_probe(targets, server, "list_players", |_| true, listing_measured),
            &case,
        ),
        step_effect(
            row,
            server,
            "listed_identity",
            "listing names the linked Arma id",
            Probe::measurement(&listed_arma_ids_key(), listed_identity),
            &case,
        ),
        step_effect(
            row,
            server,
            "link_status",
            "saved link status reads linked to the linked Arma id",
            Probe::browser_inbox(link_status),
            &case,
        ),
    ];
    single_server_step(row, server, StepKind::ChromeAction, Some(request), effects)
}

/// A probe of the operator's link row since the step began, judged by `judge`.
fn link_probe(
    targets: &WaveTargets,
    judge: fn(&IdentityLinkRow, &StepContext<'_>) -> ProbeVerdict,
) -> Probe {
    let container = targets.database_container.clone();
    let operator = targets.operator_discord_id.clone();
    Probe::host(
        move |context| single_server_reads::identity_link_since(&container, &operator, context),
        move |text, context| match json_rows::<IdentityLinkRow>(text) {
            Err(error) => ProbeVerdict::Contradicted(format!("{error:#}")),
            Ok(rows) => match rows.first() {
                None => ProbeVerdict::Contradicted("the operator has no account row".into()),
                Some(link) => judge(link, context),
            },
        },
    )
}

/// Holds when the code was consumed and the account carries the Arma id the code did; measures
/// that id.
fn code_consumed(link: &IdentityLinkRow, _: &StepContext<'_>) -> ProbeVerdict {
    let Some(consumed) = link.code_consumed_ms else {
        return ProbeVerdict::Pending("the link code is not consumed yet".into());
    };
    match (&link.code_arma_id, &link.arma_id) {
        (Some(code), Some(account)) if code == account => ProbeVerdict::Satisfied(
            ProbeVerdict::satisfied(format!(
                "link code consumed at {consumed}; the account is linked to Arma id {account}"
            ))
            .at(consumed)
            .measure(linked_arma_id_key(), account.clone()),
        ),
        (code, account) => ProbeVerdict::Contradicted(format!(
            "the link code was consumed for Arma id {code:?}, but the account is linked to \
             {account:?}"
        )),
    }
}

fn link_audited(link: &IdentityLinkRow, _: &StepContext<'_>) -> ProbeVerdict {
    match link.audit_ms {
        None => ProbeVerdict::Pending("no identity.link audit row since the step began".into()),
        Some(at) => ProbeVerdict::Satisfied(
            ProbeVerdict::satisfied(format!(
                "identity.link audited at {at}: {}",
                link.audit_message.as_deref().unwrap_or("(no message)")
            ))
            .at(at),
        ),
    }
}

/// Holds when the succeeded listing recorded its players; measures their distinct Arma ids, as a
/// count for the client count and as the list the linked id is looked up in.
fn listing_measured(
    row: &CommandRow,
    server: &FleetServer,
    satisfaction: Satisfaction,
) -> ProbeVerdict {
    let Some(arma_ids) = listed_arma_ids(row) else {
        return ProbeVerdict::Contradicted(format!(
            "list_players command {} succeeded without a players list",
            row.command_id
        ));
    };
    let distinct = u64::try_from(arma_ids.len()).unwrap_or(u64::MAX);
    let listed: Vec<Value> = arma_ids.into_iter().map(Value::from).collect();
    ProbeVerdict::Satisfied(
        Satisfaction {
            summary: format!(
                "{}; {distinct} distinct Arma ids listed",
                satisfaction.summary
            ),
            ..satisfaction
        }
        .measure(
            player_listing_measurement(IDENTITY_LINK_STEP.step_id, server.instance),
            distinct,
        )
        .measure(listed_arma_ids_key(), Value::Array(listed)),
    )
}

/// Holds when the listed Arma ids (the measurement's JSON array) include the linked one.
fn listed_identity(text: &str, context: &StepContext<'_>) -> ProbeVerdict {
    let Some(linked) = measured_text(context, &linked_arma_id_key()) else {
        return ProbeVerdict::Pending("the linked Arma id is not known yet".into());
    };
    let listed: Vec<String> = serde_json::from_str(text).unwrap_or_default();
    if listed.contains(&linked) {
        ProbeVerdict::Satisfied(ProbeVerdict::satisfied(format!(
            "the listing names the linked Arma id {linked}"
        )))
    } else {
        ProbeVerdict::Contradicted(format!(
            "the listing names {listed:?}, not the linked Arma id {linked}"
        ))
    }
}

/// Holds when the saved `/me/link/status` answer reads linked to the linked Arma id; an answer
/// not linked yet waits for a later capture.
fn link_status(text: &str, context: &StepContext<'_>) -> ProbeVerdict {
    let Some(linked) = measured_text(context, &linked_arma_id_key()) else {
        return ProbeVerdict::Pending("the linked Arma id is not known yet".into());
    };
    let page = compact(text);
    if !page.contains("\"linked\":true") {
        return ProbeVerdict::Pending("the saved link status does not read linked".into());
    }
    if page.contains(&format!("\"arma_id\":\"{linked}\"")) {
        ProbeVerdict::Satisfied(ProbeVerdict::satisfied(format!(
            "the saved link status reads linked to {linked}"
        )))
    } else {
        ProbeVerdict::Contradicted(format!(
            "the saved link status reads linked, but not to the Arma id {linked}"
        ))
    }
}

fn kick(targets: &WaveTargets, server: &FleetServer) -> Result<Step> {
    let row = &KICK_STEP;
    let case = CaseName::new(KICK)?;
    let effects = vec![
        step_effect(
            row,
            server,
            "command",
            "kick command succeeded",
            command_probe(targets, server, "kick", |_| true, kick_outcome),
            &case,
        ),
        step_effect(
            row,
            server,
            "kicked_identity",
            "kick removed the linked Arma id",
            Probe::measurement(&kicked_arma_id_key(), kicked_identity),
            &case,
        ),
        step_effect(
            row,
            server,
            "mod_log",
            "game runtime logged the kick",
            kick_logged(targets, server),
            &case,
        ),
    ];
    let request = kick_request(targets, server);
    single_server_step(row, server, StepKind::ChromeAction, Some(request), effects)
}

/// The first kick command for `server` since the step began.
fn kick_request(targets: &WaveTargets, server: &FleetServer) -> RequestPredicate {
    let container = targets.database_container.clone();
    let name = server.name.clone();
    RequestPredicate {
        description: format!("a kick command for {name}"),
        probe: Probe::host(
            move |context| fleet_reads::commands_since(&container, "kick", context),
            move |text, context| {
                let rows = json_rows::<CommandRow>(text)
                    .map(|rows| rows.into_iter().filter(|row| row.server == name).collect());
                first_request::<CommandRow>(rows, context, "kick command")
            },
        ),
    }
}

/// Holds when the succeeded kick recorded the Arma id it removed; measures it and the command.
fn kick_outcome(row: &CommandRow, _: &FleetServer, satisfaction: Satisfaction) -> ProbeVerdict {
    let kicked = row
        .outcome
        .as_ref()
        .and_then(|outcome| outcome.get("arma_id"))
        .and_then(Value::as_str);
    let Some(kicked) = kicked else {
        return ProbeVerdict::Contradicted(format!(
            "kick command {} succeeded without the kicked Arma id",
            row.command_id
        ));
    };
    ProbeVerdict::Satisfied(
        Satisfaction {
            summary: format!("{}; kicked Arma id {kicked}", satisfaction.summary),
            ..satisfaction
        }
        .measure(kick_command_key(), row.command_id.clone())
        .measure(kicked_arma_id_key(), kicked),
    )
}

/// Holds when the kicked Arma id (the measurement's JSON string) is the one W9 linked.
fn kicked_identity(text: &str, context: &StepContext<'_>) -> ProbeVerdict {
    let kicked: String = serde_json::from_str(text).unwrap_or_default();
    match measured_text(context, &linked_arma_id_key()) {
        None => ProbeVerdict::Contradicted(format!(
            "W9 measured no linked Arma id, so the kicked Arma id {kicked} cannot be compared"
        )),
        Some(linked) if linked == kicked => ProbeVerdict::Satisfied(ProbeVerdict::satisfied(
            format!("the kick removed the linked Arma id {linked}"),
        )),
        Some(linked) => ProbeVerdict::Contradicted(format!(
            "the kick removed Arma id {kicked}, not the linked Arma id {linked}"
        )),
    }
}

/// Holds when `server`'s newest `console.log` has the kick line of the measured kick command.
fn kick_logged(targets: &WaveTargets, server: &FleetServer) -> Probe {
    let fleet_root = targets.fleet_root.clone();
    let instance = server.instance;
    Probe::host(
        move |_| Ok(console_log_reader::newest(&fleet_root, instance)),
        move |text, context| {
            let Some(command) = measured_text(context, &kick_command_key()) else {
                return ProbeVerdict::Pending("the kick command is not known yet".into());
            };
            let Some(log) = console_log_reader::parse(text) else {
                return ProbeVerdict::Pending("the console log read names no log".into());
            };
            let wanted = format!("{KICK_LOG_LINE}{command} ");
            match log.text.lines().find(|line| line.contains(&wanted)) {
                Some(line) => ProbeVerdict::Satisfied(ProbeVerdict::satisfied(format!(
                    "{} logged `{}`",
                    log.path,
                    line.trim()
                ))),
                None => ProbeVerdict::Pending(format!(
                    "{} has no kick line for command {command} yet",
                    log.path
                )),
            }
        },
    )
}

fn ended_session_kick(targets: &WaveTargets, server: &FleetServer) -> Result<Step> {
    let row = &ENDED_SESSION_KICK_STEP;
    let case = CaseName::new(REJECTION_ENDED_SESSION_KICK)?;
    let effects = vec![
        step_effect(
            row,
            server,
            "refused",
            "kick naming an ended runtime session answered 409 RUNTIME_SESSION_ENDED",
            Probe::browser_inbox(kick_refused),
            &case,
        ),
        step_effect(
            row,
            server,
            "no_command",
            "no kick command recorded for the refused request",
            no_kick_recorded(targets, server),
            &case,
        ),
    ];
    single_server_step(row, server, StepKind::ChromeAction, None, effects)
}

/// Holds when the saved answer is the 409 `RUNTIME_SESSION_ENDED` refusal; measures when it was
/// read, so the ledger is read only after it.
fn kick_refused(text: &str, context: &StepContext<'_>) -> ProbeVerdict {
    let answer = compact(text);
    if answer.contains(RUNTIME_SESSION_ENDED) && answer.contains("409") {
        ProbeVerdict::Satisfied(
            ProbeVerdict::satisfied(format!("the saved answer is 409 {RUNTIME_SESSION_ENDED}"))
                .measure(refusal_read_key(), context.observed_unix_ms),
        )
    } else {
        ProbeVerdict::Contradicted(format!(
            "the saved answer is not the 409 {RUNTIME_SESSION_ENDED} refusal"
        ))
    }
}

/// Holds, once the refusal was read, when no kick command for `server` other than W10's first
/// was requested since the step began; such a command contradicts it as soon as it shows.
fn no_kick_recorded(targets: &WaveTargets, server: &FleetServer) -> Probe {
    let container = targets.database_container.clone();
    let name = server.name.clone();
    Probe::host(
        move |context| fleet_reads::commands_since(&container, "kick", context),
        move |text, context| {
            let rows = match json_rows::<CommandRow>(text) {
                Ok(rows) => rows,
                Err(error) => return ProbeVerdict::Contradicted(format!("{error:#}")),
            };
            let first_kick = measured_text(context, &kick_command_key());
            let since = since_ms(context);
            let accepted = rows.iter().find(|row| {
                row.server == name
                    && row.requested_ms >= since
                    && first_kick.as_deref() != Some(row.command_id.as_str())
            });
            match accepted {
                None if !context.measurements.contains_key(&refusal_read_key()) => {
                    ProbeVerdict::Pending("the refusal is not read yet".into())
                }
                None => ProbeVerdict::Satisfied(ProbeVerdict::satisfied(format!(
                    "no kick command for {name} recorded since the step began"
                ))),
                Some(kick) => ProbeVerdict::Contradicted(format!(
                    "kick command {} against runtime session {} was accepted ({})",
                    kick.command_id,
                    kick.arguments
                        .get("runtime_session_id")
                        .and_then(Value::as_str)
                        .unwrap_or("(none)"),
                    kick.state
                )),
            }
        },
    )
}
