//! Role: scene.
//! Position: `world` in the map engine.
//! Signals & state: the world anchor, and the two synthetic instance scenes built on it.
//! Invariants: this is the half of the old `renderers/batching/scene.rs` that knows a
//! specific 12.8 km world. Everything here is measured in Everon metres; the GPU instance
//! layouts it fills went to `website-graphics-engine`, which must not learn this number.

use website_graphics_engine::draw::geometry;
use website_graphics_engine::draw::instances::QuadInstance;

// T-0xx Phase 2B.1: the three camera-seeding world facts below came from
// `core/context/state.rs`, where they sat beside the GPU device because that is where
// `RenderEngine::create` read them. They are Everon measurements, not engine state, and
// `ANCHOR` — the same 6400 m centre — was already here (Phase 1D).

/// The camera target `RenderEngine::create` opens on — the Everon terrain centre.
pub(crate) const INITIAL_TARGET: [f64; 2] = [6400.0, 6400.0];

/// The zoom `RenderEngine::create` opens on.
pub(crate) const INITIAL_ZOOM: f64 = -2.0;

/// Everon's world bounds in meters, `[minX, minY, maxX, maxY]` — the camera's pan clamp.
pub(crate) const EVERON_BOUNDS: [f64; 4] = [0.0, 0.0, 12_800.0, 12_800.0];

/// Scene anchor in world meters — the Everon terrain center. Uploaded geometry is stored relative to this point so f32 coordinates stay small (≤ 6400 m ⇒ error ≪ 1 px at all zoom levels; bound derived in `OrthoCamera::wgpu_clip_matrix` docs).
pub const ANCHOR: [f64; 2] = [6400.0, 6400.0];

/// The two calibration instances (plan §S4 calibration scene), anchor-relative: - G: green quad, world [6300,6300]…[6500,6500] → relative [-100,-100]…[100,100] - R: red quad, world [6450,6450]…[6490,6490] → relative [50,50]…[90,90], drawn after G.
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

struct Lcg(u32);

impl Lcg {
    fn new(seed: u64, chunk_idx: u32) -> Self {
        let folded = (seed as u32) ^ ((seed >> 32) as u32);
        Self(folded ^ chunk_idx.wrapping_mul(0x9E37_79B9))
    }

    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        self.0
    }

    fn unit(&mut self) -> f32 {
        (self.next() >> 8) as f32 / 16_777_216.0
    }
}

/// Build one stress chunk of `count` deterministic quads: centers uniform over the Everon bounds (anchor-relative [-6400, 6400]²), half-sizes 1–10 m (2–20 m quads), opaque pseudo-random tint. Same `(seed, chunk_idx, count)` ⇒ bit-identical output, asserted by the native byte tests.
#[must_use]
pub fn stress_chunk(chunk_idx: u32, count: usize, seed: u64) -> Vec<QuadInstance> {
    let mut out = Vec::new();
    stress_chunk_into(chunk_idx, count, seed, &mut out);
    out
}

/// [`stress_chunk`] into a caller-owned staging `Vec` — the streaming-upload loop reuses one 64 MiB staging allocation across all chunks, so peak wasm heap is one chunk regardless of total instance count (plan §20M residency).
pub fn stress_chunk_into(chunk_idx: u32, count: usize, seed: u64, out: &mut Vec<QuadInstance>) {
    let mut rng = Lcg::new(seed, chunk_idx);
    out.clear();
    out.reserve(count);
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
}

#[cfg(test)]
#[path = "tests/scene_tests.rs"]
mod tests;

/// Anchor-relative-meters `[minX, minY, maxX, maxY]` (f32) for a world rect — the textured-quad instance geometry, matching the `QuadInstance` anchor contract.
// T-0xx Phase 2B.1: from `renderers/batching/lanes.rs`. The arithmetic is graphics-engine's
// `draw::geometry::world_rect_rel`; what it could not take with it is [`ANCHOR`], which is a
// fact about a specific 12.8 km world. This wrapper is that binding and nothing else.
#[must_use]
pub fn world_rect_rel(min: [f64; 2], max: [f64; 2]) -> [f32; 4] {
    geometry::world_rect_rel(ANCHOR, min, max)
}
