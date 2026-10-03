//! W1–W3: every fleet server stopped, started and restarted through Server Control, with the
//! unit's process and the runtime-session generations that follow each command.
//!
//! **Role:** builds the steps of the three process waves: per server, the command's success,
//! the unit's state or new process, the new runtime-session generation and the end of the
//! previous generation.
//!
//! **Position:** called by `waves/mod.rs`; builds on `shared_probes.rs` and reads
//! `fleet_procedure/fleet_reads.rs`'s commands, sessions and unit state.
//!
//! **Signals & state:** none; builders and boxed judges.
//!
//! **Invariants:** W1 holds a server only when its stop succeeded and its unit is `inactive`
//! (a `failed` unit contradicts it); W2 and W3 hold `server<N>_start` and `server<N>_restart`
//! only when the command succeeded, the unit runs a process (for W3 another than W2's) and a
//! generation started after the server's own request heartbeats; the succession case holds only
//! when that generation is the one before it plus 1, the one before it carries a recorded end
//! reason, and the server has no second open session.

use crate::error::Result;

use super::shared_probes::{
    command_request, command_succeeded, new_process, server_effect, unit_probe, wave_step,
};
use super::wave_table::{FleetWave, RESTART_WAVE, START_WAVE, STOP_WAVE};
use super::{FleetServer, WaveTargets};
use crate::fleet_procedure::fleet_cases::per_server_case;
use crate::fleet_procedure::fleet_reads::{self, SessionRow, json_rows, newest_since, since_ms};
use crate::procedure_runner::step::{Probe, ProbeVerdict, Step};

/// The steps of W1, W2 and W3.
pub(super) fn steps(targets: &WaveTargets) -> Result<Vec<Step>> {
    Ok(vec![
        stop_wave(targets)?,
        process_start_wave(targets, &START_WAVE, "start", None)?,
        process_start_wave(targets, &RESTART_WAVE, "restart", Some(START_WAVE.step_id))?,
    ])
}

fn stop_wave(targets: &WaveTargets) -> Result<Step> {
    let wave = &STOP_WAVE;
    let mut effects = Vec::new();
    for server in &targets.servers {
        let case = per_server_case(server.instance, "stop")?;
        effects.push(command_succeeded(
            wave,
            targets,
            server,
            "stop",
            case.clone(),
        ));
        let inactive = unit_probe(server, |state, _| match state.active_state.as_str() {
            "inactive" => ProbeVerdict::Satisfied(ProbeVerdict::satisfied(format!(
                "{} inactive ({})",
                state.unit, state.sub_state
            ))),
            "failed" => ProbeVerdict::Contradicted(format!(
                "{} failed ({}) instead of stopping cleanly",
                state.unit, state.sub_state
            )),
            other => {
                ProbeVerdict::Pending(format!("{} is {other}/{}", state.unit, state.sub_state))
            }
        });
        effects.push(server_effect(
            wave,
            server,
            "unit",
            "game server unit inactive",
            inactive,
            case,
        ));
    }
    wave_step(wave, targets, command_request(targets, "stop"), effects)
}

/// W2 or W3: `action` (`start` or `restart`) on every server; the process must differ from the
/// one `previous_process` measured, when named.
fn process_start_wave(
    targets: &WaveTargets,
    wave: &FleetWave,
    action: &'static str,
    previous_process: Option<&'static str>,
) -> Result<Step> {
    let mut effects = Vec::new();
    for server in &targets.servers {
        let case = per_server_case(server.instance, action)?;
        let succession = per_server_case(server.instance, "runtime_session_succession")?;
        effects.push(command_succeeded(
            wave,
            targets,
            server,
            action,
            case.clone(),
        ));
        effects.push(server_effect(
            wave,
            server,
            "process",
            "game server unit runs a new process",
            new_process(wave.step_id, server, previous_process),
            case.clone(),
        ));
        effects.push(server_effect(
            wave,
            server,
            "session",
            "new runtime-session generation heartbeats",
            session_probe(targets, server, action, new_generation),
            case,
        ));
        effects.push(server_effect(
            wave,
            server,
            "succession",
            "runtime-session generation follows the previous one, which ended, and is the only \
             open session",
            session_probe(targets, server, action, generation_succession),
            succession,
        ));
    }
    wave_step(wave, targets, command_request(targets, action), effects)
}

