//! Events domain — event / ORBAT scheduling plus [`factions`]. T-934.15: the flat
//! files moved here unchanged; the same-named `events.rs` is glob re-exported so
//! `handlers::events::*` paths hold.

// Deliberate inception: the domain keeps its same-named root handler file so every
// pre-T-934.15 `handlers::events::…` path resolves through the glob re-export below.
#[allow(clippy::module_inception)]
mod events;
pub use self::events::*;

pub mod factions;
