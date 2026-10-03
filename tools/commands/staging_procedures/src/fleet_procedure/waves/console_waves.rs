//! W4 and W5: the custom console command `#players` and the player listing on every fleet
//! server, each judged by the outcome the ledger recorded.
//!
//! **Role:** builds the two steps whose effect per server is a succeeded command with its
//! outcome: the console response for `server<N>_custom_console`, the listed players for
//! `server<N>_list_players`.
//!
//! **Position:** called by `waves/mod.rs`; builds on `shared_probes.rs`; the listing's distinct
//! Arma ids are measured for `judge_mapping.rs`'s client count, and W9 reads a listing through
//! [`listed_arma_ids`].
//!
//! **Signals & state:** none; builders and judges.
//!
//! **Invariants:** only a console command whose line is exactly `#players` counts; a succeeded
//! console command without a recorded response, or a listing without a players array,
//! contradicts its effect; the listing measures the number of distinct non-empty Arma ids.

use std::collections::BTreeSet;

use crate::error::Result;
use serde_json::Value;

use super::shared_probes::{CommandExpectation, command_effect, command_request, wave_step};
use super::wave_table::{CONSOLE_WAVE, LIST_PLAYERS_WAVE};
use super::{FleetServer, WaveTargets};
use crate::fleet_procedure::fleet_cases::per_server_case;
use crate::fleet_procedure::fleet_reads::CommandRow;
use crate::fleet_procedure::judge_mapping::player_listing_measurement;
use crate::procedure_runner::step::{ProbeVerdict, Satisfaction, Step};

/// The console line W4 sends to every server.
pub(crate) const CONSOLE_LINE: &str = "#players";

/// The steps of W4 and W5.
pub(super) fn steps(targets: &WaveTargets) -> Result<Vec<Step>> {
    Ok(vec![console_wave(targets)?, listing_wave(targets)?])
}

fn console_wave(targets: &WaveTargets) -> Result<Step> {
    let mut effects = Vec::new();
    for server in &targets.servers {
        effects.push(command_effect(
            &CONSOLE_WAVE,
            targets,
            server,
            CommandExpectation {
                action: "console_command",
                description: format!("console command `{CONSOLE_LINE}` answered and recorded"),
                accepts: |row| {
                    row.arguments.get("line").and_then(Value::as_str) == Some(CONSOLE_LINE)
                },
                on_success: console_response,
            },
            per_server_case(server.instance, "custom_console")?,
        ));
    }
    wave_step(
        &CONSOLE_WAVE,
        targets,
        command_request(targets, "console_command"),
        effects,
    )
}

fn listing_wave(targets: &WaveTargets) -> Result<Step> {
    let mut effects = Vec::new();
    for server in &targets.servers {
        effects.push(command_effect(
            &LIST_PLAYERS_WAVE,
            targets,
            server,
            CommandExpectation {
                action: "list_players",
                description: "player listing recorded".to_string(),
                accepts: |_| true,
                on_success: listing_recorded,
            },
            per_server_case(server.instance, "list_players")?,
        ));
    }
    wave_step(
        &LIST_PLAYERS_WAVE,
        targets,
        command_request(targets, "list_players"),
        effects,
    )
}

/// Holds when the succeeded console command recorded the server's response.
fn console_response(
    row: &CommandRow,
    _server: &FleetServer,
    satisfaction: Satisfaction,
) -> ProbeVerdict {
    let outcome = row.outcome.as_ref();
    let Some(response) = outcome
        .and_then(|outcome| outcome.get("response"))
        .and_then(Value::as_str)
    else {
        return ProbeVerdict::Contradicted(format!(
            "console command {} succeeded without a recorded response",
            row.command_id
        ));
    };
    let truncated = outcome
        .and_then(|outcome| outcome.get("response_truncated"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    ProbeVerdict::Satisfied(Satisfaction {
        summary: format!(
            "{}; a response of {} bytes recorded{}",
            satisfaction.summary,
            response.len(),
            if truncated { ", truncated" } else { "" }
        ),
        ..satisfaction
    })
}

/// Holds when the succeeded listing recorded its players; measures the distinct Arma ids.
fn listing_recorded(
    row: &CommandRow,
    server: &FleetServer,
    satisfaction: Satisfaction,
) -> ProbeVerdict {
    let (Some(players), Some(arma_ids)) = (listed_players(row), listed_arma_ids(row)) else {
        return ProbeVerdict::Contradicted(format!(
            "list_players command {} succeeded without a players list",
            row.command_id
        ));
    };
    let distinct = u64::try_from(arma_ids.len()).unwrap_or(u64::MAX);
    ProbeVerdict::Satisfied(
        Satisfaction {
            summary: format!(
                "{}; {} players listed, {distinct} distinct Arma ids",
                satisfaction.summary,
                players.len()
            ),
            ..satisfaction
        }
        .measure(
            player_listing_measurement(LIST_PLAYERS_WAVE.step_id, server.instance),
            distinct,
        ),
    )
}

/// The players array a listing's outcome recorded.
fn listed_players(row: &CommandRow) -> Option<&Vec<Value>> {
    row.outcome
        .as_ref()
        .and_then(|outcome| outcome.get("players"))
        .and_then(Value::as_array)
}

/// The distinct non-empty Arma ids a listing's outcome recorded, or `None` when it recorded no
/// players array.
pub(super) fn listed_arma_ids(row: &CommandRow) -> Option<BTreeSet<&str>> {
    listed_players(row).map(|players| {
        players
            .iter()
            .filter_map(|player| player.get("arma_id").and_then(Value::as_str))
            .filter(|arma_id| !arma_id.is_empty())
            .collect()
    })
}
