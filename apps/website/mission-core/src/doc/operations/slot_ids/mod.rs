//! Role: Module boundary for doc/operations/slot_ids.
//! Position: `doc/operations/slot_ids` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use crate::doc::MissionDocCore;
use std::collections::{HashMap, HashSet};
mod duplicates;
/// Expose duplicates :: duplicate slot ids at this domain boundary.
pub use duplicates::duplicate_slot_ids;
#[cfg(test)]
mod tests;
