//! The disciplinary warning record.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::core::wire_format::go_time;

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
