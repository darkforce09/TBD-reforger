//! The block renderer: a manual's typed blocks, as the server parsed them, turned into views.
//!
//! **Role:** declares the render tree, the block, table and inline mappers, the callout styles
//! and the element views, and exposes [`render_blocks`], the one call the wiki makes.
//! **Position:** fed the `blocks` of an article (`GET /wiki/{slug}`) or of a revision
//! (`GET /wiki/{slug}/revisions/{revision}`) by the reading pane and the revision view.
//! **Signals & state:** none; the views are static once built.
//! **Invariants:** rendering is two pure steps — blocks to a render tree, then the tree to views
//! — so every decision is testable without a browser. The tree holds content only as text nodes
//! and attribute values, every `href` and `src` is re-checked against the content URL policy, and
//! nothing is ever written as inner HTML.

mod block_mapping;
mod callout_style;
mod element_views;
mod inline_mapping;
mod render_tree;
mod table_mapping;

use crate::v2::core::api::dto::wiki::WikiBlock;
use leptos::prelude::*;

/// The views of `blocks`, in order.
pub(super) fn render_blocks(blocks: &[WikiBlock]) -> impl IntoView {
    element_views::node_views(block_mapping::block_nodes(blocks))
}
