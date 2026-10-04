//! The administrator routes of the fleet scenario registry: list, register or replace, and remove.
//!
//! **Role:** the registry and per-terrain paths, and one call per route.
//! **Position:** called by the server control screen's scenario registry sheet.
//! **Signals & state:** none.
//! **Invariants:** the terrain key is path data and is percent-encoded like any other. A removal
//! answers 204 with no body, so its call reads no answer; an unknown terrain answers 404. Removing a
//! terrain only stops offering it to new deployments.

use super::encode_path_segment as segment;

/// `GET /fleet/scenarios`.
pub fn fleet_scenarios_path() -> String {
    "/fleet/scenarios".to_string()
}

/// `PUT` (register or replace) and `DELETE` (remove) `/fleet/scenarios/:terrainKey`.
pub fn fleet_scenario_path(terrain_key: &str) -> String {
    format!("/fleet/scenarios/{}", segment(terrain_key))
}

#[cfg(target_arch = "wasm32")]
pub use calls::*;

/// The browser-only calls, one per route.
#[cfg(target_arch = "wasm32")]
mod calls {
    use super::*;
    use crate::client::{api_delete, api_get, api_put_keeping_refusal};
    use crate::endpoints::json_body;
    use crate::error::Result;
    use crate::token_provider::TokenProvider;
    use frontend_api_dtos::{FleetScenario, FleetScenarioList, FleetScenarioUpdate};

    /// Every registered scenario, by terrain.
    pub async fn load_fleet_scenarios(store: impl TokenProvider) -> Result<FleetScenarioList> {
        api_get(store, &fleet_scenarios_path()).await
    }

    /// Register the terrain's scenario, or replace the one registered.
    pub async fn put_fleet_scenario(
        store: impl TokenProvider,
        terrain_key: &str,
        update: &FleetScenarioUpdate,
    ) -> Result<FleetScenario> {
        let path = fleet_scenario_path(terrain_key);
        api_put_keeping_refusal(store, &path, json_body(update)?).await
    }

    /// Stop offering the terrain to new deployments.
    pub async fn delete_fleet_scenario(store: impl TokenProvider, terrain_key: &str) -> Result<()> {
        api_delete(store, &fleet_scenario_path(terrain_key)).await
    }
}
