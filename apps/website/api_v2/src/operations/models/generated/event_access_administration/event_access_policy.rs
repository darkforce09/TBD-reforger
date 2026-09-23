// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/event-access-administration.schema.json — regenerate with: cargo xtask ci schema-codegen

use super::EventAccessGrant;

///Grants are alternatives (any one admits); the conditions of one grant must all hold. An empty grant list admits nobody. A missing squad or slot policy inherits: slot, then squad, then event.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct EventAccessPolicy {
    pub grants: ::std::vec::Vec<EventAccessGrant>,
}
