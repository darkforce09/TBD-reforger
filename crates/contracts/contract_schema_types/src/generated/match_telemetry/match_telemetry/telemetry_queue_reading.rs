// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts/definitions/match-telemetry.schema.json — regenerate with: cargo xtask ci schema-codegen

///The game runtime's outbound telemetry queue as a heartbeat reports it: entries waiting, capacity, entries dropped since the queue was created, and the age of the oldest waiting entry.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct TelemetryQueueReading {
    pub backlog: u64,
    pub capacity: u64,
    pub dropped_total: u64,
    pub oldest_age_seconds: u64,
}
