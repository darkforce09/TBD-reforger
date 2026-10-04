//! The mission hub's routed page components, for `use mission_hub_pages::prelude::*;`.
//!
//! **Role:** re-exports the two route components the app's route table mounts.
//! **Position:** a re-export list over the crate's own modules.
//! **Signals & state:** none.
//! **Invariants:** re-exports only; every item keeps its home module; the route components exist
//! only on `wasm32`, so they are re-exported only there.

#[cfg(target_arch = "wasm32")]
pub use crate::library::MissionLibraryPage;
#[cfg(target_arch = "wasm32")]
pub use crate::overview::MissionOverviewPage;
