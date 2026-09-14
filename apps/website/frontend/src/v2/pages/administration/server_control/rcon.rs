//! The RCON channel: the reply the host agent sends back, and the request that asks for one.
//!
//! **Role:** the 202 body's shape, the four request bodies the action enum accepts, the reading of
//! a reply into success or failure, the sentence the operator is shown, and the send itself.
//! **Position:** behind every control on the server screen — the header's restart, the quick
//! actions and the console's command line all reach the host through [`post_rcon`].
//! **Signals & state:** `console_log` collects the transcript and `busy` gates the controls while a
//! request is out; both are owned by the screen and passed in.
//! **Invariants:** a 2xx status is **not** a delivery. The reply carries both whether the command
//! reached the host and whether the unit ended up where it was told, and success is the conjunction
//! — reading either one alone is how a green notice ends up over a command that never landed. Every
//! field of the reply is required: a body missing one fails the parse and the page reports a failed
//! request rather than formatting a sentence out of a default nobody sent. The wording of every
//! outcome interpolates the state the host observed, because that observation is the only evidence
//! behind the word "delivered": restarting a unit can report success over a dead game server, which
//! is why the host re-reads the unit instead of trusting an exit status.
#![allow(dead_code)]

use leptos::prelude::*;
use serde::Deserialize;
use serde_json::{json, Value};

/// The 202 body of `POST /admin/servers/{id}/rcon`.
///
/// Mirrors the API's serializer. The `audited` field it also sends is not modelled, because nothing
/// on this page renders it; the five below are the claim being made.
///
/// Nothing is defaulted, deliberately: a body missing a field must fail the parse rather than fall
/// back to a value that reads as an outcome. A deserialize failure surfaces as a failed request, so
/// the page reports what it does not know instead of inventing what it does.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(super) struct RconAccepted {
    /// The unit was re-read in the state the action intended. On its own this is **not** permission
    /// to report success — see [`rcon_reports_success`].
    pub(super) accepted: bool,
    /// The action the host was asked to perform.
    pub(super) action: String,
    /// The host agent took the command and ran the verb. True even when `accepted` is false — the
    /// verb ran, the unit did not get where it was told. The two are kept apart because collapsing
    /// them sends an operator hunting a network fault over a unit fault.
    pub(super) delivered: bool,
    /// The unit's service state, re-read **after** the action. This is the evidence: restarting a
    /// unit exits cleanly over a dead game server, so the host never trusts an exit status and
    /// reports what it actually observed.
    pub(super) state: String,
    /// The agent's human-readable note about that observation.
    pub(super) detail: String,
}

/// Path of the admin RCON route for one server; tracks the API's router.
pub(super) fn admin_server_rcon_path(server_id: &str) -> String {
    format!("/admin/servers/{server_id}/rcon")
}

/// Request body asking the host to restart the server.
pub(super) fn rcon_body_restart() -> Value {
    json!({ "action": "restart" })
}

/// Request body asking the host to change the running map.
pub(super) fn rcon_body_change_map(map: &str) -> Value {
    json!({ "action": "change_map", "map": map })
}

/// Request body carrying a raw console command.
pub(super) fn rcon_body_custom(command: &str) -> Value {
    json!({ "action": "custom", "command": command })
}

/// Request body asking the host to kick a player.
pub(super) fn rcon_body_kick() -> Value {
    json!({ "action": "kick" })
}

/// What to do with a prompt answer for a field that must not be empty.
#[derive(Debug, PartialEq)]
pub(super) enum PromptField {
    /// The prompt was dismissed: do nothing at all.
    Abort,
    /// The answer was blank or whitespace: refuse it and say so.
    Reject,
    /// A usable answer, trimmed.
    Send(String),
}

/// Read a prompt answer into one of the three outcomes.
///
/// Dismissing the prompt and answering it with spaces are different intentions, and only the second
/// deserves an error.
pub(super) fn classify_prompt_field(answer: Option<&str>) -> PromptField {
    match answer {
        None => PromptField::Abort,
        Some(raw) => {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                PromptField::Reject
            } else {
                PromptField::Send(trimmed.to_string())
            }
        }
    }
}

/// Whether a reply reports a delivered, confirmed command.
///
/// Both fields, never one: acceptance is the agent's verdict about the unit and delivery is about
/// the channel. The status code is not consulted — it is what got us into this arm, not evidence
/// about the host.
pub(super) fn rcon_reports_success(resp: &RconAccepted) -> bool {
    resp.delivered && resp.accepted
}

