//! The building blocks of the mission document below its rows.
//!
//! **Role:** the ordered id lists held as native `yrs` arrays ([`id_arrays`]), the row-aligned
//! slot columns the map draws and picks from with their string interner ([`soa`]), and the
//! clocks, window and cap that group local edits into undo steps ([`undo_groups`]).
//! **Position:** mission tier 1, over `time_source` and `yrs`. The mission document
//! (`mission_document`) builds its rows, materialised slot columns and undo manager from it;
//! the map engine and the Mission Creator read [`soa::SlotSoa`].
//! **Signals & state:** the grouping clock holds an atomic depth and anchor; everything else is
//! pure functions over `yrs` transactions the caller owns.
//! **Invariants:** only `slotIds` and `entityIds` are native arrays; the undo clock never reads 0
//! (an anchor of 0 means no open group); the host clock enters as a `time_source::Clock`, so no
//! public clock signature names a `yrs` type.

pub mod id_arrays;
pub mod prelude;
pub mod soa;
pub mod undo_groups;
