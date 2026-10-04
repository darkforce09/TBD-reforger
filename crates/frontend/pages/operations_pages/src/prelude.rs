//! The operations route components the app's route table mounts, for
//! `use operations_pages::prelude::*;`.
//!
//! **Role:** re-exports the five route components of the operations hub.
//! **Position:** a re-export list over the crate's own modules.
//! **Signals & state:** none.
//! **Invariants:** re-exports only; every item keeps its home module; the route components fetch
//! in the browser only, so they are re-exported on `wasm32`, where they exist.

#[cfg(target_arch = "wasm32")]
pub use crate::deployments::DeploymentsPage;
#[cfg(target_arch = "wasm32")]
pub use crate::event_detail::EventHubPage;
#[cfg(target_arch = "wasm32")]
pub use crate::leaderboards::LeaderboardsPage;
#[cfg(target_arch = "wasm32")]
pub use crate::orbat_selection::OrbatSelectionPage;
#[cfg(target_arch = "wasm32")]
pub use crate::schedule::EventSchedulePage;
