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

use terrain_elevation::full_resolution::{
    FullResolutionDemHandle, height_from_handle, new_full_resolution_dem_handle,
};

/// Height reader over a shared full-resolution elevation handle.
#[derive(Clone)]
pub struct TerrainHeights {
    handle: FullResolutionDemHandle,
}

impl TerrainHeights {
    /// A reader over a fresh, empty handle.
    #[must_use]
    pub fn new() -> Self {
        Self {
            handle: new_full_resolution_dem_handle(),
        }
    }

    /// The handle a terrain boot publishes the raster into.
    #[must_use]
    pub fn handle(&self) -> FullResolutionDemHandle {
        self.handle.clone()
    }

    /// Ground height in metres at map position `(x, y)`; `None` before the raster loads or
    /// outside it.
    #[must_use]
    pub fn height_at(&self, x: f64, y: f64) -> Option<f64> {
        height_from_handle(&self.handle, x, y)
    }
}

impl Default for TerrainHeights {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "tests/terrain_height_tests.rs"]
mod tests;
