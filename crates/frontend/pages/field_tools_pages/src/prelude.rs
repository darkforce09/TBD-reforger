//! The field tools' routed page components, for `use field_tools_pages::prelude::*;`.
//!
//! **Role:** re-exports the route component the app's route table mounts.
//! **Position:** a re-export list over the crate's own modules.
//! **Signals & state:** none.
//! **Invariants:** re-exports only; every item keeps its home module; the route component exists
//! only on `wasm32`, so it is re-exported only there.

#[cfg(target_arch = "wasm32")]
pub use crate::mortar::MortarCalculatorPage;
