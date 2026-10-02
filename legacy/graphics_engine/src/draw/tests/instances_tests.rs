//! Role: draw instances tests.
//! Position: `draw/tests` in the graphics engine.
//! Signals & state: per-instance byte layouts.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::draw::instances::*;

// T-0xx Phase 1D: moved here with the layouts they pin, from the map engine's
// `renderers/batching/tests/scene_tests.rs`. The calibration and stress cases in that file
// stayed behind with their subjects — they encode a specific 12.8 km world.

#[test]
fn icon_instance_layout_is_20_bytes() {
    assert_eq!(core::mem::size_of::<IconInstance>(), 20);
    assert_eq!(core::mem::align_of::<IconInstance>(), 4);
    let inst = IconInstance {
        pos: [1.5, -2.5],
        size: 3.0,
        yaw: -16384,
        glyph: 7,
        tint: 0xFF27_5A2D,
    };
    let got: &[u8] = bytemuck::bytes_of(&inst);
    assert_eq!(got.len(), 20);
    assert_eq!(f32::from_le_bytes(got[0..4].try_into().unwrap()), 1.5);
    assert_eq!(f32::from_le_bytes(got[4..8].try_into().unwrap()), -2.5);
    assert_eq!(f32::from_le_bytes(got[8..12].try_into().unwrap()), 3.0);
    assert_eq!(i16::from_le_bytes(got[12..14].try_into().unwrap()), -16384);
    assert_eq!(u16::from_le_bytes(got[14..16].try_into().unwrap()), 7);
}

#[test]
fn building_instance_layout_and_bytes_exact() {
    assert_eq!(core::mem::size_of::<BuildingInstance>(), 40);
    let inst = BuildingInstance {
        center: [1.5, -2.5],
        half: [40.0, 20.0],
        basis: [0.25, 0.75],
        color: [38.0 / 255.0, 38.0 / 255.0, 44.0 / 255.0, 1.0],
    };
    let got: &[u8] = bytemuck::cast_slice(core::slice::from_ref(&inst));
    let mut expect = Vec::with_capacity(40);
    for v in [
        1.5_f32,
        -2.5,
        40.0,
        20.0,
        0.25,
        0.75,
        38.0 / 255.0,
        38.0 / 255.0,
        44.0 / 255.0,
        1.0,
    ] {
        expect.extend_from_slice(&v.to_le_bytes());
    }
    assert_eq!(got, expect.as_slice());
}

#[test]
fn shader_uv_table_tracks_atlas_glyph_count() {
    let src = crate::shaders::SHADER_WGSL;
    let arr = format!("array<vec4<f32>, {ATLAS_GLYPH_COUNT}>");
    assert!(
        src.contains(&arr),
        "shader.wgsl must declare the icon UV table as `{arr}`"
    );
    let clamp = format!("min(in.glyph, {}u)", ATLAS_GLYPH_COUNT - 1);
    assert!(
        src.contains(&clamp),
        "shader.wgsl must clamp the glyph index with `{clamp}`"
    );
}
