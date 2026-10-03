//! The terrain's water as the browser loads it: the water files' loader.
//!
//! **Role:** declares the wasm32 water `loader`; the water data (bathymetry, mask, inland
//! archive, sea mesh) is `water_bodies`, which its callers import directly.
//! **Position:** `terrain/water` in `map_asset_loading`; the map host of `map_streaming_host` owns
//! the loader's `WaterHost`.
//! **Signals & state:** none here; the loader's `WaterHost` is its own.
//! **Invariants:** the loader decodes every water file through `water_bodies`.

/// `WaterHost`: fetches the declared water files and uploads the sea and inland water.
#[cfg(target_arch = "wasm32")]
pub mod loader;
