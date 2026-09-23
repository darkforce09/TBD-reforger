// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/fleet-command.schema.json — regenerate with: cargo xtask ci schema-codegen

use super::FleetAction;

///POST /api/v1/servers/:id/commands body. start, stop, restart and list_players take no arguments; broadcast takes {message} (1 to 256 bytes, no control characters); kick takes {arma_id, runtime_session_id, reason?}, where the session must be the server's open runtime session. load_mission and restart_with_mission answer 400: mission deployments issue them.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct FleetCommandRequest {
    pub action: FleetAction,
    #[serde(default, skip_serializing_if = "::serde_json::Map::is_empty")]
    pub arguments: ::serde_json::Map<::std::string::String, ::serde_json::Value>,
}
