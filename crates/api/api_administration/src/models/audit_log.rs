//! The audit log line.
//!
//! **Role:** the one shape of an audit line, read by the audit list, the CSV export and the live
//! stream alike.
//! **Position:** read from `audit_logs` by the audit log handlers and
//! [`crate::services::audit_delivery`]; its severity is `api_audit_log`'s
//! [`AuditSeverity`], which every domain that writes an audit line passes.
//! **Signals & state:** none; plain data.
//! **Invariants:** `id` is the allocation order, never the delivery order; empty `actor_name`,
//! `target_type` and `target_id` and a missing `actor_id` or `metadata` are omitted on the wire.
//! @contract audit-log.schema.json#/definitions/AuditLogEntry

use api_identifiers::{AuditLogEntryId, AuditTargetId, DiscordUserId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use api_audit_log::AuditSeverity;
use api_foundation::wire_format::RawJson;
use fleet_wire_contract::rfc3339_timestamps::rfc3339_utc;

/// One audit line, the same shape in the history list and on the live stream.
/// @contract audit-log.schema.json#/definitions/AuditLogEntry
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AuditLog {
    /// The audit row id.
    pub id: AuditLogEntryId,
    /// How serious the recorded action is.
    pub severity: AuditSeverity,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    /// The member who acted; absent for a system action.
    pub actor_id: Option<DiscordUserId>,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    /// The acting member's display name; empty for a system action.
    pub actor_name: String,
    /// The machine-readable action name.
    pub action: String,
    /// The human-readable description of the action.
    pub message: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    /// The kind of resource acted on; empty when none.
    pub target_type: String,
    #[serde(skip_serializing_if = "AuditTargetId::is_empty", default)]
    /// The id of the resource acted on; empty when none.
    pub target_id: AuditTargetId,
    /// `jsonb` (nullable) — passthrough (hazard #8).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub metadata: Option<RawJson>,
    /// When the action was recorded, in RFC 3339 UTC on the wire.
    #[serde(with = "rfc3339_utc")]
    pub created_at: DateTime<Utc>,
}
