//! Camera for the left editor dock.

use super::*;

/// `world_assets::camera_snapshot` seam the scale bar and grid-reference overlay use. `None` before
/// the engine mounts (and always on the native build), in which case the add verb declines rather
/// than saving a bookmark pointing at nothing.
#[must_use]
pub fn live_camera() -> Option<(f64, f64, f64)> {
    #[cfg(target_arch = "wasm32")]
    {
        website_map_engine::streaming::host::camera_snapshot()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        None
    }
}

/// does not, and keeps the current zoom).
///
/// This is NOT a second camera mover. It resolves zoom then forwards to `world_assets::fly_to`,
/// which runs `set_view` → `on_camera_changed` → `flush_viewport` on the registered `RENDER_CTX`
/// before the engine mounts.
#[allow(unused_variables)]
pub fn fly_to(x: f64, y: f64, zoom: Option<f64>) {
    #[cfg(target_arch = "wasm32")]
    {
        let Some(z) = zoom.or_else(|| live_camera().map(|(_, _, z)| z)) else {
            return; // no engine yet — nothing to fly.
        };
        website_map_engine::streaming::host::fly_to(x, y, z);
    }
}

/// Empty before the engine / label host mounts; the index is an accelerator, so an empty list is
/// not an error state.
pub(super) fn load_named_places() -> Vec<NamedPlace> {
    #[cfg(target_arch = "wasm32")]
    {
        let mut out: Vec<NamedPlace> = website_map_engine::streaming::host::named_locations()
            .into_iter()
            .filter(|l| !l.name.trim().is_empty())
            .map(|l| NamedPlace {
                name: l.name,
                x: l.x,
                y: l.y,
                kind: l.kind.unwrap_or_default(),
            })
            .collect();
        sort_places(&mut out);
        out
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        Vec::new()
    }
}
