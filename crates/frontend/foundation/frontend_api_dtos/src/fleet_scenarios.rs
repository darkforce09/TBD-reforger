//! Fleet scenarios: which scenario header the fleet runs for each terrain.
//!
//! **Role:** one registered terrain → scenario mapping, the registry list, and the body that
//! registers or replaces one.
//! **Position:** deserialised straight from the backend's JSON and handed to the server control
//! screen's scenario registry; re-serialised unchanged by the round-trip tests.
//! **Signals & state:** none — these are plain data.
//! **Invariants:** a deployment of an artifact is refused for a terrain with no registered scenario,
//! and a scenario id is a scenario header resource: sixteen uppercase hex digits in braces, then a
//! `.conf` path.

use super::identifiers::ScenarioId;
use serde::{Deserialize, Serialize};

/// The scenario header the fleet runs for one terrain.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FleetScenario {
    /// The terrain key the compiler writes: lowercase letters, digits and underscores.
    pub terrain_key: String,
    /// The scenario header the server boots.
    pub scenario_id: ScenarioId,
    /// The name administrators see for the scenario.
    pub display_name: String,
    /// The administrator who registered or last replaced it.
    pub updated_by: String,
    /// When it was registered or last replaced.
    pub updated_at: String,
}

/// `GET /fleet/scenarios`: every registered scenario, by terrain.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FleetScenarioList {
    /// The scenario of every terrain that has one.
    pub items: Vec<FleetScenario>,
}

/// `PUT /fleet/scenarios/:terrainKey` body.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FleetScenarioUpdate {
    /// The scenario header the server boots.
    pub scenario_id: ScenarioId,
    /// One to 128 bytes, trimmed.
    pub display_name: String,
}
