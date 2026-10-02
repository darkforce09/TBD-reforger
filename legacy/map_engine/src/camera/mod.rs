//! **Role:** the render engine's viewport methods: resize, pan, zoom and the world and screen
//! answers of its camera.
//! **Position:** `camera` in the map engine; the Mission Creator and the debug benches call these
//! methods on `RenderEngine`. The cameras, matrix routines and grid reference are the
//! `camera_math` and `map_coordinates` crates, which every caller imports directly.
//! **Signals & state:** none here; `viewport` moves the render engine's camera.
//! **Invariants:** the cameras and the grid reference have one definition each, in their crates;
//! this module adds only the browser-build entry points.

/// Where the camera is: resize, pan/zoom entry points, and the world↔screen answers.
// Its `on_camera_changed` calls `overlay::symbology::instances::symbols::cluster_mode`: what the
// camera did decides whether symbols cluster.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod viewport;
