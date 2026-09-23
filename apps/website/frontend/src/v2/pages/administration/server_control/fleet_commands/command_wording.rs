//! A fleet command in words: its action and state, its arguments and outcome, how a followed command
//! ended, the checks a request passes before it is sent, and what a refusal is told.
//!
//! **Role:** names every action and state, summarises a receipt's arguments and outcome, reads the
//! players a player listing reported, classifies a followed receipt into how it ended and announces
//! that exactly once, validates a broadcast and a kick as the backend does, and words a refused
//! request or cancellation.
//! **Position:** read by the command console's request controls, its followed-command panel and its
//! history.
//! **Signals & state:** none; pure over its arguments.
//! **Invariants:** a 202 is an acceptance, never an outcome: only a receipt that reached
//! `succeeded` is announced as success. `indeterminate` is announced as unknown — the executor
//! stopped reporting after the effect may have started and nothing repeats the command — never as
//! success or failure. A state this build does not know is announced as unknown rather than
//! followed forever. Instants are shown as UTC lines, so every sentence is testable natively.

use crate::v2::core::api::client::ApiRefusal;
use crate::v2::core::api::dto::FleetCommandReceipt;
use crate::v2::core::utils::utc_timestamp::utc_label;
use serde_json::{Map, Value};

/// An action as the console names it.
pub(crate) fn action_label(action: &str) -> String {
    match action {
        "start" => "Start".to_string(),
        "stop" => "Stop".to_string(),
        "restart" => "Restart".to_string(),
        "list_players" => "List players".to_string(),
        "broadcast" => "Broadcast".to_string(),
        "kick" => "Kick".to_string(),
        "load_mission" => "Load mission (issued by a deployment)".to_string(),
        "restart_with_mission" => "Restart with mission (issued by a deployment)".to_string(),
        other => other.replace('_', " "),
    }
}

/// A state as the console names it.
pub(crate) fn state_label(state: &str) -> String {
    match state {
        "queued" => "Queued".to_string(),
        "claimed" => "Claimed".to_string(),
        "executing" => "Executing".to_string(),
        "succeeded" => "Succeeded".to_string(),
        "failed" => "Failed".to_string(),
        "expired" => "Expired".to_string(),
        "cancelled" => "Cancelled".to_string(),
        "indeterminate" => "Outcome unknown".to_string(),
        other => other.replace('_', " "),
    }
}

/// The badge variant a state is shown in.
pub(crate) fn state_tone(state: &str) -> &'static str {
    match state {
        "queued" | "claimed" | "executing" => "primary",
        "succeeded" => "success",
        "failed" | "expired" => "error",
        "indeterminate" => "warning",
        _ => "neutral",
    }
}

/// Whether a command in `state` is still on its way to an outcome.
pub(crate) fn in_flight(state: &str) -> bool {
    matches!(state, "queued" | "claimed" | "executing")
}

/// One argument or outcome value as a reader wants it: text bare, anything else as compact JSON.
fn value_text(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

/// Every key and value of an argument or outcome object, in the order the backend sent them.
fn fields_text(fields: &Map<String, Value>) -> String {
    if fields.is_empty() {
        return "—".to_string();
    }
    fields
        .iter()
        .map(|(key, value)| format!("{key}: {}", value_text(value)))
        .collect::<Vec<_>>()
        .join(" · ")
}

/// A receipt's arguments, summarised.
pub(crate) fn arguments_summary(receipt: &FleetCommandReceipt) -> String {
    fields_text(&receipt.arguments)
}

/// The players a player listing reported, as `(arma_id, name)`.
pub(crate) fn listed_players(receipt: &FleetCommandReceipt) -> Vec<(String, String)> {
    receipt
        .outcome
        .as_ref()
        .and_then(|outcome| outcome.get("players"))
        .and_then(Value::as_array)
        .map(|players| {
            players
                .iter()
                .filter_map(|p| {
                    let arma_id = p.get("arma_id")?.as_str()?.to_string();
                    let name = p.get("name").and_then(Value::as_str).unwrap_or_default();
                    Some((arma_id, name.to_string()))
                })
                .collect()
        })
        .unwrap_or_default()
}

/// The newest player listing that succeeded, among receipts listed newest first.
pub(crate) fn latest_player_listing(
    receipts: &[FleetCommandReceipt],
) -> Option<&FleetCommandReceipt> {
    receipts
        .iter()
        .find(|r| r.action == "list_players" && r.state == "succeeded")
}

/// What a receipt's executor observed, summarised; a player listing names its players.
pub(crate) fn outcome_summary(receipt: &FleetCommandReceipt) -> Option<String> {
    let outcome = receipt.outcome.as_ref()?;
    if receipt.action == "list_players" {
        let players = listed_players(receipt);
        let names: Vec<String> = players
            .iter()
            .map(|(arma_id, name)| format!("{name} ({arma_id})"))
            .collect();
        let unread = outcome
            .get("raw_lines")
            .and_then(Value::as_array)
            .map(|lines| {
                format!(
                    " · {} line(s) of the response were not understood",
                    lines.len()
                )
            })
            .unwrap_or_default();
        return Some(match names.len() {
            0 => format!("No players connected{unread}"),
            n => format!("{n} player(s): {}{unread}", names.join(", ")),
        });
    }
    Some(fields_text(outcome))
}

/// How a followed command ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ReceiptOutcome {
    /// The executor reported the effect done.
    Succeeded(String),
    /// The executor reported a failure, or no executor claimed the command before it expired.
    Failed(String),
    /// The command was cancelled before any executor claimed it: nothing ran.
    NotRun(String),
    /// Nobody knows: the executor stopped reporting after the effect may have started.
    Unknown(String),
}

