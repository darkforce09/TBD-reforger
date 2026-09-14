//! Root of the domain-driven frontend tree.
//!
//! **Role:** owns the four top-level domains the SPA is being reorganised into — shared
//! foundations (`core`), document pages (`pages`), the tactical map engine, and the standalone
//! map applications. Modules appear here as their contents land.
//! **Position:** declared from `main.rs`; nothing here mounts on its own. Route components are
//! reached through `pages`, and every other domain is a library consumed by them.
//! **Signals & state:** none at this level. State lives in the leaf modules.
//! **Invariants:** `core` may never import from `pages` or from an app; the dependency arrow
//! points one way only. A `pub mod` line carries the same `cfg` gate as the code it declares.

pub mod core;
pub mod pages;

#[cfg(test)]
#[path = "doc_audit_tests.rs"]
mod doc_audit;
