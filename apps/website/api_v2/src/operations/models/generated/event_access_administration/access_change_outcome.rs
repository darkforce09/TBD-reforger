// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/event-access-administration.schema.json — regenerate with: cargo xtask ci schema-codegen

use super::EventAccessAdministration;

///The access view after a change and the reservations the change released or promoted.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct AccessChangeOutcome {
    pub access: EventAccessAdministration,
    pub promoted_registrations: ::std::vec::Vec<::uuid::Uuid>,
    pub released_registrations: ::std::vec::Vec<::uuid::Uuid>,
}
