// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/game-runtime-session.schema.json — regenerate with: cargo xtask ci schema-codegen

///POST /api/v1/game-runtime/sessions (mod_runtime machine credential, Authorization: Bearer tbdm_...). Starting a session ends the server's open one as superseded and takes the next generation. Heartbeats are due every heartbeat_interval_seconds; a session silent for expires_after_seconds ends as expired.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct StartedRuntimeSession {
    pub expires_after_seconds: ::std::num::NonZeroU64,
    pub generation: ::std::num::NonZeroU64,
    pub heartbeat_interval_seconds: ::std::num::NonZeroU64,
    pub runtime_session_id: ::uuid::Uuid,
    pub server_id: ::uuid::Uuid,
    pub started_at: ::chrono::DateTime<::chrono::offset::Utc>,
}
