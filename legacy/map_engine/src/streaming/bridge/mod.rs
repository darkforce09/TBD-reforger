//! Role: Module boundary for streaming/bridge.
//! Position: `streaming/bridge` in the map engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Preferences.
pub mod preferences;

/// Progress.
pub mod progress;

/// Statistics.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod statistics;

/// Host preferences.
pub mod host_preferences;
