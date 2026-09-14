//! Every platform page, grouped by the hub its route belongs to.
//!
//! **Role:** hosts the route components and their panels. Each page is a folder with a
//! `page.rs` that owns layout only, plus one file per panel it renders.
//! **Position:** mounted by the router; wrapped by the persistent navigation frame.
//! **Signals & state:** page-local signals live in the page folders themselves.
//! **Invariants:** a page may import from `core`, the map engine and the apps; nothing under
//! `core` may import from here.

pub mod account;
