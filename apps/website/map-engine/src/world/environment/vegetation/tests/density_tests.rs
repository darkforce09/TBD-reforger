//! Role: density tests.
//! Position: `world/environment/vegetation/tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::world::environment::vegetation::density::*;

#[test]
fn pack_u16_rg_roundtrip() {
    for c in [0u16, 2, 128, 255, 256, 1000, 65535] {
        assert_eq!(unpack_u16_rgba(pack_u16_rgba(c)), c);
    }
}

#[test]
fn stitch_shared_border_identity() {
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

    island[0] = 11;

    island[(ISLAND_CORNERS - 1) * ISLAND_CORNERS] = 22;
    let rgba = pack_island_rgba_yflip(&island);

    assert_eq!(unpack_u16_rgba([rgba[0], rgba[1], rgba[2], rgba[3]]), 22);

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
    assert_eq!(bytes[0], 22);
    let o = (ISLAND_CORNERS - 1) * bpr as usize;
    assert_eq!(bytes[o], 11);
}

#[test]
fn corner_sample_uv_centers() {
    let n = 1601.0;
    let z = corner_sample_uv([0.0, 0.0], n);
    assert!((z[0] - 0.5 / n).abs() < 1e-6);
    let one = corner_sample_uv([1.0, 1.0], n);
    assert!((one[0] - (1.0 - 0.5 / n)).abs() < 1e-6);
}
