// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts/definitions/match-telemetry.schema.json — regenerate with: cargo xtask ci schema-codegen

use super::{MatchReport, PlayerLine, RemovedLine};

///POST /api/v1/ingest/match-results body (mod_runtime machine credential). The report digest is the SHA-256 of the canonical JSON of this body without revision. An older revision answers 409 STALE_REVISION, the same revision with another digest 409 REVISION_CONFLICT, the same revision and digest is inert.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct MatchResultsRevision {
    #[serde(rename = "match")]
    pub match_: MatchReport,
    pub players: ::std::vec::Vec<PlayerLine>,
    #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
    pub removed_lines: ::std::vec::Vec<RemovedLine>,
    pub revision: ::std::num::NonZeroU64,
}
