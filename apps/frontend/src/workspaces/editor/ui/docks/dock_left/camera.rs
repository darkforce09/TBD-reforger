//! Camera for the left editor dock.

#[cfg(target_arch = "wasm32")]
use super::*;

/// `world_assets::camera_snapshot` seam the scale bar and grid-reference overlay use. `None` before
/// the engine mounts (and always on the native build), in which case the add verb declines rather
/// than saving a bookmark pointing at nothing.
#[cfg(target_arch = "wasm32")]
#[must_use]
pub fn live_camera() -> Option<(f64, f64, f64)> {
    {
        map_streaming_host::camera_snapshot()
    }
}

/// does not, and keeps the current zoom).
///
/// This is NOT a second camera mover. It resolves zoom then forwards to `world_assets::fly_to`,
/// which runs `set_view` → `on_camera_changed` → `flush_viewport` on the registered `RENDER_CTX`
/// before the engine mounts.
#[cfg(target_arch = "wasm32")]
pub fn fly_to(x: f64, y: f64, zoom: Option<f64>) {
    {
        let Some(z) = zoom.or_else(|| live_camera().map(|(_, _, z)| z)) else {
            return; // no engine yet — nothing to fly.
        };
        map_streaming_host::fly_to(x, y, z);
    }
}

/// Empty before the engine / label host mounts; the index is an accelerator, so an empty list is
/// not an error state.
#[cfg(target_arch = "wasm32")]
pub(super) fn load_named_places() -> Vec<NamedPlace> {
    {
        let mut out: Vec<NamedPlace> = map_streaming_host::named_locations()
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
}
