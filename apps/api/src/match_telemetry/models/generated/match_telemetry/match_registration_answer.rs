// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts/definitions/match-telemetry.schema.json — regenerate with: cargo xtask ci schema-codegen

///POST /api/v1/ingest/matches response: 201 with registered true for a new registration, 200 with registered false for a repeat of the same body.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct MatchRegistrationAnswer {
    pub match_id: ::uuid::Uuid,
    pub registered: bool,
}
