//! Role: host preferences.
//! Position: `streaming/bridge` in the graphics engine.
//! Signals & state: plain values and function pointers supplied by the embedding frontend.
//! Invariants: read current preferences at the loader's use site, including after awaits.

use super::preferences::WorldLayerPrefs;

/// Render settings read when terrain loading reaches its preference-restoration step.
#[derive(Clone, Copy)]
pub struct RenderPreferences {
    /// Hillshade opacity.
    pub hillshade_opacity: f64,

    /// Show hillshade.
    pub show_hillshade: bool,

    /// Show grid.
    pub show_grid: bool,
}

/// Current-value readers retained by a map host across asynchronous viewport refreshes.
#[derive(Clone, Copy)]
pub struct HostPreferences {
    /// Read per-user world-layer visibility at each refresh.
    pub world_layers: fn() -> WorldLayerPrefs,

    /// Read the currently selected basemap representation.
    pub basemap: fn() -> String,

    /// Read mission render settings when terrain loading restores layer visibility.
    pub render: fn() -> RenderPreferences,
}
