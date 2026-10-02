// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts/definitions/match-telemetry.schema.json — regenerate with: cargo xtask ci schema-codegen

///POST /api/v1/ingest/match-events response.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct MatchEventBatchAnswer {
    pub accepted: u64,
    pub duplicates: u64,
    pub event_count: u64,
    pub last_sequence: ::std::option::Option<::std::num::NonZeroU64>,
    pub match_id: ::uuid::Uuid,
}
