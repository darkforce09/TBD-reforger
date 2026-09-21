//! Leave-of-absence requests and the review state an admin moves them through.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::core::wire_format::{rfc3339_utc, rfc3339_utc_date};

/// Leave-request states (Postgres ENUM `leave_status`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "leave_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum LeaveStatus {
    Pending,
    Approved,
    Denied,
}

/// Backs "Submit Leave of Absence (LOA)". `starts_on`/`ends_on` are Postgres `date`
/// columns rendered as midnight-UTC timestamps on the wire.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct LeaveRequest {
    pub id: Uuid,
    pub discord_id: String,
    #[serde(with = "rfc3339_utc_date")]
    pub starts_on: NaiveDate,
    #[serde(with = "rfc3339_utc_date")]
    pub ends_on: NaiveDate,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub reason: String,
    pub status: LeaveStatus,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub reviewed_by: Option<String>,
    #[serde(with = "rfc3339_utc")]
    pub created_at: DateTime<Utc>,
}
