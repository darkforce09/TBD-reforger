//! Faction library model (T-152) — operator-authored reusable factions consumed by the
//! Mission Creator palette (side → faction → roles/vehicles).
//!
//! @contract faction-library.schema.json#/

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::RawJson;
use crate::models::serde_helpers::go_time;

/// One reusable faction. `doc` is the full faction-library document (validated against
/// the generated contract on every write); `side`/`name` are projections of the same
/// fields for listing and the (owner, name) uniqueness rule.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct UserFaction {
    pub id: Uuid,
    pub owner_id: String,
    pub side: String,
    pub name: String,
    pub doc: RawJson,
    #[serde(with = "go_time")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "go_time")]
    pub updated_at: DateTime<Utc>,
}
