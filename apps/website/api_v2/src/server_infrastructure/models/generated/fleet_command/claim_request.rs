// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/fleet-command.schema.json — regenerate with: cargo xtask ci schema-codegen

///POST /api/v1/fleet-executor/commands/claim body (machine credential). A game runtime names its open runtime session; a host agent sends {}. Answers ClaimedFleetCommand, or 204 when nothing is claimable now.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct ClaimRequest {
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub runtime_session_id: ::std::option::Option<::uuid::Uuid>,
}
impl ::std::default::Default for ClaimRequest {
    fn default() -> Self {
        Self {
            runtime_session_id: Default::default(),
        }
    }
}
