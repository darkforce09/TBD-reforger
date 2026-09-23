// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/fleet-command.schema.json — regenerate with: cargo xtask ci schema-codegen

///POST /api/v1/fleet-executor/commands/:commandId/executing body. Answers the receipt; 409 details.code STALE_FENCING_TOKEN when the claim is no longer the caller's, COMMAND_NOT_CLAIMED when the command left the claimed state.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct ExecutionStart {
    pub fencing_token: ::std::num::NonZeroU64,
}
