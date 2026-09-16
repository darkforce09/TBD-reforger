//! Role: Module boundary for renderers/text/layout/tests.
//! Position: `renderers/text/layout/tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use super::*;

use crate::symbology::labels::declutter::LabelSpec;

// T-0xx Phase 2B (Kind A): the three shader-scrub cases that used to sit below
// (`g1_text_uniforms_is_16_bytes_no_vec3`, `g1_vs_text_has_v_flip`,
// `l2_vs_text_grid_from_uniform`) moved to `website-graphics-engine`'s
// `shaders/tests/contract_tests.rs`, taking `SHADER_SRC` and its two block-scrubbing helpers
// with them. They asserted graphics-engine's own shader against graphics-engine's own
// constants and named nothing from this crate, and reading `shaders::SHADER_WGSL` from here
// was the last reach into a GPU-resource module outside a `frame/` seam (gate rule 3b).

fn cell_px(px: &[u8], w: u32, gi: u32, dx: u32, dy: u32) -> [u8; 4] {
    let cell = TEXT_CELL_PX;
    let (col, row) = (gi % TEXT_ATLAS_COLS, gi / TEXT_ATLAS_COLS);
    let x = col * cell + dx;
    let y = row * cell + dy;
    let i = ((y * w + x) * 4) as usize;
    [px[i], px[i + 1], px[i + 2], px[i + 3]]
}

mod cases_1;
