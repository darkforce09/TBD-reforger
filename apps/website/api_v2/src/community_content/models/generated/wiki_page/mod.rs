// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/wiki-page.schema.json — regenerate with: cargo xtask ci schema-codegen

//! Types generated from `contracts_v2/definitions/wiki-page.schema.json`, one module per schema definition.

pub mod error;
mod wiki_article;
pub use wiki_article::*;
mod wiki_block;
pub use wiki_block::*;
mod wiki_callout_kind;
pub use wiki_callout_kind::*;
mod wiki_inline;
pub use wiki_inline::*;
mod wiki_list_item;
pub use wiki_list_item::*;
mod wiki_markup_finding;
pub use wiki_markup_finding::*;
mod wiki_page_contract;
pub use wiki_page_contract::*;
mod wiki_page_list;
pub use wiki_page_list::*;
mod wiki_page_summary;
pub use wiki_page_summary::*;
mod wiki_revision;
pub use wiki_revision::*;
mod wiki_revision_page;
pub use wiki_revision_page::*;
mod wiki_revision_summary;
pub use wiki_revision_summary::*;
mod wiki_save_refusal;
pub use wiki_save_refusal::*;
mod wiki_save_request;
pub use wiki_save_request::*;
mod wiki_table_alignment;
pub use wiki_table_alignment::*;
