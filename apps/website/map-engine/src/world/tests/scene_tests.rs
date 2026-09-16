//! Role: scene tests.
//! Position: `world/tests` in the map engine.
//! Signals & state: the anchor-relative synthetic scenes.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::world::scene::*;
use website_graphics_engine::draw::instances::QuadInstance;

// T-0xx Phase 1D: these four cases stayed on this side of the wall when
// `renderers/batching/scene.rs` split. Every one of them encodes the 12.8 km Everon square —
// the calibration quads sit at fixed world metres, and the stress chunk's pinned f32 bits are
// the output of a generator that hardcodes the same bounds.

#[test]
fn calibration_instance_bytes_exact() {
    let instances = calibration_instances();
    let got: &[u8] = bytemuck::cast_slice(&instances);

    let mut expect = Vec::with_capacity(64);
    for v in [
        -100.0_f32, -100.0, 100.0, 100.0, 0.0, 1.0, 0.0, 1.0, 50.0, 50.0, 90.0, 90.0, 1.0, 0.0,
        0.0, 1.0,
    ] {
        expect.extend_from_slice(&v.to_le_bytes());
    }
    assert_eq!(core::mem::size_of::<QuadInstance>(), 32);
    assert_eq!(got, expect.as_slice());
}

const SEED: u64 = 0x1234_5678;

#[test]
fn stress_chunk_is_deterministic_and_chunk_independent() {
    let a = stress_chunk(0, 1_000, SEED);
    let b = stress_chunk(0, 1_000, SEED);
    assert_eq!(
        bytemuck::cast_slice::<_, u8>(&a),
        bytemuck::cast_slice::<_, u8>(&b)
    );
    let c = stress_chunk(1, 1_000, SEED);
    assert_ne!(
        bytemuck::cast_slice::<_, u8>(&a),
        bytemuck::cast_slice::<_, u8>(&c)
    );
}

#[test]
fn stress_chunk_domain_bounds() {
    for inst in stress_chunk(3, 10_000, SEED) {
        let cx = (inst.min[0] + inst.max[0]) / 2.0;
        let cy = (inst.min[1] + inst.max[1]) / 2.0;
        let hs = (inst.max[0] - inst.min[0]) / 2.0;
        assert!((-6_400.0..6_400.0).contains(&cx));
        assert!((-6_400.0..6_400.0).contains(&cy));
        assert!((1.0..=10.0).contains(&hs));
        assert_eq!(inst.color[3], 1.0);
    }
}

#[test]
fn stress_chunk_first_instances_pinned() {
    let c0 = stress_chunk(0, 4, SEED)[0];
    let c1 = stress_chunk(1, 4, SEED)[0];
    let expect_c0 = QuadInstance {
        min: [f32::from_bits(0xC5B6_3386), f32::from_bits(0xC451_A70A)],
        max: [f32::from_bits(0xC5B6_0996), f32::from_bits(0xC450_5786)],
        color: [
            f32::from_bits(0x3F33_2F4A),
            f32::from_bits(0x3F3C_71B5),
            f32::from_bits(0x3F19_A77F),
            1.0,
        ],
    };
    let expect_c1 = QuadInstance {
        min: [f32::from_bits(0x4396_6908), f32::from_bits(0x44EC_A312)],
        max: [f32::from_bits(0x439E_A338), f32::from_bits(0x44EE_B19E)],
        color: [
            f32::from_bits(0x3EE5_BB09),
            f32::from_bits(0x3F22_6D2F),
            f32::from_bits(0x3EB4_F6B9),
            1.0,
        ],
    };
    assert_eq!(c0, expect_c0);
    assert_eq!(c1, expect_c1);
}
