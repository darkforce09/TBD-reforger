//! The names a caller of the document building blocks imports with
//! `use mission_crdt::prelude::*;`: the slot columns, their codes and the undo constants.

pub use crate::soa::{Interner, NONE_IDX, STANCE_CROUCH, STANCE_PRONE, STANCE_STAND, SlotSoa};
pub use crate::undo_groups::{GESTURE_WINDOW_MS, GroupingClock, MAX_UNDO_GROUPS};
