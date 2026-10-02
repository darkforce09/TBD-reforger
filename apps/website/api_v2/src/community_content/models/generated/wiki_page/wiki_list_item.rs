// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts/definitions/wiki-page.schema.json — regenerate with: cargo xtask ci schema-codegen

use super::WikiBlock;

///One list item. checked is present on task-list items only, true when the box is ticked.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct WikiListItem {
    pub blocks: ::std::vec::Vec<WikiBlock>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub checked: ::std::option::Option<bool>,
}
