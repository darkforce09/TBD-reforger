//! The names most users of the map render diagnostics import with
//! `use map_render_diagnostics::prelude::*;`: the functions the browser hooks publish.

#[cfg(target_arch = "wasm32")]
pub use crate::benchmark::frame_benchmark::render_bench;
#[cfg(target_arch = "wasm32")]
pub use crate::benchmark::stress_pool::{clear_stress, seed_stress};
pub use crate::benchmark::stress_scene::{stress_chunk, stress_chunk_into};
#[cfg(target_arch = "wasm32")]
pub use crate::readback::{
    calibration::calibration_self_check, compute_cull::compute_cull_self_check,
    marquee::marquee_self_check, road_centerline::road_centerline_self_check, scene::readback_rgba,
    sea_band::sea_band_self_check, text::text_self_check, texture::texture_self_check,
    tree_glyph::tree_glyph_self_check, world_building::world_building_self_check,
};
