//! **Role:** the stress scene: deterministic pseudo-random quads over the Everon square that the
//! benchmark's stress lane draws.
//! **Position:** `benchmark` of the map render diagnostics; `stress_pool.rs`'s `seed_stress` fills its
//! staging allocation chunk by chunk through `stress_chunk_into`.
//! **Signals & state:** none; pure functions over a seeded generator.
//! **Invariants:** the quads are measured in Everon metres relative to
//! `map_coordinates::terrain_frames::ANCHOR`; the same `(seed, chunk_idx, count)` gives
//! bit-identical output.

use deterministic_random::LinearCongruential32;
use render_primitives::draw::instances::QuadInstance;

/// The generator one stress chunk draws from: a [`LinearCongruential32`] whose state is the seed
/// folded to 32 bits, XOR the chunk index times the 32-bit golden-ratio constant, so every chunk
/// of one seed draws its own stream.
fn chunk_draws(seed: u64, chunk_idx: u32) -> LinearCongruential32 {
    let folded = (seed as u32) ^ ((seed >> 32) as u32);
    LinearCongruential32::new(folded ^ chunk_idx.wrapping_mul(0x9E37_79B9))
}

/// Build one stress chunk of `count` deterministic quads: centres uniform over the Everon bounds
/// (anchor-relative [-6400, 6400]²), half-sizes 1–10 m (2–20 m quads), opaque pseudo-random tint.
/// The same `(seed, chunk_idx, count)` gives bit-identical output, asserted by the native byte
/// tests.
#[must_use]
pub fn stress_chunk(chunk_idx: u32, count: usize, seed: u64) -> Vec<QuadInstance> {
    let mut out = Vec::new();
    stress_chunk_into(chunk_idx, count, seed, &mut out);
    out
}

/// [`stress_chunk`] into a caller-owned staging `Vec`: the streaming-upload loop reuses one
/// 64 MiB staging allocation across all chunks, so the peak wasm heap is one chunk regardless of
/// the total instance count.
pub fn stress_chunk_into(chunk_idx: u32, count: usize, seed: u64, out: &mut Vec<QuadInstance>) {
    let mut rng = chunk_draws(seed, chunk_idx);
    out.clear();
    out.reserve(count);
    for _ in 0..count {
        let cx = rng.next_unit() * 12_800.0 - 6_400.0;
        let cy = rng.next_unit() * 12_800.0 - 6_400.0;
        let hs = 1.0 + rng.next_unit() * 9.0;
        let r = 0.25 + rng.next_unit() * 0.75;
        let g = 0.25 + rng.next_unit() * 0.75;
        let b = 0.25 + rng.next_unit() * 0.75;
        out.push(QuadInstance {
            min: [cx - hs, cy - hs],
            max: [cx + hs, cy + hs],
            color: [r, g, b, 1.0],
        });
    }
}
