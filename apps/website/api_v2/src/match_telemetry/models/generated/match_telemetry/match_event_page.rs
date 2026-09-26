// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/match-telemetry.schema.json — regenerate with: cargo xtask ci schema-codegen

use super::MatchEvent;

///GET /api/v1/matches/:matchId/events?after_sequence=&limit= (any signed-in user; limit at most 500, default 100). Events in sequence order; next_after_sequence is null on the last page. The definitions carry the machine-authenticated ingest shapes of /api/v1/ingest/matches, /api/v1/ingest/match-results and /api/v1/ingest/match-events.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct MatchEventPage {
    pub items: ::std::vec::Vec<MatchEvent>,
    pub next_after_sequence: ::std::option::Option<::std::num::NonZeroU64>,
}
