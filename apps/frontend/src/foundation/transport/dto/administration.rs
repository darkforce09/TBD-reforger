//! Administration payloads: the personnel roster, the audit log lines and the control events of the
//! live audit stream.
//!
//! **Role:** the page of members `GET /admin/users` answers, the audit line the history list and
//! the live stream share, and the `ready` and `reset` events that bracket the stream's lines.
//! **Position:** deserialised straight from the backend's JSON (the roster and history bodies, and
//! the data of each audit stream event) and handed to the personnel and audit log pages;
//! re-serialised unchanged by the round-trip tests.
//! **Signals & state:** none — these are plain data.
//! **Invariants:** a roster page's `total` counts every matching member whatever the page, and
//! `arma_id` is always on the wire, `null` until the member links an Arma identity. An audit line's
//! empty `actor_name`, `target_type` and `target_id` and its missing `actor_id` or `metadata` are
//! absent on the wire, never empty strings or `null`. Every stream cursor is a publication
//! sequence, never an audit line `id`, and every sequence at or below `retained_after` may be
//! missing.
//! @contract personnel-roster.schema.json#/definitions/PersonnelPage
//! @contract audit-log.schema.json#/definitions/AuditLogEntry

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::foundation::transport::dto::role::Role;

/// One page of the personnel roster, in `lower(username)`, then `discord_id` order.
/// @contract personnel-roster.schema.json#/definitions/PersonnelPage
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PersonnelPage {
    pub items: Vec<AdminUserRow>,
    /// The 1-based page served.
    pub page: i64,
    /// The page size served: 1 to 100.
    pub per_page: i64,
    /// Every member the search matches; a page past the end has no items and this real total.
    pub total: i64,
}

/// One member as the roster lists them.
/// @contract personnel-roster.schema.json#/definitions/PersonnelRow
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AdminUserRow {
    pub discord_id: String,
    /// Empty when unset.
    pub username: String,
    /// Empty when unset.
    pub discord_handle: String,
    /// `null` until the member links an Arma identity; always present on the wire.
    #[serde(default)]
    pub arma_id: Option<String>,
    /// Empty when unset.
    pub arma_character: String,
    pub role: Role,
    pub is_banned: bool,
    /// The member's disciplinary warnings.
    pub warnings: i64,
    /// The member's recorded deployment tally.
    pub total_deployments: i64,
}

/// How serious an audit line is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditSeverity {
    Info,
    Warn,
    Crit,
}

/// One audit line, the same shape in the history list and on the live stream.
/// @contract audit-log.schema.json#/definitions/AuditLogEntry
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AuditLogEntry {
    /// The line's allocation order, which is not the stream's delivery order.
    pub id: i64,
    pub severity: AuditSeverity,
    /// The Discord id of the member who acted; absent on system lines.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor_id: Option<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub actor_name: String,
    pub action: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub target_type: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub target_id: String,
    /// The structured context the writer recorded, carried as sent: its keys belong to the
    /// action that wrote the line.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
    pub created_at: String,
}

/// The data of `event: ready`, the first event of every audit stream; its SSE id is
/// `resume_after`.
/// @contract audit-log.schema.json#/definitions/AuditStreamReady
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditStreamReady {
    /// The start cursor: the sent `Last-Event-ID`, or the tail when none was sent.
    pub resume_after: i64,
    /// The retained floor when the stream opened.
    pub retained_after: i64,
}

/// Why an audit stream cannot replay the history its cursor asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditStreamResetReason {
    /// The cursor is beyond the newest publication sequence.
    CursorAhead,
    /// The cursor is below the retained floor, so lines after it may be gone.
    HistoryUnavailable,
}

/// The data of `event: reset`: the stream skips to the tail, and the page reloads its history
/// from the list route; its SSE id is `resume_after`.
/// @contract audit-log.schema.json#/definitions/AuditStreamReset
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditStreamReset {
    pub reason: AuditStreamResetReason,
    /// The tail the stream continues after.
    pub resume_after: i64,
    /// The retained floor the reset was decided against.
    pub retained_after: i64,
}

#[cfg(test)]
#[path = "tests/administration.rs"]
mod tests;
