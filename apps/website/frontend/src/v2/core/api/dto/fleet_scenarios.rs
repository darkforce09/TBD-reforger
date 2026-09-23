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

use serde::{Deserialize, Serialize};

/// The scenario header the fleet runs for one terrain.
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FleetScenario {
    /// The terrain key the compiler writes: lowercase letters, digits and underscores.
    pub terrain_key: String,
    pub scenario_id: String,
    pub display_name: String,
    pub updated_by: String,
    pub updated_at: String,
}

/// `GET /fleet/scenarios`: every registered scenario, by terrain.
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FleetScenarioList {
    pub items: Vec<FleetScenario>,
}

/// `PUT /fleet/scenarios/:terrainKey` body.
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FleetScenarioUpdate {
    pub scenario_id: String,
    /// One to 128 bytes, trimmed.
    pub display_name: String,
}
