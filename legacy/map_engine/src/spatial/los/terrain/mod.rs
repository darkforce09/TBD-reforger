//! Line of sight over the elevation model: the browser upload of a viewshed raster.
//!
//! **Role:** owns `overlay`, the GPU upload of a raster as the viewshed lane; the elevation
//! profile, the viewshed and the sliced viewshed job are `terrain_line_of_sight`.
//! **Position:** the map engine's `spatial/los`; the overlay writes the render engine.
//! **Signals & state:** none here; the overlay writes the render engine's viewshed lane.
//! **Invariants:** only `overlay` touches the GPU or the browser.

/// The browser upload and removal of the viewshed lane.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod overlay;
