// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/event-access-administration.schema.json — regenerate with: cargo xtask ci schema-codegen

///`RosterEntryView`
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct RosterEntryView {
    pub added_at: ::chrono::DateTime<::chrono::offset::Utc>,
    pub added_by: ::std::option::Option<::std::string::String>,
    pub discord_id: ::std::string::String,
    pub system_origin: ::std::option::Option<::std::string::String>,
    pub username: ::std::string::String,
}
