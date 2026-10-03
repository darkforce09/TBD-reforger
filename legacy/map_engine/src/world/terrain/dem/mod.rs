//! The terrain's elevation model in the map engine: the browser loader of the raw grid.
//!
//! **Role:** declares the wasm32 raw-grid [`loader`]; the elevation model itself (the PNG and raw
//! decoders, the manifest, the grid and the sampling) is `terrain_elevation`, which its callers
//! import directly.
//! **Position:** `world/terrain/dem`, behind the `world` feature; the terrain boot in
//! `crate::streaming::host` calls the loader.
//! **Signals & state:** none here; the loader's fetch state is its own.
//! **Invariants:** the loader hands every grid it fetches to `terrain_elevation`'s decoder.

/// The browser fetch of a manifest-declared raw grid, streamed with progress.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod loader;
