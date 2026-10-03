//! The mission document: the live, mergeable state the Mission Creator edits.
//!
//! **Role:** [`MissionDocCore`], the `yrs` document of one mission with its tracked root maps,
//! row writes and reads, hydrate and export, merge, materialised slot columns, selection policy
//! and local undo history; the connection, formation and transform vocabulary it speaks.
//! **Position:** mission tier 2, over `mission_crdt`, `time_source`, `yrs` and `serde_json`. The
//! document operations of `mission_operations`, the map engine's editing layer and the Mission
//! Creator drive it; the payload compiler reads the editor payload it exports.
//! **Signals & state:** a [`MissionDocCore`] owns its `yrs` document, undo manager and grouping
//! clock, and interior-mutable init mode, undo cap and side-key memo; single-threaded.
//! **Invariants:** the exported editor payload and the Yjs maps keep their wire shapes byte for
//! byte; local edits are tracked for undo and hydrate and remote updates are not; a slot belongs
//! to one squad once; every refusal of an update is one readable sentence ([`Error`]).

mod error;
pub mod ids;
pub mod prelude;
mod rows;
mod selection;
#[cfg(any(test, feature = "test_fixtures"))]
pub mod test_fixtures;

#[cfg(test)]
mod tests;

/// Why the mission document refuses an incoming update.
pub use error::Error;
/// The result of applying an update to the mission document.
pub use error::Result;
/// The formation offsets a squad's members take behind its leader.
pub use rows::formation_offsets;
/// The connection vocabulary and its validation.
pub use rows::{ConnectionFinding, ConnectionKind, ConnectionRow, validate_connection_rows};
/// The mission document and the plain rows and patches it reads and writes.
pub use rows::{EntityTransformPatch, MergeOpts, MergeReport, MissionDocCore, SquadMembership};
