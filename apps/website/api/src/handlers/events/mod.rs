//! Events domain — event / ORBAT scheduling plus [`factions`]. T-934.15: the flat
//! files moved here unchanged; the same-named `events.rs` is glob re-exported so
//! `handlers::events::*` paths hold.

mod events;
pub use self::events::*;

pub mod factions;
