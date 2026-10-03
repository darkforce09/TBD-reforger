//! **Role:** the values and readers an embedding frontend hands the map host: what the terrain
//! boot loads, and the live preference readers consulted at every refresh.
//! **Position:** the frontend (the Mission Creator, the fire-planning map) builds a
//! [`HostPreferences`]; the map engine's terrain boot and map host read it.
//! **Signals & state:** plain values and function pointers; no state of its own.
//! **Invariants:** readers are called at the loader's use site, including after awaits, so a
//! preference changed mid-boot takes effect; the [`BootstrapScope`] is fixed for a host's life.

use crate::world_layer_preferences::WorldLayerPrefs;

/// Which layers a terrain boot loads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BootstrapScope {
    /// Everything the Mission Creator draws: terrain, imagery, grid, world objects, forest,
    /// water and labels. The full-resolution elevation raster is not kept.
    Full,

    /// Terrain and imagery only: manifest, elevation model (with the full-resolution raster kept
    /// for 2 m height sampling), hillshade, satellite imagery, basemap tiles and the grid. World
    /// objects, forest, water and labels are never fetched.
    TerrainAndImagery,
}

impl BootstrapScope {
    /// Whether this scope fetches world objects, forest, water and labels.
    #[must_use]
    pub fn loads_world_content(self) -> bool {
        matches!(self, Self::Full)
    }

    /// Whether this scope keeps the full-resolution elevation raster after boot.
    #[must_use]
    pub fn keeps_full_resolution_dem(self) -> bool {
        matches!(self, Self::TerrainAndImagery)
    }
}

/// Render settings read when terrain loading reaches its preference-restoration step.
#[derive(Clone, Copy)]
pub struct RenderPreferences {
    /// Hillshade layer opacity, `0.0` to `1.0`.
    pub hillshade_opacity: f64,

    /// Whether the hillshade layer shows.
    pub show_hillshade: bool,

    /// Whether the map grid shows.
    pub show_grid: bool,
}

/// Boot scope plus the current-value readers a map host keeps across asynchronous viewport
/// refreshes.
#[derive(Clone, Copy)]
pub struct HostPreferences {
    /// Which layers the terrain boot loads.
    pub scope: BootstrapScope,

    /// Read per-user world-layer visibility at each refresh.
    pub world_layers: fn() -> WorldLayerPrefs,

    /// Read the currently selected basemap representation.
    pub basemap: fn() -> String,

    /// Read mission render settings when terrain loading restores layer visibility.
    pub render: fn() -> RenderPreferences,
}
