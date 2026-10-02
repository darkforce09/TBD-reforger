//! Root of the domain-driven frontend tree.
//!
//! **Role:** owns the top-level domains the SPA is organised into — shared foundations (`core`),
//! the routed document pages (`pages`), and the standalone CAD workspaces (`apps`): the scenario
//! creator, the tactical planner, the after-action replay player and the engine testbenches, each
//! of which consumes `core` and is reached from a page. Modules appear here as their contents
//! land.
//! **Position:** declared from `main.rs`; nothing here mounts on its own. Route components are
//! reached through `pages`, and every other domain is a library consumed by them.
//! **Signals & state:** none at this level. State lives in the leaf modules.
//! **Invariants:** `core` may never import from `pages` or from an app; the dependency arrow
//! points one way only. A `pub mod` line carries the same `cfg` gate as the code it declares.

pub mod apps;
pub mod core;
pub mod pages;

#[cfg(test)]
#[path = "tests/doc_audit/mod.rs"]
mod doc_audit;
