//! **Role:** Module boundary for `mission_model::objectives`.
//! **Position:** `mission_model::objectives` in the `mission_model` crate.
//! **Signals & state:** explicit data inputs; no UI or graphics state.
//! **Invariants:** preserve authored order, numeric precision, and wire representations.

/// Authored task hierarchy and state transitions.
pub mod tasks;

/// Authored victory conditions.
pub mod win_conditions;
