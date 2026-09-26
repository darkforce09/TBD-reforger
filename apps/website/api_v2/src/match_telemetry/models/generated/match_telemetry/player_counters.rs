// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/match-telemetry.schema.json — regenerate with: cargo xtask ci schema-codegen

///A complete per-player scoreline. Present on a line it replaces the stored counters (a higher revision may lower them); absent, the line makes no counter claim.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct PlayerCounters {
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub command_win: ::std::option::Option<bool>,
    pub deaths: u64,
    pub is_command: bool,
    pub kills: u64,
    pub longest_kill_m: u64,
    pub team_kills: u64,
    pub vehicles_destroyed: u64,
}
