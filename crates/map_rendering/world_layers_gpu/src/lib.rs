//! The streamed world's typed GPU layers.
//!
//! **Role:** the GPU half of the ground the map draws: the building footprints, outlines and
//! fence strips (`building_layer`), the forest density lane (`forest_layer`), the satellite
//! basemap and hillshade texture lanes (`terrain_texture_layer`) and the terrain line of sight
//! overlay's viewshed lane (`terrain_line_of_sight_overlay`), with the textured lane record they
//! share (`textured_lane`) and the texture layout it reports ([`basemap_mode`]).
//! **Position:** map rendering tier 3 over the renderer contracts (`renderer_core`), the GPU frame
//! (`gpu_frame`) and the lane roles (`map_draw_lanes`); the map renderer holds the typed layers as
//! fields, its asset sink forwards the browser loaders' uploads to them, and it lends them its lane
//! sink for each call.
//! **Signals & state:** each typed layer owns its counters and its binding state (bind-group
//! layout, sampler, pending textures); a textured lane's record owns its texture and bind group.
//! **Invariants:** no layer names the renderer: every lane write goes through
//! `renderer_core::lane_sink::LaneSink`, keyed by the lane role's `LaneId`. Every module that names
//! a GPU type is `wasm32` only; the native build keeps the basemap mode and [`Error`].

pub mod basemap_mode;
#[cfg(target_arch = "wasm32")]
pub mod building_layer;
mod error;
#[cfg(target_arch = "wasm32")]
pub mod forest_layer;
pub mod prelude;
#[cfg(any(target_arch = "wasm32", test))]
mod raster_layout;
#[cfg(target_arch = "wasm32")]
pub mod terrain_line_of_sight_overlay;
#[cfg(target_arch = "wasm32")]
pub mod terrain_texture_layer;
#[cfg(target_arch = "wasm32")]
pub mod textured_lane;
#[cfg(any(target_arch = "wasm32", test))]
mod textured_quad;

pub use error::{Error, LayerCall, Result};
