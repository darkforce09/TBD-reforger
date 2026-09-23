// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/fleet-command.schema.json — regenerate with: cargo xtask ci schema-codegen

use super::FleetAction;

///A claimed command. The executor reports executing (ExecutionStart) before acting and the outcome (ExecutionResult) after; both carry fencing_token. A claim not reported executing before lease_expires_at returns to the queue.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct ClaimedFleetCommand {
    pub action: FleetAction,
    pub arguments: ::serde_json::Map<::std::string::String, ::serde_json::Value>,
    pub command_id: ::uuid::Uuid,
    pub fencing_token: ::std::num::NonZeroU64,
    pub lease_expires_at: ::chrono::DateTime<::chrono::offset::Utc>,
    pub server_id: ::uuid::Uuid,
}
