//! Role: the `slot_ids` module tree and the items it re-exports.
//! Position: the `slot_ids` module of `mission_operations`; hosted commands drive it.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use mission_document::MissionDocCore;
use std::collections::{HashMap, HashSet};
mod duplicates;
/// Expose duplicates :: duplicate slot ids at this domain boundary.
pub use duplicates::duplicate_slot_ids;
#[cfg(test)]
mod tests;
