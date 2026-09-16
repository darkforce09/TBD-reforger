//! Role: preferences.
//! Position: `streaming/host` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use super::*;

/// Swap basemap.
pub(super) async fn swap_basemap(engine: &EngineHandle, terrain: &str, view: &str) {
    if view == "map" {
        let ok = crate::world::terrain::satellite::quadtree::load_map_basemap(
            engine, terrain, TERRAIN_M, TERRAIN_M,
        )
        .await;
        if !ok {
            crate::diagnostics::platform::console::warn!(
                "map basemap tiles unavailable — falling back to satellite"
            );
            crate::world::terrain::satellite::quadtree::show_satellite_basemap(engine);
        }
    } else {
        crate::world::terrain::satellite::quadtree::show_satellite_basemap(engine);
    }
}

/// Apply hillshade.
pub fn apply_hillshade(visible: bool, opacity: f64) {
    RENDER_CTX.with(|c| {
        if let Some((engine, _)) = c.borrow().as_ref()
            && let Some(e) = engine.borrow_mut().as_mut()
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
            && let Some(e) = engine.borrow_mut().as_mut()
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
