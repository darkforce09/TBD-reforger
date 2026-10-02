// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts/definitions/wiki-page.schema.json — regenerate with: cargo xtask ci schema-codegen

use super::WikiRevisionSummary;

///GET /api/v1/wiki/{slug}/revisions?page=&per_page= (any signed-in member; 404 when no page has the slug): one page of the page's revisions, newest first.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct WikiRevisionPage {
    pub items: ::std::vec::Vec<WikiRevisionSummary>,
    pub page: ::std::num::NonZeroU64,
    pub per_page: ::std::num::NonZeroU64,
    pub total: u64,
}
