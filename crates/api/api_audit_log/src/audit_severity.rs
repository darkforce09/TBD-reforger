//! The severity of an audit line.
//!
//! **Role:** the one severity vocabulary every audit line carries, whichever domain writes it.
//! **Position:** bound by [`crate::audit_writer`] and
//! [`crate::required_audit`] into `audit_logs.severity`; read back into the
//! administration domain's audit line.
//! **Signals & state:** none; plain data.
//! **Invariants:** [`AuditSeverity`] and the Postgres enum `audit_severity` hold the same three
//! values, spelled snake_case on the wire and in SQL.
//! @contract audit-log.schema.json#/definitions/AuditLogEntry

use serde::{Deserialize, Serialize};

/// Audit severities (Postgres ENUM `audit_severity`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "audit_severity", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum AuditSeverity {
    /// A routine action.
    Info,
    /// An action a moderator should notice, such as a warning or a failed sync.
    Warn,
    /// An action or failure that needs attention, such as a ban or a failed webhook push.
    Crit,
}

impl AuditSeverity {
    /// The Postgres/JSON wire string.
    pub fn as_str(self) -> &'static str {
        match self {
            AuditSeverity::Info => "info",
            AuditSeverity::Warn => "warn",
            AuditSeverity::Crit => "crit",
        }
    }
}
