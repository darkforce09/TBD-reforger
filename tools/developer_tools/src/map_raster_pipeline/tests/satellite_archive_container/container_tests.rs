//! Version 2 `TBDS` container contracts: a v2 bundle carries the same tiles at the same rects as
//! the v1 bundle built from one source, a malformed index (short length, a grid disagreeing with
//! `tile_px`, an unknown tile format) is refused rather than read, and the v2 geometry derivation
//! reproduces the committed Everon tiling.

use serde_json::Value;

use super::*;
use crate::map_raster_pipeline::satellite_archive::build_tbds_v1_bytes;

/// 5 px base at `tile_px` 2: the chain is 5 → 2 → 1 (three mips), and level 0 is a ragged 3×3
/// grid whose last column and row are 1 px wide. That raggedness is the point — a reader that
/// treats `tile_px` as the *actual* tile edge rather than the threshold derives 2 px there and
/// lands every tile of the level in the wrong place while every cast still succeeds.
fn synthetic() -> (Vec<TileBuf>, Vec<(usize, usize)>) {
    let meta = mip_dims(5, 5);
    let mut blocks = Vec::new();
    for (level, &(lw, lh)) in meta.iter().enumerate() {
        let (wt, ht) = (lw.div_ceil(2), lh.div_ceil(2));
        for i in 0..wt * ht {
            let (x, y, w, h) = tile_rect(lw, lh, wt, ht, i);
            // A distinct fill per tile, so identical payload bytes mean identical tiles.
            let data = vec![(level * 37 + i * 11 + 1) as u8; w * h * 3];
            let buf =
                image_operations::encode_webp_lossless_rgb(&image_operations::Rgb8 { w, h, data })
                    .expect("vp8l");
            blocks.push(TileBuf {
                level,
                x,
                y,
                w,
                h,
                buf,
            });
        }
    }
    (blocks, meta)
}

fn v2_of(blocks: &[TileBuf], meta: &[(usize, usize)]) -> Vec<u8> {
    let index = tbds_v2_index(blocks, meta, (5, 5), 2).expect("index");
    tbds_v2_bytes(&index, blocks).expect("frame")
}

/// The acceptance, at container level: one source, two containers, and the tiles a reader
/// finds must be the same bytes at the same rects. The payload comparison is what makes
/// "renders identically at every mip" checkable without a GPU.
#[test]
fn v1_and_v2_carry_the_same_tiles_at_the_same_rects() {
    let (blocks, meta) = synthetic();
    let v1 = build_tbds_v1_bytes(
        &blocks,
        &meta,
        (5, 5),
        "arland",
        [0, 0, 4096, 4096],
        &Value::Null,
        "0",
    )
    .expect("v1");
    let v2 = v2_of(&blocks, &meta);

    let json_len = u32::from_le_bytes(v1[8..12].try_into().unwrap()) as usize;
    let idx_len = u32::from_le_bytes(v2[8..12].try_into().unwrap()) as usize;
    assert_eq!(
        &v1[12 + json_len..],
        &v2[HEADER_BYTES + idx_len..],
        "the two containers must share a byte-identical payload"
    );

    let summary = read_bundle_v2(&v2).expect("the v2 bundle must verify");
    assert_eq!(
        (
            summary.base_w,
            summary.base_h,
            summary.mip_count,
            summary.block_count,
            summary.encoding
        ),
        (5, 5, 3, 11, "tbd-sat-v2")
    );

    // Every rect the v1 table states is the rect the v2 grid derives, and at the same absolute
    // byte range — the two indexes describe the same picture.
    let table: Value = serde_json::from_slice(&v1[12..12 + json_len]).expect("v1 json");
    let index_bytes = AlignedArchive::new(&v2[HEADER_BYTES..HEADER_BYTES + idx_len]);
    let index = access_checked::<TbdSatIndexV2>(index_bytes.as_slice()).expect("v2 index");
    let mut seen = 0;
    for (level, &(lw, lh)) in meta.iter().enumerate() {
        let (wt, ht) = (lw.div_ceil(2), lh.div_ceil(2));
        let lv = &index.levels[level];
        let mips = table["mips"][level]["tiles"].as_array().expect("v1 tiles");
        assert_eq!(mips.len(), lv.tiles.len(), "level {level} tile count");
        for (i, t) in mips.iter().enumerate() {
            let (x, y, w, h) = tile_rect(lw, lh, wt, ht, i);
            assert_eq!(
                (
                    t["x"].as_u64(),
                    t["y"].as_u64(),
                    t["width"].as_u64(),
                    t["height"].as_u64()
                ),
                (
                    Some(x as u64),
                    Some(y as u64),
                    Some(w as u64),
                    Some(h as u64)
                ),
                "level {level} tile {i} rect"
            );
            let abs = HEADER_BYTES as u64 + idx_len as u64 + lv.tiles[i].offset.to_native();
            assert_eq!(
                t["offset"].as_u64(),
                Some(12 + json_len as u64 + lv.tiles[i].offset.to_native()),
                "level {level} tile {i} v1 offset"
            );
            assert_eq!(
                &v1[t["offset"].as_u64().unwrap() as usize
                    ..t["offset"].as_u64().unwrap() as usize
                        + t["length"].as_u64().unwrap() as usize],
                &v2[abs as usize..abs as usize + lv.tiles[i].len.to_native() as usize],
                "level {level} tile {i} bytes"
            );
            seen += 1;
        }
    }
    assert_eq!(seen, 11, "9 + 1 + 1 tiles over three mips");
}

