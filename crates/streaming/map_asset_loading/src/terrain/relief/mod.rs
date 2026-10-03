//! The terrain relief as the browser keeps it: the vector grid and its contour and sea lanes.
//!
//! **Role:** declares the wasm32 `dem_vectors` module; the relief computation (contours,
//! hillshade, sea band) is `terrain_relief`, which its callers import directly.
//! **Position:** `terrain/relief` in `map_asset_loading`; the map host of `map_streaming_host` owns
//! the `DemVectors` and syncs it on each settle.
//! **Signals & state:** none here; the lane state is `DemVectors`' own.
//! **Invariants:** the lanes hold only what `terrain_relief` computes.

/// `DemVectors`: the vector grid, and the sea band and contour lanes for each zoom.
#[cfg(target_arch = "wasm32")]
pub mod dem_vectors;
