//! The browser map host of map streaming.
//!
//! **Role:** the entry point the Mission Creator and the shared map view mount a map through:
//! `bootstrap` loads a terrain's elevation model and hillshade, satellite basemap and grid, and in
//! the full scope its world objects, forest, water and labels, into the renderer; the `MapHost` it
//! leaves behind refreshes them after each camera settle and answers the camera, place-name, water
//! and line-of-sight queries; the layer switches change the mounted map.
//! **Position:** streaming category, over `map_asset_loading`, whose loaders the host owns and
//! drives, and `map_streaming_model`, whose asset sink is the host's only way to the renderer; the
//! frontend registers the mounted pair in `RENDER_CTX`.
//! **Signals & state:** the thread-local render context and camera gesture flag; one `MapHost` per
//! mounted map, shared through a `HostHandle`.
//! **Invariants:** the host never names a GPU crate or the renderer's type; every module is
//! wasm32 only, so the native build is the crate root and its prelude.

#[cfg(target_arch = "wasm32")]
mod bootstrap;
#[cfg(target_arch = "wasm32")]
mod map_host;
pub mod prelude;
#[cfg(target_arch = "wasm32")]
mod queries;
#[cfg(target_arch = "wasm32")]
mod render_context;
#[cfg(target_arch = "wasm32")]
mod terrain_load;
#[cfg(target_arch = "wasm32")]
mod view_preferences;
#[cfg(target_arch = "wasm32")]
mod viewport;

#[cfg(target_arch = "wasm32")]
pub use bootstrap::bootstrap;
#[cfg(target_arch = "wasm32")]
pub use map_host::{DemGridHandle, HostHandle, MapHost, new_dem_grid_handle, new_host_handle};
#[cfg(target_arch = "wasm32")]
pub use queries::{
    camera_snapshot, fly_to, is_known_dry_land, is_water, named_locations, with_occluder,
    with_occluder_host, with_water_mask,
};
#[cfg(target_arch = "wasm32")]
pub use render_context::RENDER_CTX;
#[cfg(target_arch = "wasm32")]
pub use view_preferences::{apply_basemap_view, apply_grid, apply_hillshade, refresh_world_layers};
#[cfg(target_arch = "wasm32")]
pub use viewport::{flush_viewport, schedule_camera_settle, set_camera_gesture};
