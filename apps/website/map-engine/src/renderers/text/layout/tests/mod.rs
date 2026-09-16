//! Role: Module boundary for renderers/text/layout/tests.
//! Position: `renderers/text/layout/tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use super::*;

use crate::symbology::labels::declutter::LabelSpec;

const SHADER_SRC: &str = website_graphics_engine::shaders::SHADER_WGSL;

fn text_uniforms_block() -> &'static str {
    let start = SHADER_SRC
        .find("struct TextUniforms")
        .expect("TextUniforms struct present");
    let end = SHADER_SRC[start..].find('}').expect("struct closes") + start;
    &SHADER_SRC[start..end]
}

fn vs_text_body() -> &'static str {
    let start = SHADER_SRC.find("fn vs_text(").expect("vs_text present");
    let end = SHADER_SRC[start..]
        .find("fn fs_text(")
        .expect("fs_text follows vs_text")
        + start;
    &SHADER_SRC[start..end]
}

fn cell_px(px: &[u8], w: u32, gi: u32, dx: u32, dy: u32) -> [u8; 4] {
    let cell = TEXT_CELL_PX;
    let (col, row) = (gi % TEXT_ATLAS_COLS, gi / TEXT_ATLAS_COLS);
    let x = col * cell + dx;
    let y = row * cell + dy;
    let i = ((y * w + x) * 4) as usize;
    [px[i], px[i + 1], px[i + 2], px[i + 3]]
}

mod cases_1;
