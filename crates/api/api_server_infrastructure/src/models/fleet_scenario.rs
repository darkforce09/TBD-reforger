//! The scenario the fleet runs for each terrain: a deployment of an artifact on that terrain
//! starts or restarts the server on this scenario header.
//!
//! @contract mission-deployment.schema.json#/definitions/FleetScenario
//! @contract mission-deployment.schema.json#/definitions/FleetScenarioUpdate
//! @contract mission-deployment.schema.json#/definitions/FleetScenarioList

use api_identifiers::ScenarioId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use fleet_wire_contract::rfc3339_timestamps::rfc3339_utc;

/// The fleet scenario registered for one terrain.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct FleetScenario {
    /// The terrain the scenario runs on.
    pub terrain_key: String,
    /// The scenario header the server boots.
    pub scenario_id: ScenarioId,
    /// The name administrators see for the scenario.
    pub display_name: String,
    /// The administrator who registered or last replaced it.
    pub updated_by: String,
    /// When it was registered or last replaced.
    #[serde(with = "rfc3339_utc")]
    pub updated_at: DateTime<Utc>,
}

/// `PUT /fleet/scenarios/{terrainKey}` body.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FleetScenarioUpdate {
    /// The scenario header the server boots.
    pub scenario_id: ScenarioId,
    /// The name administrators see for the scenario.
    pub display_name: String,
}

/// `GET /fleet/scenarios` response.
#[derive(Debug, Serialize)]
pub struct FleetScenarioList {
    /// The scenario of every terrain that has one.
    pub items: Vec<FleetScenario>,
}
