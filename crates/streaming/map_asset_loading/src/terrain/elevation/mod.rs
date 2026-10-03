//! The terrain's elevation model as the browser loads it: the raw grid loader.
//!
//! **Role:** declares the wasm32 raw-grid `loader`; the elevation model itself (the PNG and raw
//! decoders, the manifest, the grid and the sampling) is `terrain_elevation`, which its callers
//! import directly.
//! **Position:** `terrain/elevation` in `map_asset_loading`; the terrain boot of
//! `map_streaming_host` calls the loader.
//! **Signals & state:** none here; the loader's fetch state is its own.
//! **Invariants:** the loader hands every grid it fetches to `terrain_elevation`'s decoder.

/// The browser fetch of a manifest-declared raw grid, streamed with progress.
#[cfg(target_arch = "wasm32")]
pub mod loader;
