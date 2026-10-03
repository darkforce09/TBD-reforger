//! The terrain's browser loaders.
//!
//! **Role:** module boundary for the ground's loads: the elevation model's raw grid, the relief's
//! vector grid and lanes, the satellite image and map tiles, and the water files.
//! **Position:** `terrain` in `map_asset_loading`; the map host of `map_streaming_host` drives every
//! load at boot and on each camera settle; the data models are the terrain crates under
//! `crates/terrain/`.
//! **Signals & state:** none here; each loader's state is its own.
//! **Invariants:** every load reaches the renderer only through the asset sink.

/// The elevation model's raw grid loader.
pub mod elevation;

/// The relief's vector grid and its contour and sea lanes.
pub mod relief;

/// The satellite image's preview and mip chain, and the cartographic map tiles.
#[cfg(target_arch = "wasm32")]
pub mod satellite_quadtree;

/// The water files' loader.
pub mod water;
