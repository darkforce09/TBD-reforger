// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/wiki-page.schema.json — regenerate with: cargo xtask ci schema-codegen

use super::WikiPageSummary;

///GET /api/v1/wiki (any signed-in member): every page's summary, ordered by nav_order, then title, then slug.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct WikiPageList {
    pub data: ::std::vec::Vec<WikiPageSummary>,
}
