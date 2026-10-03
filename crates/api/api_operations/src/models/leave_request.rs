//! Leave-of-absence requests and the review state an admin moves them through.
//!
//! @contract leave-request.schema.json#/definitions/LeaveRequest

use api_identifiers::{DiscordUserId, LeaveRequestId};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use api_foundation::wire_format::rfc3339_utc_date;
use fleet_wire_contract::rfc3339_timestamps::rfc3339_utc;

/// Leave-request states (Postgres ENUM `leave_status`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "leave_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum LeaveStatus {
    /// Awaiting admin review; wire value `pending`.
    Pending,
    /// Granted by an admin; wire value `approved`.
    Approved,
    /// Refused by an admin; wire value `denied`.
    Denied,
}

/// Backs "Submit Leave of Absence (LOA)". `starts_on`/`ends_on` are Postgres `date`
/// columns rendered as midnight-UTC timestamps on the wire.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct LeaveRequest {
    /// Leave request id (uuid).
    pub id: LeaveRequestId,
    /// Discord id of the member requesting leave.
    pub discord_id: DiscordUserId,
    /// First day of the leave (a midnight-UTC timestamp on the wire).
    #[serde(with = "rfc3339_utc_date")]
    pub starts_on: NaiveDate,
    /// Last day of the leave (a midnight-UTC timestamp on the wire).
    #[serde(with = "rfc3339_utc_date")]
    pub ends_on: NaiveDate,
    /// The member's stated reason; empty (absent on the wire) when the row holds none.
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub reason: String,
    /// Review state of the request.
    pub status: LeaveStatus,
    /// Discord id of the reviewing admin; `None` (absent on the wire) until reviewed.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub reviewed_by: Option<String>,
    /// When the request was filed (RFC 3339 UTC on the wire).
    #[serde(with = "rfc3339_utc")]
    pub created_at: DateTime<Utc>,
}
