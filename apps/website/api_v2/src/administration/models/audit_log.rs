//! The admin papertrail row and its severity ENUM.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::core::wire_format::RawJson;
use crate::core::wire_format::rfc3339_utc;

/// Audit severities (Postgres ENUM `audit_severity`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "audit_severity", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum AuditSeverity {
    Info,
    Warn,
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

/// Admin papertrail line. `id` is a bigint sequence.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AuditLog {
    pub id: i64,
    pub severity: AuditSeverity,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub actor_id: Option<String>,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub actor_name: String,
    pub action: String,
    pub message: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub target_type: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub target_id: String,
    /// `jsonb` (nullable) — passthrough (hazard #8).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub metadata: Option<RawJson>,
    #[serde(with = "rfc3339_utc")]
    pub created_at: DateTime<Utc>,
}
