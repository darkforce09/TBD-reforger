//! Role: Module boundary for streaming/host.
//! Position: `streaming/host` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

#![cfg(target_arch = "wasm32")]
thread_local! {

    /// Mounted render context; the embedding frontend owns registration and teardown.
    pub static RENDER_CTX: RefCell<Option<(EngineHandle, HostHandle)>> = const { RefCell::new(None) };
}
thread_local! {

    static CAMERA_GESTURE: Cell<bool> = const { Cell::new(false) };
}
use crate::core::context::handles::EngineHandle;
use crate::streaming::bridge::progress::ProgressFn;
use crate::streaming::bridge::statistics::BridgeHandle;
use crate::streaming::bridge::statistics::new_bridge;
use crate::streaming::bridge::statistics::publish;
use crate::streaming::bridge::statistics::publish_engine;
use crate::world::environment::vegetation::loader::ForestMassHost;

/// Re-export `crate::streaming::loaders::fetch::fetch_bytes`.
pub use crate::streaming::loaders::fetch::fetch_bytes;
use crate::streaming::loaders::fetch::fetch_bytes_streamed;

/// Re-export `crate::streaming::loaders::fetch::fetch_text`.
pub use crate::streaming::loaders::fetch::fetch_text;

/// Re-export `crate::streaming::loaders::occluder_loader::OccluderHost`.
pub use crate::streaming::loaders::occluder_loader::OccluderHost;
use crate::streaming::loaders::world_loader::WorldHost;

/// Re-export `crate::streaming::memory::budget::hud_suffixasmemory_hud_suffix`.
pub use crate::streaming::memory::budget::hud_suffix as memory_hud_suffix;
use crate::world::terrain::dem::png::decode_png_to_meters;
use crate::world::terrain::relief::hillshade::build_hillshade_image;
use crate::world::terrain::relief::host::DemVectors;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
mod queries;
use queries::TERRAIN_M;

/// Re-export `queries::{camera_snapshot,fly_to,is_known_dry_land,is_water,named_locations,with_occluder,with_occluder_host,with_water_mask,}`.
pub use queries::{
    camera_snapshot, fly_to, is_known_dry_land, is_water, named_locations, with_occluder,
    with_occluder_host, with_water_mask,
};
mod state;

/// Re-export `state::{DemGridHandle,HostHandle,MapHost,new_dem_grid_handle,new_host_handle}`.
pub use state::{DemGridHandle, HostHandle, MapHost, new_dem_grid_handle, new_host_handle};
mod preferences;
use preferences::swap_basemap;

/// Re-export `preferences::{apply_basemap_view,apply_grid,apply_hillshade,refresh_world_layers}`.
pub use preferences::{apply_basemap_view, apply_grid, apply_hillshade, refresh_world_layers};
mod viewport;

/// Re-export `viewport::{flush_viewport,schedule_camera_settle,set_camera_gesture}`.
pub use viewport::{flush_viewport, schedule_camera_settle, set_camera_gesture};
mod bootstrap;

/// Re-export `bootstrap::WORLD_LABEL_FILES`.
pub(crate) use bootstrap::WORLD_LABEL_FILES;

/// Re-export `bootstrap::bootstrap`.
pub use bootstrap::bootstrap;
mod terrain;
use terrain::ManifestDem;
use terrain::load_dem_and_hillshade;
use terrain::sat_url_from;
