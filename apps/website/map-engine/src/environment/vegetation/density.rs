//! Role: density.
//! Position: `environment/vegetation` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Everon island corner count: `12800/8 + 1` = `25*(65-1) + 1`.
pub const ISLAND_CORNERS: usize = 1601;

/// Cells per chunk edge (`DENSITY_COLS - 1`).
pub const CHUNK_CELLS: usize = 64;

/// Corners per chunk edge.
pub const CHUNK_CORNERS: usize = 65;

/// Chunks per Everon axis (`12800/512`).
pub const CHUNKS_PER_AXIS: usize = 25;

/// Total density bins on Everon.
pub const EVERON_DENSITY_BINS: u32 = 625;

/// Pack a u16 count into RGBA8 (R=lo, G=hi, B=0, A=255).
#[must_use]
pub fn pack_u16_rgba(c: u16) -> [u8; 4] {
    [(c & 0xFF) as u8, (c >> 8) as u8, 0, 255]
}

/// Unpack RGBA8 Unorm-style bytes back to u16 (matches FS `round(R*255)+round(G*255)*256` for exact byte inputs).
#[must_use]
pub fn unpack_u16_rgba(rgba: [u8; 4]) -> u16 {
    u16::from(rgba[0]) | (u16::from(rgba[1]) << 8)
}

/// Write one chunk's 65×65 tree channel into the island `u16` buffer (row-major, south = gy=0).
pub fn stitch_chunk_into_island(island: &mut [u16], cx: u32, cy: u32, tree: &[u16]) {
    assert_eq!(tree.len(), CHUNK_CORNERS * CHUNK_CORNERS);
    assert!((cx as usize) < CHUNKS_PER_AXIS && (cy as usize) < CHUNKS_PER_AXIS);
    assert_eq!(island.len(), ISLAND_CORNERS * ISLAND_CORNERS);
    let cx = cx as usize;
    let cy = cy as usize;
    for j in 0..CHUNK_CORNERS {
        let gy = cy * CHUNK_CELLS + j;
        for i in 0..CHUNK_CORNERS {
            let gx = cx * CHUNK_CELLS + i;
            island[gy * ISLAND_CORNERS + gx] = tree[j * CHUNK_CORNERS + i];
        }
    }
}

/// Y-flip pack: texture row 0 = north = global `gy = N-1` (matches `vs_textured` UV).
#[must_use]
pub fn pack_island_rgba_yflip(island: &[u16]) -> Vec<u8> {
    assert_eq!(island.len(), ISLAND_CORNERS * ISLAND_CORNERS);
    let mut out = vec![0u8; ISLAND_CORNERS * ISLAND_CORNERS * 4];
    for gy in 0..ISLAND_CORNERS {
        let tex_row = (ISLAND_CORNERS - 1) - gy;
        for gx in 0..ISLAND_CORNERS {
            let c = island[gy * ISLAND_CORNERS + gx];
            let rgba = pack_u16_rgba(c);
            let o = (tex_row * ISLAND_CORNERS + gx) * 4;
            out[o..o + 4].copy_from_slice(&rgba);
        }
    }
    out
}

/// Bytes-per-row alignment required by `wgpu`/`WebGPU` texture uploads.
#[must_use]
pub fn align_bytes_per_row(unpadded: u32) -> u32 {
    (unpadded + 255) & !255
}

/// Pack island r8 yflip.
#[must_use]
pub fn pack_island_r8_yflip(island: &[u16]) -> (Vec<u8>, u32) {
    assert_eq!(island.len(), ISLAND_CORNERS * ISLAND_CORNERS);
    let bpr = align_bytes_per_row((ISLAND_CORNERS as u32) * 4);
    let mut out = vec![0u8; (bpr as usize) * ISLAND_CORNERS];
    for gy in 0..ISLAND_CORNERS {
        let tex_row = (ISLAND_CORNERS - 1) - gy;
        for gx in 0..ISLAND_CORNERS {
            let c = island[gy * ISLAND_CORNERS + gx].min(255) as u8;
            let o = (tex_row * bpr as usize) + gx * 4;
            out[o] = c;
            out[o + 1] = 0;
            out[o + 2] = 0;
            out[o + 3] = 255;
        }
    }
    (out, bpr)
}

/// Corner-correct UV for Linear sampling of an `N×N` corner grid spanning the unit square. Texel centers sit on corners: `uv * (N-1)/N + 0.5/N`.
#[must_use]
pub fn corner_sample_uv(uv: [f32; 2], n: f32) -> [f32; 2] {
    let s = (n - 1.0) / n;
    let o = 0.5 / n;
    [uv[0] * s + o, uv[1] * s + o]
}

#[cfg(test)]
#[path = "tests/density_tests.rs"]
mod tests;
