//! A mission deployment in words: its state and transition, its detail, how a followed deployment
//! ended, and the choices a request offers.
//!
//! **Role:** names every state, transition and request channel; lays a deployment out as the rows
//! its detail shows; classifies a followed deployment into how it ended and announces that once;
//! and narrows the mission library and the operation calendar to the choices a deployment of this
//! server can make.
//! **Position:** read by the deployments panel's list, detail and request form.
//! **Signals & state:** none; pure over its arguments.
//! **Invariants:** a 202 records a deployment; only a runtime session reporting the artifact
//! confirms it, so only `confirmed` is announced as success. The missions offered are the live ones
//! whose latest approval names an artifact — exactly what the backend deploys — and the event
//! missions offered are those of operations scheduled on this server that run the chosen mission.
//! Instants are shown as UTC lines, so every sentence is testable natively.

use super::super::fleet_commands::command_wording::{
    state_label as command_state, OutcomeAnnouncer,
};
use crate::v2::core::api::dto::{EventHub, EventListItem, MissionCard, MissionDeployment};
use crate::v2::core::utils::utc_timestamp::utc_label;
use crate::v2::pages::mission_hub::mission_review::review_wording::short_digest;

/// A deployment state as the panel names it.
pub(crate) fn state_label(state: &str) -> String {
    match state {
        "requested" => "In flight".to_string(),
        "confirmed" => "Confirmed".to_string(),
        "failed" => "Failed".to_string(),
        "cancelled" => "Cancelled".to_string(),
        other => other.replace('_', " "),
    }
}

/// The badge variant a deployment state is shown in.
pub(crate) fn state_tone(state: &str) -> &'static str {
    match state {
        "requested" => "primary",
        "confirmed" => "success",
        "failed" => "error",
        _ => "neutral",
    }
}

/// Whether a deployment is still waiting for a runtime session to confirm it.
pub(crate) fn in_flight(state: &str) -> bool {
    state == "requested"
}

/// How a deployment reaches the running server.
pub(crate) fn transition_label(transition: &str) -> String {
    match transition {
        "scenario_restart" => {
            "Scenario restart — the running game loads the artifact and restarts in-process"
                .to_string()
        }
        "host_restart" => {
            "Host restart — the host agent restarts the server on the terrain's scenario"
                .to_string()
        }
        other => other.replace('_', " "),
    }
}

/// Where a deployment was requested from.
pub(crate) fn via_label(via: &str) -> &str {
    match via {
        "web" => "the website",
        "game_runtime" => "in game",
        other => other,
    }
}

/// An account as shown: "you" for the viewer's own.
fn account(account: &str, me: Option<&str>) -> String {
    if me == Some(account) {
        "you".to_string()
    } else {
        account.to_string()
    }
}

/// A deployment's detail, as `(label, value)` rows in reading order.
pub(crate) fn detail_rows(d: &MissionDeployment, me: Option<&str>) -> Vec<(&'static str, String)> {
    let mut rows = vec![
        ("Mission", d.mission_title.clone()),
        ("State", state_label(&d.state)),
        ("Artifact digest", d.artifact_digest.clone()),
        ("Document SHA-256", d.artifact_sha256.clone()),
        ("Terrain", d.terrain_key.clone()),
        ("Scenario", d.scenario_id.clone()),
        ("Transition", transition_label(&d.transition)),
        (
            "Requested",
            format!(
                "by {} from {}, {}",
                account(&d.requested_by, me),
                via_label(&d.requested_via),
                utc_label(&d.requested_at)
            ),
        ),
        ("Deadline", utc_label(&d.deadline_at)),
        ("Bound seats", d.bound_slots.to_string()),
        (
            "Fleet command",
            format!(
                "{} — {}",
                d.fleet_command_id,
                command_state(&d.fleet_command_state)
            ),
        ),
    ];
    if let Some(event_mission) = &d.event_mission_id {
        rows.push(("Event mission", event_mission.clone()));
    }
    if let Some(session) = &d.confirmed_runtime_session_id {
        rows.push(("Confirmed by runtime session", session.clone()));
    }
    if let Some(finished) = &d.finished_at {
        rows.push(("Finished", utc_label(finished)));
    }
    if let Some(reason) = &d.failure_reason {
        rows.push(("Failure reason", reason.clone()));
    }
    rows
}

/// A deployment's one-line summary in the list.
pub(crate) fn summary_line(d: &MissionDeployment) -> String {
    format!(
        "{} · artifact {} · requested {}",
        d.terrain_key,
        short_digest(&d.artifact_digest),
        utc_label(&d.requested_at)
    )
}

/// Announce how a followed deployment ended, once; `false` while it is still in flight.
pub(crate) fn announce_deployment(
    d: &MissionDeployment,
    announcer: &impl OutcomeAnnouncer,
) -> bool {
    match d.state.as_str() {
        "requested" => return false,
        "confirmed" => announcer.succeeded(format!(
            "{} is confirmed — a runtime session reported artifact {}",
            d.mission_title,
            short_digest(&d.artifact_digest)
        )),
        "failed" => announcer.failed(format!(
            "The deployment of {} failed: {}",
            d.mission_title,
            d.failure_reason
                .as_deref()
                .unwrap_or("no reason was recorded")
        )),
        "cancelled" => announcer.noted(format!(
            "The deployment of {} was cancelled before its command was claimed",
            d.mission_title
        )),
        other => announcer.noted(format!(
            "The deployment of {} is in a state this page does not know ({other})",
            d.mission_title
        )),
    }
    true
}

/// The newest confirmed deployment's runtime session, among deployments listed newest first.
pub(crate) fn latest_confirmed_session(deployments: &[MissionDeployment]) -> Option<String> {
    deployments
        .iter()
        .find(|d| d.state == "confirmed")
        .and_then(|d| d.confirmed_runtime_session_id.clone())
}

/// A mission a deployment may run: live, with the artifact its latest approval decided.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DeployableMission {
    pub(crate) mission_id: String,
    pub(crate) title: String,
    pub(crate) artifact_id: String,
}

/// The live missions whose latest approval names an artifact, in the order the library lists them.
pub(crate) fn deployable_missions(cards: &[MissionCard]) -> Vec<DeployableMission> {
    cards
        .iter()
        .filter(|m| m.status == "live")
        .filter_map(|m| {
            Some(DeployableMission {
                mission_id: m.id.clone(),
                title: m.title.clone(),
                artifact_id: m.approved_artifact_id.clone()?,
            })
        })
        .collect()
}

/// The operations scheduled on `server_id`.
pub(crate) fn operations_on_server<'a>(
    operations: &'a [EventListItem],
    server_id: &str,
) -> Vec<&'a EventListItem> {
    operations
        .iter()
        .filter(|o| o.server_id.as_deref() == Some(server_id))
        .collect()
}

/// One event mission a deployment may bind: the operation it belongs to, and when it runs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct EventMissionChoice {
    pub(crate) event_mission_id: String,
    pub(crate) mission_id: String,
    pub(crate) label: String,
}

/// The event missions of one operation on this server, labelled for the picker.
pub(crate) fn event_mission_choices(operation: &EventHub) -> Vec<EventMissionChoice> {
    let name = operation
        .name_override
        .clone()
        .unwrap_or_else(|| "Operation".to_string());
    operation
        .missions
        .iter()
        .map(|m| EventMissionChoice {
            event_mission_id: m.event_mission_id.clone(),
            mission_id: m.mission_id.clone(),
            label: format!("{name} — {} ({})", m.title, utc_label(&m.start_time)),
        })
        .collect()
}
