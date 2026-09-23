// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/event-hub.schema.json — regenerate with: cargo xtask ci schema-codegen

///`MissionArmory`
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct MissionArmory {
    pub category: ::std::string::String,
    pub faction: ::std::string::String,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub icon: ::std::option::Option<::std::string::String>,
    pub id: ::uuid::Uuid,
    pub item_name: ::std::string::String,
    pub mission_id: ::uuid::Uuid,
    ///Absent means unlimited.
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub quantity: ::std::option::Option<i64>,
    pub sort_order: i64,
}
