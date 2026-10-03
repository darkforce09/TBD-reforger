//! Shared platform foundations: transport, session, route table, design-system primitives, helpers.
//!
//! **Role:** groups the zero-business-logic building blocks every feature, page, workspace and the
//! shell depend on — the HTTP/SSE transport and its wire types, the route table and the navigation
//! menu, the authentication store, the Aegis interface primitives, the offline pack, the shared
//! map mount, and small pure utilities.
//! **Position:** the bottom layer of the frontend (foundation < features < pages, workspaces <
//! shell). Every layer above may import from here.
//! **Signals & state:** none directly; the child modules own their own contexts and stores.
//! **Invariants:** no module under `foundation` imports from `features`, `pages`, `workspaces` or
//! `shell`. All children are ungated so the native test build compiles them; browser-only bodies
//! carry their own `#[cfg(target_arch = "wasm32")]` inside the files.

pub mod auth;
pub mod map_view;
pub mod offline;
pub mod route_table;
pub mod transport;
pub mod ui;
pub mod utils;

#[cfg(test)]
pub mod test_support;
