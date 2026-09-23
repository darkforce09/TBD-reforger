// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/event-access-administration.schema.json — regenerate with: cargo xtask ci schema-codegen

use super::EventAccessPolicy;

///`SquadAccessPolicy`
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct SquadAccessPolicy {
    pub event_mission_id: ::uuid::Uuid,
    pub faction: ::std::string::String,
    pub policy: EventAccessPolicy,
    pub squad: ::std::string::String,
}
