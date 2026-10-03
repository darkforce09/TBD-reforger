//! The fixed layer preferences of a terrain-and-imagery map view.
//!
//! **Role:** supplies the map host's preference readers for a view that shows only terrain and
//! imagery: satellite basemap, hillshade at its default opacity, grid on, and the
//! terrain-and-imagery boot scope.
//! **Position:** passed to the terrain boot by [`super::mount::mount_map_view`] callers such as
//! the fire-planning map picker; the Mission Creator supplies its own live readers instead.
//! **Signals & state:** none; the readers return constants.
//! **Invariants:** the scope is always
//! [`map_streaming_model::host_preferences::BootstrapScope::TerrainAndImagery`],
//! so world objects, forest, water and labels are never fetched through these preferences.

#[cfg(any(target_arch = "wasm32", test))]
use map_streaming_model::host_preferences::{BootstrapScope, HostPreferences, RenderPreferences};
#[cfg(any(target_arch = "wasm32", test))]
use map_streaming_model::world_layer_preferences::WorldLayerPrefs;

/// Hillshade opacity a terrain-and-imagery view draws with (the engine's default lane opacity).
#[cfg(any(target_arch = "wasm32", test))]
pub const HILLSHADE_OPACITY: f64 = 0.4;

/// The basemap a terrain-and-imagery view shows.
#[cfg(any(target_arch = "wasm32", test))]
pub const SATELLITE_BASEMAP: &str = "satellite";

#[cfg(any(target_arch = "wasm32", test))]
fn satellite_basemap() -> String {
    SATELLITE_BASEMAP.to_string()
}

#[cfg(any(target_arch = "wasm32", test))]
fn terrain_render_preferences() -> RenderPreferences {
    RenderPreferences {
        hillshade_opacity: HILLSHADE_OPACITY,
        show_hillshade: true,
        show_grid: true,
    }
}

/// Preferences for a terrain-and-imagery view: the boot keeps the full-resolution heights and
/// skips every world layer.
#[cfg(any(target_arch = "wasm32", test))]
#[must_use]
pub fn terrain_and_imagery_preferences() -> HostPreferences {
    HostPreferences {
        scope: BootstrapScope::TerrainAndImagery,
        world_layers: WorldLayerPrefs::default,
        basemap: satellite_basemap,
        render: terrain_render_preferences,
    }
}

#[cfg(test)]
#[path = "tests/terrain_preferences_tests.rs"]
mod tests;
