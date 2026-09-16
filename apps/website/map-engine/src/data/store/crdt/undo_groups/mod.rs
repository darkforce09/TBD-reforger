//! Role: Module boundary for doc/crdt/undo_groups.
//! Position: `doc/crdt/undo_groups` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use std::collections::HashSet;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use yrs::Origin;
use yrs::sync::{Clock, Timestamp};
use yrs::undo::Options as UndoOptions;
mod clocks;

/// Expose clocks :: gesture window ms at this domain boundary.
pub use clocks::GESTURE_WINDOW_MS;

/// Expose clocks ::  grouping clock at this domain boundary.
pub use clocks::GroupingClock;

/// Expose clocks :: max undo groups at this domain boundary.
pub use clocks::MAX_UNDO_GROUPS;

/// Expose clocks ::  manual clock at this domain boundary.
pub use clocks::ManualClock;

/// Expose clocks :: default inner clock at this domain boundary.
pub use clocks::default_inner_clock;

/// Expose clocks :: hidden prefix after at this domain boundary.
pub use clocks::hidden_prefix_after;

/// Expose clocks :: install wasm now at this domain boundary.
pub use clocks::install_wasm_now;

/// Expose clocks :: undo options at this domain boundary.
pub use clocks::undo_options;
#[cfg(test)]
mod tests;
