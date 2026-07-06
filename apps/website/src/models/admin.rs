//! Admin / audit models — Rust port of `internal/models/admin.go`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::RawJson;
use crate::models::serde_helpers::go_time;

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

/// Disciplinary record; the Personnel Roster "Warnings" column counts these.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Warning {
    pub id: Uuid,
    pub discord_id: String,
    pub issued_by: String,
    pub reason: String,
    #[serde(with = "go_time")]
    pub created_at: DateTime<Utc>,
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
    #[serde(with = "go_time")]
    pub created_at: DateTime<Utc>,
}

/// Saved mortar firing solution from the Mortar Calculator.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct FireMission {
    pub id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub event_id: Option<Uuid>,
    pub created_by: String,
    pub weapon_system: String,
    pub fp_grid: String,
    pub target_grid: String,
    pub distance_m: i64,
    /// `numeric(5,1)` — queries must `CAST(azimuth_deg AS double precision)`.
    pub azimuth_deg: f64,
    pub elevation_mils: i64,
    #[serde(with = "go_time")]
    pub created_at: DateTime<Utc>,
}