/// How a receipt ended, or `None` while it is still on its way to an outcome.
pub(crate) fn receipt_outcome(receipt: &FleetCommandReceipt) -> Option<ReceiptOutcome> {
    let action = action_label(&receipt.action);
    let reason = receipt.failure_reason.as_deref();
    match receipt.state.as_str() {
        "queued" | "claimed" | "executing" => None,
        "succeeded" => Some(ReceiptOutcome::Succeeded(match outcome_summary(receipt) {
            Some(observed) => format!("{action} succeeded — {observed}"),
            None => format!("{action} succeeded"),
        })),
        "failed" => Some(ReceiptOutcome::Failed(format!(
            "{action} failed: {}",
            reason.unwrap_or("the executor gave no reason")
        ))),
        "expired" => Some(ReceiptOutcome::Failed(format!(
            "{action} expired — no executor carried it out before {}, so nothing ran",
            utc_label(&receipt.expires_at)
        ))),
        "cancelled" => Some(ReceiptOutcome::NotRun(format!(
            "{action} was cancelled before any executor claimed it — nothing ran"
        ))),
        "indeterminate" => Some(ReceiptOutcome::Unknown(format!(
            "{action}: the outcome is unknown. The executor stopped reporting after the command \
             started, so it may or may not have taken effect, and nothing repeats it. Inspect the \
             server before issuing it again."
        ))),
        other => Some(ReceiptOutcome::Unknown(format!(
            "{action} is in a state this page does not know ({other}); read the command history \
             for its outcome"
        ))),
    }
}

/// Where the outcome of a followed command is announced.
pub(crate) trait OutcomeAnnouncer {
    /// A success.
    fn succeeded(&self, text: String);
    /// A failure.
    fn failed(&self, text: String);
    /// Neither: nothing ran, or nobody knows what happened.
    fn noted(&self, text: String);
}

impl OutcomeAnnouncer for crate::v2::core::ui::toast::Toasts {
    fn succeeded(&self, text: String) {
        self.success(text);
    }
    fn failed(&self, text: String) {
        self.error(text);
    }
    fn noted(&self, text: String) {
        self.message(text);
    }
}

/// Announce how a followed command ended, once; `false` while it is still in flight and nothing
/// was announced.
pub(crate) fn announce_receipt(
    receipt: &FleetCommandReceipt,
    announcer: &impl OutcomeAnnouncer,
) -> bool {
    match receipt_outcome(receipt) {
        Some(ReceiptOutcome::Succeeded(text)) => announcer.succeeded(text),
        Some(ReceiptOutcome::Failed(text)) => announcer.failed(text),
        Some(ReceiptOutcome::NotRun(text) | ReceiptOutcome::Unknown(text)) => announcer.noted(text),
        None => return false,
    }
    true
}

/// What an accepted command is told: accepted, not done.
pub(crate) fn accepted_line(receipt: &FleetCommandReceipt) -> String {
    format!(
        "{} accepted — waiting for the {} to carry it out",
        action_label(&receipt.action),
        super::super::machine_credentials::executor_label(&receipt.executor_kind).to_lowercase()
    )
}

/// Printable text, trimmed, of 1 to `max` bytes — the backend's argument rule.
fn bounded_text(text: &str, what: &str, max: usize) -> Result<String, String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(format!("Enter the {what}"));
    }
    if trimmed.len() > max {
        return Err(format!("The {what} is too long: at most {max} bytes"));
    }
    if trimmed.chars().any(char::is_control) {
        return Err(format!(
            "The {what} cannot hold line breaks or control characters"
        ));
    }
    Ok(trimmed.to_string())
}

/// A broadcast message as the backend accepts it: 1 to 256 bytes, no control characters.
pub(crate) fn validated_broadcast(message: &str) -> Result<String, String> {
    bounded_text(message, "broadcast message", 256)
}

/// Whether `text` is a hyphenated UUID.
pub(crate) fn is_uuid(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.len() == 36
        && bytes.iter().enumerate().all(|(i, b)| match i {
            8 | 13 | 18 | 23 => *b == b'-',
            _ => b.is_ascii_hexdigit(),
        })
}

/// A kick's arguments as the backend accepts them: the Arma identity (1 to 128 bytes), the runtime
/// session it is issued against (a UUID), and an optional reason (1 to 128 bytes).
pub(crate) fn validated_kick(
    arma_id: &str,
    runtime_session_id: &str,
    reason: &str,
) -> Result<(String, String, Option<String>), String> {
    let arma_id = bounded_text(arma_id, "player's Arma identity", 128)?;
    let session = runtime_session_id.trim();
    if !is_uuid(session) {
        return Err("Enter the runtime session the kick is issued against, as its id".to_string());
    }
    let reason = match reason.trim() {
        "" => None,
        text => Some(bounded_text(text, "kick reason", 128)?),
    };
    Ok((arma_id, session.to_string(), reason))
}

/// What a refused request or cancellation is told; `fallback` when the backend sent no sentence.
pub(crate) fn command_refusal_sentence(refusal: &ApiRefusal, fallback: &str) -> String {
    match refusal.code() {
        Some("COMMAND_NOT_CANCELLABLE") => {
            let state = refusal.detail("state").unwrap_or("no longer queued");
            format!(
                "The command is {} — an executor has taken it up, so it can no longer be \
                 cancelled.",
                state_label(state).to_lowercase()
            )
        }
        Some("RUNTIME_SESSION_ENDED") => "That runtime session is not the server's open session \
                                          any more — the game server has restarted since. Issue \
                                          the kick against the session running now."
            .to_string(),
        _ => refusal.message_or(fallback),
    }
}
