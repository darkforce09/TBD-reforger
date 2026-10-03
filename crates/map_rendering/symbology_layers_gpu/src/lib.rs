//! The map symbology's typed GPU layers.
//!
//! **Role:** the GPU half of what the map draws on the terrain as symbols: the slot symbology
//! layer of the Mission Creator's slots, vehicles, markers, comments, clusters and placement
//! preview (`slot_symbology`), the glyph atlas the world icon lanes sample
//! (`glyph_atlas_gpu`), the compute cull of the icon lanes (`icon_cull_gpu`), the world icon
//! lane uploads (`world_icon_lanes`), the lane visibility, tint and grid preferences
//! (`lane_preferences`) and the icon uniform layout they share ([`icon_uniforms`]).
//! **Position:** map rendering tier 4 over the renderer contracts (`renderer_core`), the GPU frame
//! (`gpu_frame`, `gpu_device`) and the CPU symbology (`overlay_instances`, `unit_symbology`,
//! `map_draw_lanes`); the map renderer holds the typed layers as fields, lends them its lane sink
//! and layer context, and binds their atlases when it fills the frame packet.
//! **Signals & state:** each typed layer owns its GPU state (atlas textures and uniform blocks,
//! the slot bridge's columns, the pooled lane buffers, the compute cull); the free functions own
//! none.
//! **Invariants:** no layer names the renderer: every lane write goes through
//! `renderer_core::lane_sink::LaneSink`, keyed by the lane role's `LaneId`. Every module that names
//! a GPU type is `wasm32` only; the native build keeps the icon uniform layout and [`Error`].

mod error;
#[cfg(target_arch = "wasm32")]
pub mod glyph_atlas_gpu;
#[cfg(target_arch = "wasm32")]
pub mod icon_cull_gpu;
pub mod icon_uniforms;
#[cfg(target_arch = "wasm32")]
pub mod lane_preferences;
pub mod prelude;
#[cfg(target_arch = "wasm32")]
pub mod slot_symbology;
#[cfg(target_arch = "wasm32")]
pub mod world_icon_lanes;

pub use error::{Error, Result};
