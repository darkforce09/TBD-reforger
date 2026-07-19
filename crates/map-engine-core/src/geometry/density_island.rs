//! T-178 — stitch per-chunk TBDD tree channels into one island grid + pack RGBA8 for GPU.
//! Class-R equations pinned in `.ai/artifacts/t178_inventory.md`.

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

/// Unpack RGBA8 Unorm-style bytes back to u16 (matches FS `round(R*255)+round(G*255)*256` for
/// exact byte inputs).
#[must_use]
pub fn unpack_u16_rgba(rgba: [u8; 4]) -> u16 {
    u16::from(rgba[0]) | (u16::from(rgba[1]) << 8)
}

/// Write one chunk's 65×65 tree channel into the island `u16` buffer (row-major, south = gy=0).
///
/// # Panics
/// If `tree.len() != 65*65` or `cx`/`cy` out of range for Everon.
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

/// T-179 — Y-flip pack as RGBA8Unorm with tree count in R (`min(c,255)`); G=B=0, A=255.
/// Linear-sampleable on WebGPU + WebGL2. Returns `(bytes, bytes_per_row)` padded to 256 B.
/// Shader recovers `count = sample.r * 255`. Everon max tree channel ≪ 255.
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

/// Corner-correct UV for Linear sampling of an `N×N` corner grid spanning the unit square.
/// Texel centers sit on corners: `uv * (N-1)/N + 0.5/N`.
#[must_use]
pub fn corner_sample_uv(uv: [f32; 2], n: f32) -> [f32; 2] {
    let s = (n - 1.0) / n;
    let o = 0.5 / n;
    [uv[0] * s + o, uv[1] * s + o]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_u16_rg_roundtrip() {
        for c in [0u16, 2, 128, 255, 256, 1000, 65535] {
            assert_eq!(unpack_u16_rgba(pack_u16_rgba(c)), c);
        }
    }

    #[test]
    fn stitch_shared_border_identity() {
        // Two adjacent chunks share the vertical edge at gx = 64.
        let mut left = vec![0u16; CHUNK_CORNERS * CHUNK_CORNERS];
        let mut right = vec![0u16; CHUNK_CORNERS * CHUNK_CORNERS];
        for j in 0..CHUNK_CORNERS {
            left[j * CHUNK_CORNERS + (CHUNK_CORNERS - 1)] = 40 + j as u16;
            right[j * CHUNK_CORNERS] = 40 + j as u16;
        }
        let mut island = vec![0u16; ISLAND_CORNERS * ISLAND_CORNERS];
        stitch_chunk_into_island(&mut island, 0, 0, &left);
        stitch_chunk_into_island(&mut island, 1, 0, &right);
        for j in 0..CHUNK_CORNERS {
            let gy = j;
            let edge = island[gy * ISLAND_CORNERS + CHUNK_CELLS];
            assert_eq!(edge, 40 + j as u16);
            assert_eq!(left[j * CHUNK_CORNERS + 64], right[j * CHUNK_CORNERS]);
        }
    }

    #[test]
    fn y_flip_north_is_tex_row_zero() {
        let mut island = vec![0u16; ISLAND_CORNERS * ISLAND_CORNERS];
        // World south-west corner gy=0,gx=0
        island[0] = 11;
        // World north-west corner gy=N-1,gx=0
        island[(ISLAND_CORNERS - 1) * ISLAND_CORNERS] = 22;
        let rgba = pack_island_rgba_yflip(&island);
        // tex (0,0) = north-west = 22
        assert_eq!(unpack_u16_rgba([rgba[0], rgba[1], rgba[2], rgba[3]]), 22);
        // tex (row N-1, col 0) = south-west = 11
        let o = ((ISLAND_CORNERS - 1) * ISLAND_CORNERS) * 4;
        assert_eq!(
            unpack_u16_rgba([rgba[o], rgba[o + 1], rgba[o + 2], rgba[o + 3]]),
            11
        );
    }

    #[test]
    fn island_dims_pin() {
        assert_eq!(ISLAND_CORNERS, 1601);
        assert_eq!(
            CHUNKS_PER_AXIS * CHUNKS_PER_AXIS,
            EVERON_DENSITY_BINS as usize
        );
        assert_eq!(25 * CHUNK_CELLS + 1, ISLAND_CORNERS);
    }

    #[test]
    fn pack_r8_yflip_north_is_tex_row_zero() {
        let mut island = vec![0u16; ISLAND_CORNERS * ISLAND_CORNERS];
        island[0] = 11;
        island[(ISLAND_CORNERS - 1) * ISLAND_CORNERS] = 22;
        let (bytes, bpr) = pack_island_r8_yflip(&island);
        assert_eq!(bpr, align_bytes_per_row(1601 * 4));
        assert_eq!(bytes[0], 22); // north-west
        let o = (ISLAND_CORNERS - 1) * bpr as usize;
        assert_eq!(bytes[o], 11); // south-west
    }

    #[test]
    fn corner_sample_uv_centers() {
        let n = 1601.0;
        let z = corner_sample_uv([0.0, 0.0], n);
        assert!((z[0] - 0.5 / n).abs() < 1e-6);
        let one = corner_sample_uv([1.0, 1.0], n);
        assert!((one[0] - (1.0 - 0.5 / n)).abs() < 1e-6);
    }
}
