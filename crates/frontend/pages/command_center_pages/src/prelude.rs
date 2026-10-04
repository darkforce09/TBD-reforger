//! The command center route components the app's route table mounts, for
//! `use command_center_pages::prelude::*;`.
//!
//! **Role:** re-exports the three route components of the command center.
//! **Position:** a re-export list over the crate's own modules.
//! **Signals & state:** none.
//! **Invariants:** re-exports only; every item keeps its home module; the route components fetch
//! in the browser only, so they are re-exported on `wasm32`, where they exist.

#[cfg(target_arch = "wasm32")]
pub use crate::announcements::AnnouncementsPage;
#[cfg(target_arch = "wasm32")]
pub use crate::dashboard::DashboardPage;
#[cfg(target_arch = "wasm32")]
pub use crate::server_intel::ServerIntelPage;