/// The perturbation the ticket names: `index_len` one byte short.
///
/// The good container is verified FIRST, so this pin is red for a writer that ships a short
/// `index_len` as well as for one that ships a long one — an `expect_err` on its own passes
/// happily over a writer that was already broken.
#[test]
fn an_index_len_one_byte_short_is_rejected() {
    let (blocks, meta) = synthetic();
    let mut f = v2_of(&blocks, &meta);
    read_bundle_v2(&f).expect("the unmutated container must verify");
    let short = u32::from_le_bytes(f[8..12].try_into().unwrap()) - 1;
    f[8..12].copy_from_slice(&short.to_le_bytes());
    let e = read_bundle_v2(&f).expect_err("a short index_len must not validate");
    assert!(e.contains("rkyv index does not validate"), "{e}");
}

/// Meaning, not layout — isolated so that **only** the grid rule can catch the fault. Moving
/// `tile_px` alone leaves the rects, the offsets, the VP8L dimensions and bytecheck all intact:
/// a verifier without the rule reports OK over a container it reads as a different picture.
#[test]
fn a_grid_that_disagrees_with_tile_px_is_rejected() {
    let (blocks, meta) = synthetic();
    let mut index = tbds_v2_index(&blocks, &meta, (5, 5), 2).expect("index");
    index.tile_px = 4;
    let f = tbds_v2_bytes(&index, &blocks).expect("frame");
    let e = read_bundle_v2(&f).expect_err("a grid that contradicts tile_px must not pass");
    assert!(
        e.contains("level 0: grid 3x3, tile_px 4 over 5x5 expects 2x2"),
        "{e}"
    );
}

#[test]
fn an_unknown_tile_format_is_rejected() {
    let (blocks, meta) = synthetic();
    let mut index = tbds_v2_index(&blocks, &meta, (5, 5), 2).expect("index");
    index.levels[2].tiles[0].format = 1;
    let f = tbds_v2_bytes(&index, &blocks).expect("frame");
    let e = read_bundle_v2(&f).expect_err("an unknown codec must not be handed to a decoder");
    assert!(e.contains("format 1 is not webp"), "{e}");
}

/// The committed `everon-sat.tbd-sat` index, read off the bundle: 14 levels from 12800², and
/// level 0 is four 6400 px quadrants — **not** 8192 + 4608. If the v2 derivation ever stops
/// reproducing that, a regenerated everon paints its quadrants in the wrong places.
#[test]
fn the_v2_derivation_reproduces_the_committed_everon_tiling() {
    let dims = mip_dims(12_800, 12_800);
    assert_eq!(dims.len(), 14);
    assert_eq!(dims[1], (6_400, 6_400));
    assert_eq!(*dims.last().expect("chain"), (1, 1));
    assert_eq!(12_800_usize.div_ceil(8_192), 2, "level 0 is a 2x2 grid");
    let rects: Vec<_> = (0..4).map(|i| tile_rect(12_800, 12_800, 2, 2, i)).collect();
    assert_eq!(
        rects,
        vec![
            (0, 0, 6_400, 6_400),
            (6_400, 0, 6_400, 6_400),
            (0, 6_400, 6_400, 6_400),
            (6_400, 6_400, 6_400, 6_400),
        ]
    );
    assert_eq!(6_400_usize.div_ceil(8_192), 1, "level 1 is one whole tile");
    assert_eq!(tile_rect(6_400, 6_400, 1, 1, 0), (0, 0, 6_400, 6_400));
}
