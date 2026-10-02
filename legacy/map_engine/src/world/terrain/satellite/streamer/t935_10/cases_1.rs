//! Role: Domain regression cases.
//! Position: `world/terrain/satellite/streamer/t935_10` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::world::terrain::satellite::streamer::archive::mip_dims;

use crate::world::terrain::satellite::streamer::archive::tile_rect;

use super::*;

#[test]
fn index_range_end_sizes_v1_and_v2_from_a_twelve_byte_prefix() {
    let (v1b, v2b) = (v1(), frame(&index()));
    let e1 = index_range_end(&v1b[..12]).expect("v1 prefix");
    let e2 = index_range_end(&v2b[..12]).expect("v2 prefix");
    assert!(e1 >= 11, "v1 end {e1}");
    assert!(e2 >= 31, "v2 end {e2}");
    let i1 = parse_tbd_sat_index_strict(&v1b[..=e1 as usize], v1b.len() as u64).expect("v1");
    let i2 = parse_tbd_sat_index_strict(&v2b[..=e2 as usize], v2b.len() as u64).expect("v2");
    assert_eq!((i1.container_version, i2.container_version), (1, 2));
    assert_eq!(
        index_range_end(&v2b[..12]).unwrap(),
        index_range_end(&v2b[..32]).unwrap(),
        "a 32-byte v2 header must not change the sized end"
    );
    let third = {
        let mut b = MAGIC.to_le_bytes().to_vec();
        b.extend_from_slice(&3u16.to_le_bytes());
        b.extend_from_slice(&0u16.to_le_bytes());
        b.extend_from_slice(&8u32.to_le_bytes());
        b
    };
    match index_range_end(&third) {
        Err(TbdSatError::UnsupportedVersion(3)) => {}
        other => panic!("third version must be named, got {other:?}"),
    }
}

#[test]
fn v1_and_v2_of_one_pyramid_read_back_identical() {
    let (v1b, v2b) = (v1(), frame(&index()));
    let a = parse_tbd_sat_index_strict(&v1b, v1b.len() as u64).expect("v1 strict");
    let b = parse_tbd_sat_index_strict(&v2b, v2b.len() as u64).expect("v2 strict");
    assert_eq!((a.container_version, b.container_version), (1, 2));
    assert_eq!(
        (a.base_width_px, a.base_height_px, a.mip_count),
        (b.base_width_px, b.base_height_px, b.mip_count)
    );
    assert_eq!(rects(&a), rects(&b), "the two indexes describe one pyramid");
    assert_eq!(rects(&a).len(), 11, "9 + 1 + 1 tiles over three mips");
    for (ta, tb) in a
        .mips
        .iter()
        .flat_map(|m| &m.tiles)
        .zip(b.mips.iter().flat_map(|m| &m.tiles))
    {
        let (sa, sb) = (ta.offset as usize, tb.offset as usize);
        assert_eq!(
            &v1b[sa..sa + ta.length as usize],
            &v2b[sb..sb + tb.length as usize],
            "the Range each index points at must hold the same tile"
        );
    }
}

#[test]
fn the_v1_path_is_untouched() {
    let v1b = v1();
    let loose = parse_tbd_sat_index_only(&v1b, v1b.len() as u64).expect("v1 loose");
    assert_eq!(loose.container_version, 1);
    assert_eq!(loose.terrain_id.as_deref(), Some("synthetic"));
    assert_eq!(loose.world_bounds, Some([0.0, 0.0, 5.0, 5.0]));
    assert_eq!(
        pick_base_level(&loose, 2),
        1,
        "5 px does not fit a 2 px limit"
    );
    assert_eq!(pick_preview_level(&loose, 1).width, 1);
}

#[test]
fn v2_reports_the_fields_it_does_not_carry_as_absent() {
    let f = frame(&index());
    let idx = parse_tbd_sat_index_strict(&f, f.len() as u64).expect("v2");
    assert_eq!((idx.terrain_id, idx.world_bounds), (None, None));
}

#[test]
fn an_index_len_one_byte_short_is_an_error() {
    let mut f = frame(&index());
    parse_tbd_sat_index_strict(&f, f.len() as u64).expect("the unmutated container must parse");
    let short = u32::from_le_bytes(f[8..12].try_into().unwrap()) - 1;
    f[8..12].copy_from_slice(&short.to_le_bytes());
    let e = parse_tbd_sat_index_strict(&f, f.len() as u64).expect_err("must not validate");
    assert!(matches!(e, TbdSatError::Archive(_)), "{e}");
}

#[test]
fn a_corrupt_index_is_an_error() {
    let mut f = frame(&index());
    let mid = HEADER_BYTES + 8;
    f[mid] ^= 0xff;
    assert!(parse_tbd_sat_index_strict(&f, f.len() as u64).is_err());
}

#[test]
fn a_grid_that_contradicts_tile_px_is_rejected() {
    let mut i = index();
    assert_eq!(i.tile_px, TILE_PX);
    i.tile_px = 4;
    let f = frame(&i);
    let e = parse_tbd_sat_index_strict(&f, f.len() as u64).expect_err("must not validate");
    assert!(
        format!("{e}").contains("level 0: grid 3x3, tile_px 4 over 5x5 means 2x2"),
        "{e}"
    );
}

#[test]
fn an_unknown_tile_format_is_rejected() {
    let mut i = index();
    i.levels[2].tiles[0].format = 1;
    let f = frame(&i);
    let e = parse_tbd_sat_index_strict(&f, f.len() as u64).expect_err("must not validate");
    assert!(format!("{e}").contains("format 1"), "{e}");
}

#[test]
fn an_unaligned_buffer_still_validates() {
    let f = frame(&index());
    for pad in 1..8usize {
        let mut shifted = vec![0u8; pad];
        shifted.extend_from_slice(&f);
        let view = &shifted[pad..];
        parse_tbd_sat_index_strict(view, view.len() as u64)
            .unwrap_or_else(|e| panic!("pad {pad}: {e}"));
    }
}

#[test]
fn a_third_container_version_is_refused_by_name() {
    let mut f = frame(&index());
    f[4..6].copy_from_slice(&3u16.to_le_bytes());
    let e = parse_tbd_sat_index_strict(&f, f.len() as u64).expect_err("must not validate");
    assert!(matches!(e, TbdSatError::UnsupportedVersion(3)), "{e}");
}

#[test]
fn the_derivation_reproduces_the_committed_everon_tiling() {
    let d = mip_dims(12_800, 12_800);
    assert_eq!(
        (d.len(), d[1], *d.last().expect("chain")),
        (14, (6_400, 6_400), (1, 1))
    );
    assert_eq!(12_800u32.div_ceil(8_192), 2);
    assert_eq!(
        (0..4)
            .map(|i| tile_rect(12_800, 12_800, 2, 2, i))
            .collect::<Vec<_>>(),
        vec![
            (0, 0, 6_400, 6_400),
            (6_400, 0, 6_400, 6_400),
            (0, 6_400, 6_400, 6_400),
            (6_400, 6_400, 6_400, 6_400),
        ]
    );
    assert_eq!(tile_rect(6_400, 6_400, 1, 1, 0), (0, 0, 6_400, 6_400));

    let per_level: Vec<u32> = d
        .iter()
        .map(|&(lw, lh)| lw.div_ceil(8_192) * lh.div_ceil(8_192))
        .collect();
    assert_eq!(per_level[0], 4);
    assert_eq!(per_level.iter().sum::<u32>(), 17);
}
