//! The doctrine and info route components the app's route table mounts, for
//! `use doctrine_pages::prelude::*;`.
//!
//! **Role:** re-exports the three route components of the doctrine hub.
//! **Position:** a re-export list over the crate's own modules.
//! **Signals & state:** none.
//! **Invariants:** re-exports only; every item keeps its home module; the route components fetch
//! in the browser only, so they are re-exported on `wasm32`, where they exist.

#[cfg(target_arch = "wasm32")]
pub use crate::modpacks::ModpacksPage;
#[cfg(target_arch = "wasm32")]
pub use crate::vehicles::VehicleDatabasePage;
#[cfg(target_arch = "wasm32")]
pub use crate::wiki::WikiPage;
