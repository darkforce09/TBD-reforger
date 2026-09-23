//! A machine credential in words, and the checks an issue or a revocation passes before it is sent.
//!
//! **Role:** names the program a credential authenticates, states where a credential stands —
//! live and when it was last used, or revoked by whom, when and why — and validates the label an
//! issue names and the reason a revocation gives.
//! **Position:** read by the credential sheet's list and forms.
//! **Signals & state:** none; pure over its arguments.
//! **Invariants:** the checks mirror the backend's bounds — a label of 1 to 128 bytes and a reason
//! of 1 to 512 bytes, both trimmed — so a request the backend would refuse is never sent. A
//! revocation without a reason is refused here too: the reason is what the audit trail keeps. A
//! program kind this build does not know is shown as the backend spells it.

use crate::v2::core::api::dto::{ExecutorKind, MachineCredential};
use crate::v2::core::utils::utc_timestamp::{utc_label, UtcTimestamp};

/// The two program kinds, as the issue form's select offers them.
pub(super) const EXECUTOR_KINDS: [(ExecutorKind, &str, &str); 2] = [
    (
        ExecutorKind::ModRuntime,
        "mod_runtime",
        "Game runtime — sessions, heartbeats, roster reads, broadcasts, kicks and deployments",
    ),
    (
        ExecutorKind::HostAgent,
        "host_agent",
        "Host agent — process control, the RCON player list and cross-terrain restarts",
    ),
];

/// The program kind a select value names.
pub(super) fn executor_kind(value: &str) -> Option<ExecutorKind> {
    EXECUTOR_KINDS
        .iter()
        .find(|(_, wire, _)| *wire == value)
        .map(|(kind, _, _)| *kind)
}

/// The program a stored credential authenticates — or a fleet command runs on — in words.
pub(in super::super) fn executor_label(kind: &str) -> String {
    match kind {
        "host_agent" => "Host agent".to_string(),
        "mod_runtime" => "Game runtime".to_string(),
        other => other.replace('_', " "),
    }
}

/// An account id as shown: "you" for the viewer's own.
pub(super) fn account_label(account: &str, me: Option<&str>) -> String {
    if me == Some(account) {
        "you".to_string()
    } else {
        account.to_string()
    }
}

/// Who issued the credential, and when.
pub(super) fn issued_line(credential: &MachineCredential, me: Option<&str>) -> String {
    format!(
        "Issued by {}, {}",
        account_label(&credential.created_by, me),
        utc_label(&credential.created_at)
    )
}

/// Where the credential stands: live and its last use, or its revocation.
pub(super) fn standing_line(credential: &MachineCredential, me: Option<&str>) -> String {
    match &credential.revoked_at {
        Some(revoked_at) => {
            let by = credential
                .revoked_by
                .as_deref()
                .map(|account| format!(" by {}", account_label(account, me)))
                .unwrap_or_default();
            let reason = credential
                .revoke_reason
                .as_deref()
                .map(|reason| format!(": {reason}"))
                .unwrap_or_default();
            format!("Revoked{by}, {}{reason}", utc_label(revoked_at))
        }
        None => match &credential.last_used_at {
            Some(used) => format!("Live · last used {}", utc_label(used)),
            None => "Live · never used".to_string(),
        },
    }
}

/// Live credentials first and revoked ones below them, each half newest first. Issue times are
/// compared as instants: the wire trims fractions, so their text does not sort.
pub(super) fn listing_order(mut credentials: Vec<MachineCredential>) -> Vec<MachineCredential> {
    credentials.sort_by(|a, b| {
        a.revoked_at
            .is_some()
            .cmp(&b.revoked_at.is_some())
            .then_with(|| {
                UtcTimestamp::parse(&b.created_at).cmp(&UtcTimestamp::parse(&a.created_at))
            })
    });
    credentials
}

/// The label an issue names, trimmed, or what is wrong with it.
pub(super) fn validated_label(text: &str) -> Result<String, String> {
    let label = text.trim();
    match label.len() {
        0 => Err("Name the credential, for example the host it is installed on".to_string()),
        1..=128 => Ok(label.to_string()),
        _ => Err("A credential label is too long: at most 128 bytes".to_string()),
    }
}

/// The reason a revocation gives, trimmed, or what is wrong with it.
pub(super) fn validated_reason(text: &str) -> Result<String, String> {
    let reason = text.trim();
    match reason.len() {
        0 => Err("A revocation needs a reason; it is kept in the audit trail".to_string()),
        1..=512 => Ok(reason.to_string()),
        _ => Err("A revocation reason is too long: at most 512 bytes".to_string()),
    }
}
