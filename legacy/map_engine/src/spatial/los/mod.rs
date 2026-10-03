//! Line of sight in the map engine: the browser upload of the viewshed lane.
//!
//! **Role:** holds `terrain`, whose `overlay` uploads a viewshed raster as the viewshed lane; the
//! three line of sight layers (over the elevation model, inside one building, through the
//! streamed world) are the `terrain_line_of_sight`, `interior_line_of_sight` and
//! `world_line_of_sight` crates.
//! **Position:** the map engine's `spatial` module.
//! **Signals & state:** none here.
//! **Invariants:** the three layers stay three crates; only the GPU upload lives here.

/// Over the elevation model: the viewshed lane upload.
pub mod terrain;
