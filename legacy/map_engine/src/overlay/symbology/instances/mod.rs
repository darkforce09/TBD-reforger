//! Role: Module boundary for symbology/instances.
//! Position: `overlay/symbology/instances` in the map engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Bridge 1.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod bridge_1;

/// Bridge 2.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod bridge_2;

/// Bridge 3.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod bridge_3;

/// Lanes.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod lanes;
