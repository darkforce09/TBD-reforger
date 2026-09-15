//! Role: Module boundary for core/context.
//! Position: `core/context` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// State.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod state;

/// Device 1.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod device_1;

/// Device 2.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod device_2;

/// Viewport.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod viewport;

/// Preferences.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod preferences;

/// Handles.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod handles;