/// A probe of the sessions around `server`'s newest `action` command since the step began,
/// judged by `judge`.
fn session_probe(
    targets: &WaveTargets,
    server: &FleetServer,
    action: &'static str,
    judge: fn(&SessionRow) -> ProbeVerdict,
) -> Probe {
    let container = targets.database_container.clone();
    let name = server.name.clone();
    Probe::host(
        move |context| fleet_reads::sessions_since(&container, action, context),
        move |text, context| match json_rows::<SessionRow>(text) {
            Err(error) => ProbeVerdict::Contradicted(format!("{error:#}")),
            Ok(rows) => match newest_since(&rows, &name, since_ms(context)) {
                None => ProbeVerdict::Pending(format!("no {action} command for {name} yet")),
                Some(row) => judge(row),
            },
        },
    )
}

/// Holds when a generation started at or after the request is open and has heartbeated; the
/// heartbeat's own time is the effect's time.
fn new_generation(row: &SessionRow) -> ProbeVerdict {
    let Some(session) = &row.new_session else {
        return ProbeVerdict::Pending("no runtime session started after the request yet".into());
    };
    let started = session.started_ms.map_or_else(String::new, |started| {
        format!(
            " {} ms after the request",
            started.saturating_sub(row.requested_ms)
        )
    });
    match (session.heartbeat_ms, session.end_reason.as_deref()) {
        (_, Some(reason)) => ProbeVerdict::Pending(format!(
            "generation {} started{started} and ended {reason}",
            session.generation
        )),
        (None, None) => ProbeVerdict::Pending(format!(
            "generation {} started{started} and has not heartbeated yet",
            session.generation
        )),
        (Some(heartbeat), None) => ProbeVerdict::Satisfied(
            ProbeVerdict::satisfied(format!(
                "generation {} started{started} and heartbeats",
                session.generation
            ))
            .at(heartbeat),
        ),
    }
}

/// The end reasons a runtime session records: a newer generation began, its heartbeats
/// lapsed, the runtime ended it, or its credential was revoked.
const RECORDED_END_REASONS: [&str; 4] = [
    "superseded",
    "expired",
    "ended_by_runtime",
    "credential_revoked",
];

/// Holds when the generation started after the request is the newest generation started before
/// it plus 1, that predecessor carries a recorded end reason, and the server has at most one
/// open session; a gap in the generations, an unknown end reason or two open sessions
/// contradict it.
fn generation_succession(row: &SessionRow) -> ProbeVerdict {
    let Some(new) = &row.new_session else {
        return ProbeVerdict::Pending("no newer runtime-session generation yet".into());
    };
    let Some(previous) = &row.previous_session else {
        return ProbeVerdict::Contradicted(
            "no runtime session started before the request, so no generation preceded it".into(),
        );
    };
    if row.open_sessions > 1 {
        return ProbeVerdict::Contradicted(format!(
            "{} has {} open runtime sessions",
            row.server, row.open_sessions
        ));
    }
    if new.generation != previous.generation + 1 {
        return ProbeVerdict::Contradicted(format!(
            "generation {} follows generation {}, not generation {}",
            new.generation,
            previous.generation,
            previous.generation + 1
        ));
    }
    match previous.end_reason.as_deref() {
        None => ProbeVerdict::Pending(format!("generation {} is still open", previous.generation)),
        Some(reason) if RECORDED_END_REASONS.contains(&reason) => ProbeVerdict::Satisfied(
            ProbeVerdict::satisfied(format!(
                "generation {} (last heartbeat {}) ended {reason}; generation {} follows it and \
                 is the only open session",
                previous.generation,
                previous
                    .heartbeat_ms
                    .map_or_else(|| "none".to_string(), |at| at.to_string()),
                new.generation
            ))
            .at(previous.ended_ms.unwrap_or(row.requested_ms)),
        ),
        Some(reason) => ProbeVerdict::Contradicted(format!(
            "generation {} ended with the unknown reason {reason:?}",
            previous.generation
        )),
    }
}
