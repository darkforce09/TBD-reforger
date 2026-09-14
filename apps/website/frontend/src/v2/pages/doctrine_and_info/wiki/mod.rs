//! The doctrine wiki: its index, its article surface and the route that binds them.
//!
//! **Role:** declares the route component, the category-grouped manual index, the article
//! surface and the Markdown renderer behind it, and re-exports the page for the router.
//! **Position:** the `/wiki` and `/wiki/:slug` routes, in the doctrine hub.
//! **Signals & state:** none at this level; the page owns the fetch and every signal the panes
//! share.
//! **Invariants:** both panes read the one fetched list — nothing here fetches a second time.
#![allow(dead_code)]

mod category_nav;
mod helpers;
mod markdown;
mod markdown_article;
mod page;

pub use page::WikiPage;
