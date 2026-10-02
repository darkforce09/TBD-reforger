//! Role: shader contract tests.
//! Position: `shaders/tests` in `render_primitives`.
//! Signals & state: the WGSL source, read as text.
//! Invariants: these assert that the shader agrees with the CPU-side constants in this crate —
//! `text::pack::TEXT_UNIFORM_BYTES` and the atlas grid uniform. They scrub the source rather
//! than compile it so they run on the native test target with no GPU.

use crate::shaders::SHADER_WGSL;

fn text_uniforms_block() -> &'static str {
    let start = SHADER_WGSL
        .find("struct TextUniforms")
        .expect("TextUniforms struct present");
    let end = SHADER_WGSL[start..].find('}').expect("struct closes") + start;
    &SHADER_WGSL[start..end]
}

fn vs_text_body() -> &'static str {
    let start = SHADER_WGSL.find("fn vs_text(").expect("vs_text present");
    let end = SHADER_WGSL[start..]
        .find("fn fs_text(")
        .expect("fs_text follows vs_text")
        + start;
    &SHADER_WGSL[start..end]
}

#[test]
fn g1_text_uniforms_is_16_bytes_no_vec3() {
    let block = text_uniforms_block();
    assert!(
        !block.contains("vec3"),
        "TextUniforms must not use vec3 padding (align-16 makes the struct 32 B \
             against the 16 B min_binding_size — dead text pipeline)"
    );

    assert_eq!(
        block.matches(": f32").count(),
        4,
        "TextUniforms must stay exactly 4×f32 (16 B contract)"
    );
}

#[test]
fn g1_vs_text_has_v_flip() {
    let body = vs_text_body();
    assert!(
        body.contains("1.0 - in.unit.y"),
        "vs_text must flip V (world-top → atlas cell top) like vs_textured"
    );
}

#[test]
fn l2_vs_text_grid_from_uniform() {
    let body = vs_text_body();
    assert!(
        body.contains("text_u.grid_cols") && body.contains("text_u.grid_rows"),
        "vs_text must read atlas grid dims from TextUniforms"
    );
    assert!(
        !body.contains("/ 16.0") && !body.contains("/ 6.0") && !body.contains("% 16u"),
        "vs_text must not hardcode the atlas grid (16/6 remnants)"
    );
}
