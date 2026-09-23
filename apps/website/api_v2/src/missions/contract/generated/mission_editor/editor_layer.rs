// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/mission-editor-payload.schema.json — regenerate with: cargo xtask ci schema-codegen

///One authored editor layer row.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct EditorLayer {
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub collapsed: ::std::option::Option<bool>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub color: ::std::option::Option<::std::string::String>,
    #[serde(
        rename = "entityIds",
        default,
        skip_serializing_if = "::std::vec::Vec::is_empty"
    )]
    pub entity_ids: ::std::vec::Vec<::std::string::String>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub hidden: ::std::option::Option<bool>,
    pub id: ::std::string::String,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub locked: ::std::option::Option<bool>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub name: ::std::option::Option<::std::string::String>,
    #[serde(
        rename = "parentId",
        default,
        skip_serializing_if = "::std::option::Option::is_none"
    )]
    pub parent_id: ::std::option::Option<::std::string::String>,
}
