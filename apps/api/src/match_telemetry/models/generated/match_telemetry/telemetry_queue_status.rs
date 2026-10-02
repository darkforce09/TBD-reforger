// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts/definitions/match-telemetry.schema.json — regenerate with: cargo xtask ci schema-codegen

///ServerStatus.telemetry_queue: the last queue reading a server reported and when. Absent when the server never reported one.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct TelemetryQueueStatus {
    pub backlog: u64,
    pub capacity: u64,
    pub dropped_total: u64,
    pub oldest_age_seconds: u64,
    pub reported_at: ::chrono::DateTime<::chrono::offset::Utc>,
}
