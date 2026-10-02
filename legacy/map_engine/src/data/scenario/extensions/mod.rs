//! Role: Module boundary for mission/extensions.
//! Position: `mission/extensions` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

/// Shared authored-block registry and compiler carriers.
pub mod authored;

/// Weather and audio extensions.
pub mod environment;

/// Spawn and garrison modules.
pub mod modules;

/// Tasks and victory conditions.
pub mod objectives;

/// Radio nets and frequencies.
pub mod radio;

/// Authored tactical graphics.
pub mod tactical_graphics;
/// Expose authored :: * at this domain boundary.
pub use authored::*;
