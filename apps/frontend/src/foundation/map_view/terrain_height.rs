//! Terrain heights at 2 m resolution for a mounted map view.
//!
//! **Role:** answers the ground height at a map position from the full-resolution elevation
//! raster the terrain boot publishes.
//! **Position:** wraps the map engine's
//! [`terrain_elevation::full_resolution::FullResolutionDemHandle`]; a map
//! view hands one to its terrain boot and reads heights through [`TerrainHeights::height_at`].
//! **Signals & state:** shares the handle with the boot task, which fills it once.
//! **Invariants:** answers `None` until the raster has loaded and outside the terrain, never a
//! guessed or clamped height; a scope that does not keep the raster leaves every answer `None`.

#[cfg(any(target_arch = "wasm32", test))]
use terrain_elevation::full_resolution::{
    height_from_handle, new_full_resolution_dem_handle, FullResolutionDemHandle,
};

/// Height reader over a shared full-resolution elevation handle.
#[cfg(any(target_arch = "wasm32", test))]
#[derive(Clone)]
pub struct TerrainHeights {
    handle: FullResolutionDemHandle,
}

#[cfg(any(target_arch = "wasm32", test))]
impl TerrainHeights {
    /// A reader over a fresh, empty handle.
    #[cfg(any(target_arch = "wasm32", test))]
    #[must_use]
    pub fn new() -> Self {
        Self {
            handle: new_full_resolution_dem_handle(),
        }
    }

    /// The handle a terrain boot publishes the raster into.
    #[cfg(any(target_arch = "wasm32", test))]
    #[must_use]
    pub fn handle(&self) -> FullResolutionDemHandle {
        self.handle.clone()
    }

    /// Ground height in metres at map position `(x, y)`; `None` before the raster loads or
    /// outside it.
    #[cfg(any(target_arch = "wasm32", test))]
    #[must_use]
    pub fn height_at(&self, x: f64, y: f64) -> Option<f64> {
        height_from_handle(&self.handle, x, y)
    }
}

#[cfg(any(target_arch = "wasm32", test))]
impl Default for TerrainHeights {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "tests/terrain_height_tests.rs"]
mod tests;
