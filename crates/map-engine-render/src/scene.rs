//! Pure instance-data builders for the render engine — no wgpu/web types, so this module
//! compiles natively and its byte-level tests run under plain `cargo test` (plan §S4b: the
//! exact bytes the GPU upload receives are asserted, closing the "did we upload what we
//! think" gap).
//!
//! Coordinate contract: instance coordinates are **anchor-relative meters** (world minus
//! [`ANCHOR`]), stored f32 once; the per-frame f64 view-projection matrix carries the
//! `target − anchor` translation (`OrthoCamera::wgpu_clip_matrix`). See plan §20M
//! feasibility — anchor rule.

use bytemuck::{Pod, Zeroable};

/// Scene anchor in world meters — the Everon terrain center. Uploaded geometry is stored
/// relative to this point so f32 coordinates stay small (≤ 6400 m ⇒ error ≪ 1 px at all
/// zoom levels; bound derived in `OrthoCamera::wgpu_clip_matrix` docs).
pub const ANCHOR: [f64; 2] = [6400.0, 6400.0];

/// Instance-buffer pool unit: 2^21 instances × 32 B = 64 MiB per GPU buffer — legal by
/// construction under WebGPU's *default* `maxBufferSize` (256 MiB) with 4× headroom, so no
/// device-limit negotiation is ever load-bearing (plan §S4 chunked pool).
pub const CHUNK_CAPACITY: usize = 2_097_152;

/// Unit quad (triangle-strip order) expanded per instance in the vertex shader via
/// `pos = mix(inst.min, inst.max, unit_uv)`. Culling is disabled in the pipeline, so
/// winding is irrelevant.
pub const UNIT_QUAD: [[f32; 2]; 4] = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]];

/// One axis-aligned colored quad instance (anchor-relative meters).
///
/// 32 B — deliberately *heavier* than the pinned ≤ 20 B production icon layout, so every
/// stress measurement is a conservative lower bound on production throughput (plan §S4d).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct QuadInstance {
    /// Anchor-relative [minX, minY] corner, meters.
    pub min: [f32; 2],
    /// Anchor-relative [maxX, maxY] corner, meters.
    pub max: [f32; 2],
    /// RGBA, linear 0..1 (rendered to a non-sRGB target — no transfer function).
    pub color: [f32; 4],
}

/// The two calibration instances (plan §S4 calibration scene), anchor-relative:
/// - G: green quad, world [6300,6300]…[6500,6500] → relative [-100,-100]…[100,100]
/// - R: red quad, world [6450,6450]…[6490,6490] → relative [50,50]…[90,90], drawn after G
///
/// At the fixed probe camera (800×600, zoom 0, target = ANCHOR) every edge lands on an
/// integer pixel coordinate (G: x∈[300,500], y∈[200,400]; R: x∈[450,490], y∈[210,250]),
/// which is what makes the readback probes byte-exact with zero rasterization-rule
/// dependence (plan §S4 margin argument).
#[must_use]
pub fn calibration_instances() -> [QuadInstance; 2] {
    [
        QuadInstance {
            min: [-100.0, -100.0],
            max: [100.0, 100.0],
            color: [0.0, 1.0, 0.0, 1.0],
        },
        QuadInstance {
            min: [50.0, 50.0],
            max: [90.0, 90.0],
            color: [1.0, 0.0, 0.0, 1.0],
        },
    ]
}

/// Deterministic 32-bit LCG (the `meters.parity.test.ts` constants: `s*1103515245+12345`),
/// seeded per chunk so any chunk is independently regenerable — the streaming-upload loop
/// fills one 64 MiB staging buffer per chunk without holding N instances in wasm memory.
struct Lcg(u32);

impl Lcg {
    fn new(seed: u64, chunk_idx: u32) -> Self {
        // Fold the u64 seed and de-correlate chunks with the golden-ratio Weyl constant.
        let folded = (seed as u32) ^ ((seed >> 32) as u32);
        Self(folded ^ chunk_idx.wrapping_mul(0x9E37_79B9))
    }

    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        self.0
    }

    /// Uniform in [0, 1) with 24-bit resolution — exact in f32.
    fn unit(&mut self) -> f32 {
        (self.next() >> 8) as f32 / 16_777_216.0
    }
}

/// Build one stress chunk of `count` deterministic quads: centers uniform over the Everon
/// bounds (anchor-relative [-6400, 6400]²), half-sizes 1–10 m (2–20 m quads), opaque
/// pseudo-random tint. Same `(seed, chunk_idx, count)` ⇒ bit-identical output, asserted by
/// the native byte tests.
#[must_use]
pub fn stress_chunk(chunk_idx: u32, count: usize, seed: u64) -> Vec<QuadInstance> {
    let mut rng = Lcg::new(seed, chunk_idx);
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        let cx = rng.unit() * 12_800.0 - 6_400.0;
        let cy = rng.unit() * 12_800.0 - 6_400.0;
        let hs = 1.0 + rng.unit() * 9.0;
        let r = 0.25 + rng.unit() * 0.75;
        let g = 0.25 + rng.unit() * 0.75;
        let b = 0.25 + rng.unit() * 0.75;
        out.push(QuadInstance {
            min: [cx - hs, cy - hs],
            max: [cx + hs, cy + hs],
            color: [r, g, b, 1.0],
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Class R: the calibration instances' upload bytes, memcmp'd against literals built
    /// from the exact f32 constants of the plan's calibration scene.
    #[test]
    fn calibration_instance_bytes_exact() {
        let instances = calibration_instances();
        let got: &[u8] = bytemuck::cast_slice(&instances);

        let mut expect = Vec::with_capacity(64);
        for v in [
            -100.0_f32, -100.0, 100.0, 100.0, 0.0, 1.0, 0.0, 1.0, // G
            50.0, 50.0, 90.0, 90.0, 1.0, 0.0, 0.0, 1.0, // R
        ] {
            expect.extend_from_slice(&v.to_le_bytes());
        }
        assert_eq!(core::mem::size_of::<QuadInstance>(), 32);
        assert_eq!(got, expect.as_slice());
    }

    const SEED: u64 = 0x1234_5678;

    /// Determinism (Class R): same inputs ⇒ bit-identical bytes; distinct chunks differ.
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

    /// Domain properties: centers within the anchor-relative Everon bounds, half-sizes in
    /// [1, 10] m, alpha exactly 1.
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

    /// Class R cross-oracle pin: the first instance of chunks 0 and 1 at the house seed,
    /// as f32 bit patterns derived from an INDEPENDENT JavaScript implementation of the
    /// generator (Math.imul LCG + Math.fround per f32 op) — two implementations agreeing
    /// bit-for-bit, not a self-snapshot. Any change to the LCG, fold, or arithmetic order
    /// fails this loudly.
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
}
