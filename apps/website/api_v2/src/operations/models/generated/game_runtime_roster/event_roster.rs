// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/game-runtime-roster.schema.json — regenerate with: cargo xtask ci schema-codegen

use super::{RosterAssignment, RosterSlot};

///GET /api/v1/game-runtime/events/:id/roster (mod_runtime machine credential of the event's bound server). Game wire: camelCase keys. assignments lists each seated player's compiled slot; slots lists every compiled slot with the ids a deployment request names. A player registered on two missions of one event is seated in the earliest by start time.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct EventRoster {
    pub assignments: ::std::vec::Vec<RosterAssignment>,
    #[serde(rename = "eventId")]
    pub event_id: ::uuid::Uuid,
    ///The catalog mission when the event holds exactly one mission, otherwise empty.
    #[serde(rename = "missionId")]
    pub mission_id: ::std::string::String,
    pub slots: ::std::vec::Vec<RosterSlot>,
    pub version: i64,
}
