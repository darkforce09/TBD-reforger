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
}
