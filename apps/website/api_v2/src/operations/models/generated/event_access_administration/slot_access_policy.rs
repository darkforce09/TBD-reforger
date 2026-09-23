// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/event-access-administration.schema.json — regenerate with: cargo xtask ci schema-codegen

use super::EventAccessPolicy;

///`SlotAccessPolicy`
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct SlotAccessPolicy {
    pub event_mission_id: ::uuid::Uuid,
    pub faction: ::std::string::String,
    pub policy: EventAccessPolicy,
    pub slot_id: ::uuid::Uuid,
    pub slot_index: i64,
    pub squad: ::std::string::String,
}
