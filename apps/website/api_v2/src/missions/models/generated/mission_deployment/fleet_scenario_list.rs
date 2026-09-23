// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/mission-deployment.schema.json — regenerate with: cargo xtask ci schema-codegen

use super::FleetScenario;

///GET /api/v1/fleet/scenarios (administrator).
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct FleetScenarioList {
    pub items: ::std::vec::Vec<FleetScenario>,
}
