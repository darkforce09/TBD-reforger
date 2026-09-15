//! Role: Module boundary for mission/extensions/objectives.
//! Position: `mission/extensions/objectives` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

/// Authored task hierarchy and state transitions.
pub mod tasks;

/// Authored victory conditions.
pub mod win_conditions;
