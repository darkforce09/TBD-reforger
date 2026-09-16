//! Role: Module boundary for camera.
//! Position: `camera` in the map engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Math.
pub mod math;

/// Orbit.
pub mod orbit;

/// Ortho.
pub mod ortho;

/// Where the camera is: resize, pan/zoom entry points, and the world↔screen answers.
// T-0xx Phase 2B.1: from `core/context/viewport.rs`. It answers "where is the camera", which
// is this module's question. Its `on_camera_changed` calls
// `overlay::symbology::instances::symbols::cluster_mode` — a camera → overlay cross that is
// real and stays: what the camera did decides whether symbols cluster.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod viewport;
