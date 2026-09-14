//! Shared platform foundations: transport, session, design-system primitives, helpers.
//!
//! **Role:** groups the zero-business-logic building blocks every page and app depends on —
//! the HTTP/SSE layer and its wire types, the authentication store, the Aegis UI primitives,
//! and small pure utilities.
//! **Position:** the bottom of the dependency graph. Everything above may import from here.
//! **Signals & state:** none directly; the child modules own their own contexts and stores.
//! **Invariants:** no module under `core` imports from `pages` or from an app. All children are
//! ungated so the native test build compiles them; browser-only bodies carry their own
//! `#[cfg(target_arch = "wasm32")]` inside the files.

pub mod api;
pub mod auth;
pub mod ui;
pub mod utils;

#[cfg(test)]
pub mod test_support;
