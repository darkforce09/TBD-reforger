//! Role: Module boundary for mission/compiler.
//! Position: `mission/compiler` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

/// Editor payload and export envelope compilation.
pub mod payload;

/// Canonical game-document compilation.
pub mod flatten;

/// Resource aliases and faction defaults.
pub mod kit;
