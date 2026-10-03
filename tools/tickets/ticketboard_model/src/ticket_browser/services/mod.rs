//! The browser's filters and scope facets.
//!
//! **Role:** declares `filtering` (per-ticket facts and composable filters) and `scope_facets` (the
//! narrowed scope dropdowns).
//! **Position:** run by `crate::application_state` when a filter changes or the corpus reloads.
//! **Signals & state:** none here; see each module.
//! **Invariants:** filters change the projection, never the registry.

pub mod filtering;
pub mod scope_facets;
