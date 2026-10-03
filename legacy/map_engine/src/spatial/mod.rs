//! **Role:** module boundary for the map engine's spatial code: the browser upload of a viewshed
//! raster under `los`.
//! **Position:** `spatial` in the map engine, behind the `world` feature; the triangle BVH, the
//! point indexes and the three line of sight layers are crates (`spatial_indexes`,
//! `terrain_line_of_sight`, `interior_line_of_sight`, `world_line_of_sight`) their callers import
//! directly.
//! **Signals & state:** none here.
//! **Invariants:** nothing here computes a spatial query; it only draws the results.

/// Line of sight: the browser upload of the viewshed lane.
pub mod los;