/// The transcript and notification line for a reply — only what the host reported observing.
///
/// Each combination of the two booleans gets its own wording and every one of them interpolates the
/// observed state, so the sentence cannot read as success in any direction the reply does not
/// support. The delivered-but-not-accepted and not-delivered shapes are the ones that matter: the
/// word "accepted" never appears unqualified when nothing reached the host. The fourth combination
/// — not delivered, yet accepted — is unreachable through today's serializer and is written out
/// anyway, because it is exactly the shape this page must never render as success, and what the
/// server can currently emit is a property of the server, not of this function.
pub(super) fn rcon_accepted_message(resp: &RconAccepted) -> String {
    let (action, state, detail) = (&resp.action, &resp.state, &resp.detail);
    match (resp.delivered, resp.accepted) {
        (true, true) => format!(
            "RCON delivered action={action} — host agent re-read the unit as state={state} ({detail})"
        ),
        (true, false) => format!(
            "RCON delivered action={action} but the unit did NOT reach the expected state \
             (state={state}; {detail})"
        ),
        (false, true) => format!(
            "RCON NOT delivered action={action} — host reported acceptance of a command it never \
             carried; treat as FAILED (state={state}; {detail})"
        ),
        (false, false) => format!(
            "RCON NOT delivered action={action} — nothing reached the host agent \
             (state={state}; {detail})"
        ),
    }
}

/// Append one line to the console transcript.
pub(super) fn append_log(console_log: RwSignal<Vec<String>>, line: impl Into<String>) {
    console_log.update(|lines| {
        lines.push(line.into());
    });
}

/// Send one RCON request and report what came back.
///
/// Echoes the command into the transcript before the request goes out, then branches on the
/// **reply body** rather than on the status: the transcript colours a `RCON:` line as success, so
/// routing an undelivered reply down that branch would paint success over a command the host never
/// carried. A second send while one is in flight, or a send with no server selected, is a no-op.
pub(super) fn post_rcon(
    store: crate::v2::core::auth::AuthStore,
    server_id: String,
    body: Value,
    echo: String,
    console_log: RwSignal<Vec<String>>,
    busy: RwSignal<bool>,
    toasts: crate::v2::core::ui::toast::Toasts,
) {
    if busy.get_untracked() || server_id.is_empty() {
        return;
    }
    busy.set(true);
    append_log(console_log, echo);
    #[cfg(target_arch = "wasm32")]
    {
        leptos::task::spawn_local(async move {
            let path = admin_server_rcon_path(&server_id);
            match crate::v2::core::api::client::api_post::<RconAccepted>(store, &path, body).await {
                Ok(resp) => {
                    let msg = rcon_accepted_message(&resp);
                    // Branch on the body, not on the status that got us here: the transcript
                    // colours a `RCON:` line as success.
                    if rcon_reports_success(&resp) {
                        append_log(console_log, format!("RCON: {msg}"));
                        toasts.success(msg);
                    } else {
                        append_log(console_log, format!("RCON error: {msg}"));
                        toasts.error(msg);
                    }
                }
                Err(e) => {
                    let msg =
                        crate::v2::core::api::client::api_error_message(&e, "RCON request failed");
                    append_log(console_log, format!("RCON error: {msg}"));
                    toasts.error(msg);
                }
            }
            busy.set(false);
        });
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (store, body, console_log, toasts);
        busy.set(false);
    }
}

/// Send whatever is typed in the console's command line, then clear it.
///
/// A blank command is refused before anything is sent; the field is cleared before the request so a
/// second press cannot resend the same line.
pub(super) fn fire_custom_command(
    store: crate::v2::core::auth::AuthStore,
    server_id: String,
    command: RwSignal<String>,
    console_log: RwSignal<Vec<String>>,
    busy: RwSignal<bool>,
    toasts: crate::v2::core::ui::toast::Toasts,
) {
    let cmd = command.get_untracked();
    let trimmed = cmd.trim().to_string();
    if trimmed.is_empty() {
        toasts.error("Enter an RCON command");
        return;
    }
    command.set(String::new());
    post_rcon(
        store,
        server_id,
        rcon_body_custom(&trimmed),
        format!("$ {trimmed}"),
        console_log,
        busy,
        toasts,
    );
}
