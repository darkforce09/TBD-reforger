//! The acknowledgement-dropping relay's control commands on the staging host.
//!
//! **Role:** builds `acknowledgement-dropping-relay control` for the relay instance: arm one drop,
//! disarm, and the status read; and judges whether a status answer is disarmed.
//!
//! **Position:** used by the fleet procedure's lost-acknowledgement waves, its recovery action
//! list and `staging status`; the relay itself is
//! `tools/staging/acknowledgement_dropping_relay/src/`, installed by
//! `cargo xtask deploy staging` as `acknowledgement-dropping-relay@<instance>`.
//!
//! **Signals & state:** none; pure builders.
//!
//! **Invariants:** the control socket is the unit's runtime folder
//! (`$XDG_RUNTIME_DIR/acknowledgement-dropping-relay-<instance>/control.sock`, falling back to
//! `/run/user/<uid>` when the ssh session sets no `XDG_RUNTIME_DIR`); status only reads.

use crate::error::Result;

use crate::remote_observers::remote_command::{CommandPurpose, RemoteCommand};

/// The relay binary the deploy installs.
pub(crate) const RELAY_BINARY: &str = "$HOME/.local/bin/acknowledgement-dropping-relay";

/// Which executor answer the relay withholds once.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DropTarget {
    /// The next 200 answer to a claim.
    ClaimResponse,
    /// The next 200 answer to a result.
    ResultResponse,
}

/// The control socket of `instance`'s relay, as a double-quoted shell word.
fn control_socket(instance: u16) -> String {
    format!(
        "\"${{XDG_RUNTIME_DIR:-/run/user/$(id -u)}}/acknowledgement-dropping-relay-{instance}/control.sock\""
    )
}

fn control(instance: u16, purpose: CommandPurpose, operation: &str) -> RemoteCommand {
    RemoteCommand {
        observer: "relay control",
        purpose,
        command_line: format!(
            "\"{RELAY_BINARY}\" control --control-socket {} {operation}",
            control_socket(instance)
        ),
        stdin: None,
    }
}

/// Arms the relay of `instance` to withhold the next answer of `target`.
pub(crate) fn arm(instance: u16, target: DropTarget) -> RemoteCommand {
    let mode = match target {
        DropTarget::ClaimResponse => "drop-next-claim-response",
        DropTarget::ResultResponse => "drop-next-result-response",
    };
    control(instance, CommandPurpose::Change, &format!("arm {mode}"))
}

/// Disarms the relay of `instance`.
pub(crate) fn disarm(instance: u16) -> RemoteCommand {
    control(instance, CommandPurpose::Change, "disarm")
}

/// The relay's JSON status: armed mode, the withheld command id and fencing token.
pub(crate) fn status(instance: u16) -> RemoteCommand {
    control(instance, CommandPurpose::Read, "status")
}

/// Whether a status answer shows the relay disarmed: its `arming` field reads `"disarmed"`;
/// an armed target reads `false`, and an answer without a string `arming` is an error.
pub(crate) fn is_disarmed(status_json: &str) -> Result<bool> {
    let value: serde_json::Value = serde_json::from_str(status_json.trim())?;
    match value.get("arming").and_then(serde_json::Value::as_str) {
        Some(arming) => Ok(arming == "disarmed"),
        None => crate::error::bail!(
            "the relay status names no `arming` state: {}",
            status_json.trim()
        ),
    }
}
