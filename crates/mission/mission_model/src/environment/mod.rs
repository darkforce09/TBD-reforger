//! **Role:** Module boundary for `mission_model::environment`.
//! **Position:** `mission_model::environment` in the `mission_model` crate.
//! **Signals & state:** explicit data inputs; no UI or graphics state.
//! **Invariants:** preserve authored order, numeric precision, and wire representations.

/// Audio emitters and cues.
pub mod audio;

/// Weather keyframes.
pub mod weather;
