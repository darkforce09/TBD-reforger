//! The undo grouping of the mission document's local edits.
//!
//! **Role:** the capture window, the grouping clock that adapts a host `time_source::Clock` to
//! `yrs` and freezes while an explicit group is open, the undo options built on it, and the depth
//! cap math.
//! **Position:** re-exports the public items of `clocks.rs`; the mission document builds its
//! undo manager from [`undo_options`] and applies [`hidden_prefix_after`] in its undo depth.
//! **Signals & state:** [`GroupingClock`] holds an atomic group depth and anchor.
//! **Invariants:** see `clocks.rs`: every reading is at least 1 ms; a group is dropped whole.

mod clocks;

/// The window, the cap, the grouping clock, the platform clock, the undo options and the cap math.
pub use clocks::{
    GESTURE_WINDOW_MS, GroupingClock, MAX_UNDO_GROUPS, hidden_prefix_after, platform_clock,
    undo_options,
};

#[cfg(test)]
#[path = "tests/grouping_clock.rs"]
mod tests;
