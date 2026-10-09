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

mod api_paths;
#[cfg(target_arch = "wasm32")]
mod article;
mod blocks;
mod category_nav;
#[cfg(target_arch = "wasm32")]
mod display_text;
pub mod page;
mod page_state;
#[cfg(target_arch = "wasm32")]
mod revisions;
mod saving;

#[cfg(target_arch = "wasm32")]
pub use page::WikiPage;
