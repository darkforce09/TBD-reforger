// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/event-orbat.schema.json — regenerate with: cargo xtask ci schema-codegen

use super::OrbatSquad;

///GET /api/v1/event-missions/:emid/orbat: the ORBAT grouped by squad, projected for the viewer. A partial viewer receives only admitted seats and no occupant outside them; a mission of a hidden event, or without admitted seats for a partial viewer, answers 404 like a missing one.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct EventMissionOrbat {
    pub data: ::std::vec::Vec<OrbatSquad>,
}
