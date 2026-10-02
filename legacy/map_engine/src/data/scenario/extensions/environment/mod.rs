//! Role: Module boundary for mission/extensions/environment.
//! Position: `mission/extensions/environment` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

/// Audio emitters and cues.
pub mod audio;

/// Weather keyframes.
pub mod weather;
