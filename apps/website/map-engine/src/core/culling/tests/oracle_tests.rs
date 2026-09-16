//! Role: oracle tests.
//! Position: `core/culling/tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::core::culling::oracle::*;

fn pack_one(x: f32, y: f32, size: f32) -> [u8; 20] {
    let mut b = [0u8; 20];
    b[0..4].copy_from_slice(&x.to_le_bytes());
    b[4..8].copy_from_slice(&y.to_le_bytes());
    b[8..12].copy_from_slice(&size.to_le_bytes());
    b
}

#[test]
fn class_r_inside_outside() {
    let frustum = [0.0, 0.0, 100.0, 100.0];
    assert!(icon_intersects_frustum(50.0, 50.0, 10.0, frustum));
    assert!(!icon_intersects_frustum(200.0, 200.0, 10.0, frustum));

    assert!(icon_intersects_frustum(100.0, 50.0, 0.0, frustum));
}

#[test]
fn class_r_compact_preserves_order_and_count() {
    let mut src = Vec::new();
    src.extend_from_slice(&pack_one(10.0, 10.0, 4.0));
    src.extend_from_slice(&pack_one(500.0, 500.0, 4.0));
    src.extend_from_slice(&pack_one(20.0, 20.0, 4.0));
    let frustum = [0.0, 0.0, 100.0, 100.0];
    let (out, n) = compact_icons_cpu(&src, frustum);
    assert_eq!(n, 2);
    assert_eq!(out.len(), 40);
    assert_eq!(&out[0..20], &src[0..20]);
    assert_eq!(&out[20..40], &src[40..60]);
}

#[test]
fn class_r_1k_random_frusta_count_stable() {
    let mut src = Vec::new();
    let mut s = 0xC0FFEE_u32;
    for _ in 0..500 {
        s = s.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        let x = (s >> 8) as f32 / 16_777_216.0 * 12800.0 - 6400.0;
        s = s.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        let y = (s >> 8) as f32 / 16_777_216.0 * 12800.0 - 6400.0;
        src.extend_from_slice(&pack_one(x, y, 8.0));
    }
    let mut seed = 42_u32;
    for _ in 0..1000 {
        seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        let cx = (seed >> 8) as f64 / 16_777_216.0 * 12800.0 - 6400.0;
        seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        let cy = (seed >> 8) as f64 / 16_777_216.0 * 12800.0 - 6400.0;
        let half = 400.0;
        let frustum = [cx - half, cy - half, cx + half, cy + half];
        let a = count_icons_in_frustum(&src, frustum);
        let (bytes, b) = compact_icons_cpu(&src, frustum);
        assert_eq!(a, b);
        assert_eq!(bytes.len(), (b as usize) * ICON_STRIDE);
    }
}

#[test]
fn storage32_roundtrip() {
    let mut src = Vec::new();
    src.extend_from_slice(&pack_one(1.5, -2.5, 3.0));
    let s32 = pack_icon_storage32(&src);
    assert_eq!(s32.len(), 32);
    let back = unpack_icon_storage32(&s32, 1);
    assert_eq!(back, src);
}

fn fixture_src_frustum() -> (Vec<u8>, Frustum) {
    let mut src = Vec::new();
    src.extend_from_slice(&pack_one(10.0, 10.0, 4.0));
    src.extend_from_slice(&pack_one(500.0, 500.0, 4.0));
    src.extend_from_slice(&pack_one(20.0, 20.0, 4.0));
    src.extend_from_slice(&pack_one(30.0, 30.0, 4.0));
    (src, [0.0, 0.0, 100.0, 100.0])
}

#[test]
fn t938_3_debug_hud_off_skips_cpu_count() {
    let (src, frustum) = fixture_src_frustum();
    let n = cpu_count_for_encode(&src, frustum, false);
    assert!(
        n.is_none(),
        "debug HUD off must skip the CPU frustum scan; got {n:?}"
    );
}

#[test]
fn t938_3_gpu_visible_count_equals_cpu_on_fixture() {
    let (src, frustum) = fixture_src_frustum();
    let cpu = count_icons_in_frustum(&src, frustum);
    let gpu = gpu_workgroup_visible_count(&src, frustum);
    assert_eq!(cpu, 3);
    assert_eq!(
        gpu, cpu,
        "GPU workgroup reduce must match the CPU oracle on the fixture frustum"
    );
}

#[test]
fn t938_3_debug_hud_on_still_counts() {
    let (src, frustum) = fixture_src_frustum();
    let n = cpu_count_for_encode(&src, frustum, true);
    assert_eq!(n, Some(3));
}

#[test]
fn per_lane_cull_params_not_shared() {
    assert!(
        cull_params_is_per_lane(),
        "every cull lane must own its cull_params uniform: one shared buffer plus one \
             staged write per lane makes all lanes read the last lane's src_count"
    );
}
