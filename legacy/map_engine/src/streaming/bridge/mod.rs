//! Role: Module boundary for streaming/bridge.
//! Position: `streaming/bridge` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Preferences.
#[cfg(feature = "io")]
pub mod preferences;

/// Progress.
pub mod progress;

/// Toggles.
#[cfg(feature = "streaming")]
pub mod toggles;

/// Statistics.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod statistics;

/// Host preferences.
#[cfg(feature = "io")]
pub mod host_preferences;
