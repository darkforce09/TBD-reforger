//! Missions domain — mission library CRUD plus the [`approvals`] queue and asset
//! [`registry`]. T-934.15: the flat files moved here unchanged; the same-named
//! `missions.rs` is glob re-exported so `handlers::missions::*` paths hold.

// Deliberate inception: the domain keeps its same-named root handler file so every
// pre-T-934.15 `handlers::missions::…` path resolves through the glob re-export below.
#[allow(clippy::module_inception)]
mod missions;
pub use self::missions::*;

pub mod approvals;
pub mod registry;
