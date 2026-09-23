//! The scenario the fleet runs for each terrain: a deployment of an artifact on that terrain
//! starts or restarts the server on this scenario header.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::core::wire_format::rfc3339_utc;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct FleetScenario {
    pub terrain_key: String,
    pub scenario_id: String,
    pub display_name: String,
    pub updated_by: String,
    #[serde(with = "rfc3339_utc")]
    pub updated_at: DateTime<Utc>,
}

/// `PUT /fleet/scenarios/{terrainKey}` body.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FleetScenarioUpdate {
    pub scenario_id: String,
    pub display_name: String,
}

/// `GET /fleet/scenarios` response.
#[derive(Debug, Serialize)]
pub struct FleetScenarioList {
    pub items: Vec<FleetScenario>,
}
