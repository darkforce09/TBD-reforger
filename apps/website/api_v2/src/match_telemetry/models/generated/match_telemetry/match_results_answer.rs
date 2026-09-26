// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/match-telemetry.schema.json — regenerate with: cargo xtask ci schema-codegen

///POST /api/v1/ingest/match-results response. linked + unlinked == players counts the submitted lines; applied is false for an inert retry.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct MatchResultsAnswer {
    pub applied: bool,
    pub linked: u64,
    pub match_id: ::uuid::Uuid,
    pub players: u64,
    pub revision: ::std::num::NonZeroU64,
    pub unlinked: u64,
    pub unlinked_arma_ids: ::std::vec::Vec<::std::string::String>,
}
