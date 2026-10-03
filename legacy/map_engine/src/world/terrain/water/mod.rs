//! The terrain's water in the map engine: the browser loader of the water files.
//!
//! **Role:** declares the wasm32 water [`loader`]; the water data (bathymetry, mask, inland
//! archive, sea mesh) is `water_bodies`, which its callers import directly.
//! **Position:** `world/terrain/water`, behind wasm32 and `render`; the streaming host owns the
//! loader.
//! **Signals & state:** none here; the loader's `WaterHost` is its own.
//! **Invariants:** the loader decodes every water file through `water_bodies`.

/// Loader.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod loader;
