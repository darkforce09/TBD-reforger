// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/event-access-administration.schema.json — regenerate with: cargo xtask ci schema-codegen

use super::EventAccessCondition;

///`EventAccessGrant`
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct EventAccessGrant {
    pub conditions: ::std::vec::Vec<EventAccessCondition>,
}
