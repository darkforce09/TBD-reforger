// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/game-runtime-session.schema.json — regenerate with: cargo xtask ci schema-codegen

///POST /api/v1/game-runtime/sessions/:sessionId/heartbeats body. generation must equal the session's generation and sequence must exceed every sequence the session already admitted; gaps are allowed. At least one reading is required. server_id is refused: the credential identifies the server. current_match_id: absent keeps, empty string clears, a uuid sets.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct RuntimeHeartbeat {
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub current_match_id: ::std::option::Option<::std::string::String>,
    pub generation: ::std::num::NonZeroU64,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub ingame_time: ::std::option::Option<::std::string::String>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub ingame_weather: ::std::option::Option<::std::string::String>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub is_online: ::std::option::Option<bool>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub max_players: ::std::option::Option<u64>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub player_count: ::std::option::Option<u64>,
    pub sequence: ::std::num::NonZeroU64,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub server_fps: ::std::option::Option<f64>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub uptime_seconds: ::std::option::Option<u64>,
}
