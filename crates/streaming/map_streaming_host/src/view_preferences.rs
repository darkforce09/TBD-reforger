//! **Role:** the view preferences applied to the mounted map: the hillshade, the grid, the
//! basemap view (satellite or cartographic map) and a world-layer refresh.
//! **Position:** `view_preferences` in `map_streaming_host`; the Mission Creator's settings
//! dialogs call the `apply_*` functions, the boot and [`crate::map_host::MapHost`] swap the
//! basemap.
//! **Signals & state:** reads [`crate::render_context::RENDER_CTX`]; writes the renderer only
//! through the asset sink.
//! **Invariants:** with no map mounted every call does nothing; a missing map basemap falls back
//! to the satellite with a console warning.

use map_asset_loading::browser_asset_sink::BrowserAssetSinkHandle;

use crate::queries::TERRAIN_M;
use crate::render_context::RENDER_CTX;
use crate::viewport::schedule_camera_settle;

/// Swap basemap.
pub(crate) async fn swap_basemap(engine: &BrowserAssetSinkHandle, terrain: &str, view: &str) {
    if view == "map" {
        let ok = map_asset_loading::terrain::satellite_quadtree::load_map_basemap(
            engine, terrain, TERRAIN_M, TERRAIN_M,
        )
        .await;
        if !ok {
            browser_platform::console_warn!(
                "map basemap tiles unavailable — falling back to satellite"
            );
            map_asset_loading::terrain::satellite_quadtree::show_satellite_basemap(engine);
        }
    } else {
        map_asset_loading::terrain::satellite_quadtree::show_satellite_basemap(engine);
    }
}

/// Apply hillshade.
pub fn apply_hillshade(visible: bool, opacity: f64) {
    RENDER_CTX.with(|c| {
        if let Some((engine, _)) = c.borrow().as_ref()
            && let Some(e) = engine.borrow_mut().sink_mut()
        {
            #[allow(clippy::cast_possible_truncation)]
            e.set_lane_opacity(1, opacity as f32, visible);
        }
    });
}

/// Apply grid.
pub fn apply_grid(visible: bool) {
    RENDER_CTX.with(|c| {
        if let Some((engine, _)) = c.borrow().as_ref()
            && let Some(e) = engine.borrow_mut().sink_mut()
        {
            e.set_grid(TERRAIN_M, TERRAIN_M, true, visible);
        }
    });
}

/// Refresh world layers.
pub fn refresh_world_layers() {
    RENDER_CTX.with(|c| {
        if let Some((engine, host)) = c.borrow().as_ref() {
            schedule_camera_settle(host.clone(), engine.clone());
        }
    });
}

/// Apply basemap view.
pub fn apply_basemap_view(view: &str) {
    RENDER_CTX.with(|c| {
        if let Some((engine, host)) = c.borrow().as_ref() {
            let terrain = host
                .borrow()
                .as_ref()
                .map(|mh| mh.terrain.clone())
                .unwrap_or_default();
            let engine = engine.clone();
            let view = view.to_string();
            wasm_bindgen_futures::spawn_local(async move {
                swap_basemap(&engine, &terrain, &view).await;
            });
        }
    });
}
