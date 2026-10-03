//! **Role:** Module boundary for `mission_model::slot_line`.
//! **Position:** `mission_model::slot_line` in the `mission_model` crate.
//! **Signals & state:** explicit data inputs; no UI or graphics state.
//! **Invariants:** preserve authored order, numeric precision, and wire representations.

mod format_slot_line;
/// Expose format slot line :: format slot line at this domain boundary.
pub use format_slot_line::format_slot_line;
#[cfg(test)]
mod tests;
