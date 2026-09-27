//! The doctrine wiki: its index, the open manual with its history, and the route that binds them.
//!
//! **Role:** declares the route component, the category-grouped manual index, the article pane,
//! the block renderer, the revision history, the save path and their shared state, and re-exports
//! the page for the router.
//! **Position:** the `/wiki` and `/wiki/:slug` routes, in the doctrine hub, over the
//! `/api/v1/wiki` routes of the community content domain.
//! **Signals & state:** none at this level; the page owns the shared signals and each open
//! manual its own.
//! **Invariants:** the index reads the page summaries of `GET /wiki`; the open manual is fetched
//! on its own and rendered from the typed blocks the server parsed, as text nodes, never as HTML.

// The native build mounts nothing (the app's entry point is wasm-only), so the page's views have
// no caller there; the wasm build, which clippy checks, keeps the dead-code lint.
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]

mod api_paths;
mod article;
mod blocks;
mod category_nav;
mod display_text;
mod page;
mod page_state;
mod revisions;
mod saving;

#[cfg(test)]
#[path = "tests/view_attributes.rs"]
mod tests;

pub use page::WikiPage;
