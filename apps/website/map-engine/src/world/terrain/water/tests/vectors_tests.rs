//! Role: vectors tests.
//! Position: `world/terrain/water/tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::io::archives::codec::to_bytes;
use crate::io::archives::water::WaterBody;
use crate::io::archives::water::WaterLine;
use crate::io::containers::header::HEADER_BYTES;
use crate::world::terrain::water::vectors::*;

const W: u32 = 4;
const H: u32 = 4;
const MIPS: u16 = 3;
const SCALE: f32 = 0.1;
const WET: (u32, u32) = (1, 2);

fn synth(width: u32, height: u32, mips: u16, wet: &[(u32, u32, u16)]) -> Vec<u8> {
    let head = TbdbHeader::new(width, height, mips, SCALE);
    let mut out = bytemuck::bytes_of(&head).to_vec();
    let (mut w, mut h) = (width as usize, height as usize);
    let mut depth = vec![0_u16; w * h];
    let mut mask = vec![0_u8; w * h];
    for &(x, z, d) in wet {
        depth[z as usize * w + x as usize] = d;
        mask[z as usize * w + x as usize] = 1;
    }
    for level in 0..mips {
        out.extend(depth.iter().flat_map(|d| d.to_le_bytes()));
        out.extend_from_slice(&mask);
        out.resize(out.len().next_multiple_of(4), 0);
        if level + 1 == mips {
            break;
        }
        let (nw, nh) = ((w / 2).max(1), (h / 2).max(1));
        let mut nd = vec![0_u16; nw * nh];
        let mut nm = vec![0_u8; nw * nh];
        for z in 0..h {
            let dz = downsample_index(z as u32, nh as u32) as usize;
            for x in 0..w {
                let dx = downsample_index(x as u32, nw as u32) as usize;
                nd[dz * nw + dx] = nd[dz * nw + dx].max(depth[z * w + x]);
                nm[dz * nw + dx] |= u8::from(mask[z * w + x] != 0);
            }
        }
        (w, h, depth, mask) = (nw, nh, nd, nm);
    }
    out
}

fn mask_4x4() -> WaterMask {
    let bytes = synth(W, H, MIPS, &[(WET.0, WET.1, 30)]);
    WaterMask::from_bytes(&bytes, [0.0, 0.0, 4.0, 4.0]).expect("4x4 mask")
}

fn world_of(t: u32, dim: u32, span: f64) -> f64 {
    f64::from(t) * span / f64::from(dim - 1)
}

fn refusal<T>(r: Result<T, BinaryError>) -> &'static str {
    match r {
        Ok(_) => "ok",
        Err(BinaryError::Truncated { .. }) => "truncated",
        Err(BinaryError::BadMagic { .. }) => "magic",
        Err(BinaryError::UnsupportedVersion { .. }) => "version",
        Err(BinaryError::Misaligned { .. }) => "align",
        Err(BinaryError::LengthMismatch { .. }) => "length",
        Err(BinaryError::Archive { .. }) => "archive",
    }
}

#[test]
fn every_mip_level_agrees_with_level_zero() {
    let wet = [(0_u32, 0_u32, 7_u16), (1, 2, 30), (3, 3, 12)];
    let b = Bathymetry::from_bytes(&synth(W, H, MIPS, &wet)).expect("parse");
    assert_eq!((b.width(), b.height(), b.level_count()), (W, H, MIPS));
    assert_eq!((b.header().depth_scale, b.finest_level()), (SCALE, 0));
    assert!(b.level(MIPS).is_none(), "past the last level");
    for (level, dims) in [(0_u16, (4_u32, 4_u32)), (1, (2, 2)), (2, (1, 1))] {
        let g = b.level(level).expect("level present");
        let n = (dims.0 * dims.1) as usize;
        assert_eq!(
            ((g.width, g.height), g.depth.len(), g.mask.len()),
            (dims, n, n)
        );
    }
    let base = b.level(0).expect("level 0");
    for level in 1..b.level_count() {
        let grid = b.level(level).expect("level present");
        let mut want_mask = vec![0_u8; (grid.width * grid.height) as usize];
        let mut want_depth = vec![0_u16; (grid.width * grid.height) as usize];
        for z in 0..base.height {
            for x in 0..base.width {
                let (mut tx, mut tz) = (x, z);
                for l in 1..=level {
                    let (w, h) = b.header().level_dims(l).expect("dims");
                    tx = downsample_index(tx, w);
                    tz = downsample_index(tz, h);
                }
                let src = base.index(x, z).expect("src index");
                let dst = grid.index(tx, tz).expect("dst index");
                want_mask[dst] |= u8::from(base.mask[src] != 0);
                want_depth[dst] = want_depth[dst].max(base.depth[src]);
            }
        }
        let got_mask: Vec<u8> = grid.mask.iter().map(|&m| u8::from(m != 0)).collect();
        assert_eq!(got_mask, want_mask, "level {level} mask");
        assert_eq!(grid.depth, &want_depth[..], "level {level} depth");
        assert!(
            want_mask.contains(&1) && want_depth.iter().any(|&d| d > 0),
            "level {level} oracle is vacuous — nothing wet folded into it"
        );
    }
}

#[test]
fn wet_and_dry_texels_read_consistently_across_the_predicates() {
    let m = mask_4x4();
    let (x, z) = (world_of(WET.0, W, 4.0), world_of(WET.1, H, 4.0));
    assert_eq!(m.sample(x, z), WaterAt::Water { depth_m: 3.0 });
    assert!(m.is_water(x, z) && !m.is_known_dry_land(x, z));
    assert_eq!(m.depth_m(x, z), Some(3.0));
    for level in 0..MIPS {
        let want = WaterAt::Water { depth_m: 3.0 };
        assert_eq!(m.sample_at_level(x, z, level), want, "level {level}");
    }
    let (dx, dz) = (world_of(3, W, 4.0), world_of(0, H, 4.0));
    assert_eq!(m.sample(dx, dz), WaterAt::Dry);
    assert!(!m.is_water(dx, dz) && m.is_known_dry_land(dx, dz));
    assert_eq!(m.depth_m(dx, dz), Some(0.0));
}

#[test]
fn outside_the_map_is_unknown_not_dry() {
    let m = mask_4x4();
    #[rustfmt::skip]
        let off = [
            (-0.001, 2.0), (4.001, 2.0), (2.0, -0.001), (2.0, 4.001),
            (-1e9, -1e9), (1e9, 1e9), (f64::NAN, 2.0), (2.0, f64::NAN),
            (f64::INFINITY, 2.0), (2.0, f64::NEG_INFINITY),
        ];
    for (x, z) in off {
        assert_eq!(m.sample(x, z), WaterAt::Unknown, "({x}, {z})");
        assert!(!m.is_water(x, z), "({x}, {z}) claimed water off the map");
        assert!(
            !m.is_known_dry_land(x, z),
            "({x}, {z}) claimed KNOWN DRY LAND off the map — this is the answer that puts a \
                 unit outside the world"
        );
        assert_eq!(m.depth_m(x, z), None, "({x}, {z})");
        assert!(m.texel(x, z).is_none(), "({x}, {z})");
    }

    for (x, z) in [(0.0, 0.0), (4.0, 4.0), (0.0, 4.0), (4.0, 0.0)] {
        assert_ne!(m.sample(x, z), WaterAt::Unknown, "corner ({x}, {z})");
    }

    for level in [MIPS, u16::MAX] {
        assert_eq!(m.sample_at_level(2.0, 2.0, level), WaterAt::Unknown);
        assert!(m.texel_at_level(2.0, 2.0, level).is_none());
    }
}

#[test]
fn a_level_suffix_answers_identically_to_the_whole_file() {
    let bytes = synth(W, H, MIPS, &[(WET.0, WET.1, 30)]);
    let full = Bathymetry::from_bytes(&bytes).expect("full");
    assert_eq!(full.finest_level(), 0);
    for first in 1..MIPS {
        let (before, _) = payload_span(full.header(), first).expect("span");
        let start = size_of::<TbdbHeader>() + before;
        let part = Bathymetry::from_level_suffix(*full.header(), first, &bytes[start..])
            .expect("suffix parses");
        assert_eq!(part.finest_level(), first);
        assert_eq!((part.width(), part.height()), (W, H), "header stays whole");
        let a = WaterMask::new(full.clone(), [0.0, 0.0, 4.0, 4.0]).expect("full mask");
        let b = WaterMask::new(part, [0.0, 0.0, 4.0, 4.0]).expect("part mask");
        for tz in 0..H {
            for tx in 0..W {
                let (x, z) = (world_of(tx, W, 4.0), world_of(tz, H, 4.0));
                for level in 0..MIPS {
                    let want = if level < first {
                        WaterAt::Unknown
                    } else {
                        a.sample_at_level(x, z, level)
                    };
                    assert_eq!(
                        b.sample_at_level(x, z, level),
                        want,
                        "suffix from {first}, level {level} at ({x}, {z})"
                    );
                }

                assert_eq!(b.sample(x, z), a.sample_at_level(x, z, first));
            }
        }
        assert_eq!(
            b.level_for_texel_size_m(0.0),
            first,
            "cannot go finer than it has"
        );
    }
}

#[test]
fn suffix_plan_picks_the_finest_level_inside_the_budget() {
    let everon = TbdbHeader::new(12_800, 12_800, 14, 0.1);
    let whole = suffix_plan(&everon, u64::MAX).expect("whole");
    assert_eq!(whole.first_level, 0);
    assert_eq!(whole.file_offset, 32);
    assert_eq!(whole.bytes, 655_359_948);
    for (budget, level, bytes) in [
        (16_u64 << 20, 3_u16, 10_239_948_u64),
        (4 << 20, 4, 2_559_948),
        (1 << 20, 5, 639_948),
        (1024, 10, 572),
    ] {
        let p = suffix_plan(&everon, budget).expect("plan");
        assert_eq!((p.first_level, p.bytes), (level, bytes), "budget {budget}");
        assert!(p.bytes <= budget, "budget {budget} overrun");
        let (before, _) = payload_span(&everon, p.first_level).expect("span");
        assert_eq!(p.file_offset, 32 + before as u64);
    }

    assert!(suffix_plan(&everon, 3).is_none());
    assert!(suffix_plan(&TbdbHeader::new(0, 0, 0, 0.1), u64::MAX).is_none());

    let m = mask_4x4();
    for (target, want) in [(0.5, 0_u16), (1.0, 0), (2.0, 1), (100.0, 2)] {
        assert_eq!(m.level_for_texel_size_m(target), want, "{target} m");
    }
}

#[test]
fn a_suffix_longer_than_its_level_is_refused() {
    let whole = synth(W, H, MIPS, &[(1, 2, 30)]);
    let header = TbdbHeader::new(W, H, MIPS, SCALE);
    let (skip, bytes) = payload_span(&header, 1).expect("span");
    let payload = &whole[size_of::<TbdbHeader>() + skip..];
    assert_eq!(payload.len(), bytes, "the fixture's own suffix is exact");
    assert!(Bathymetry::from_level_suffix(header, 1, payload).is_ok());

    let from_zero = &whole[size_of::<TbdbHeader>()..];
    assert!(
        from_zero.len() > bytes,
        "the fixture must actually be longer: {} vs {bytes}",
        from_zero.len()
    );
    let err = Bathymetry::from_level_suffix(header, 1, from_zero)
        .expect_err("a body that does not start at first_level must not be read");
    println!("── over-long suffix ── {err}");
    assert!(matches!(err, BinaryError::LengthMismatch { .. }));
}

#[test]
fn a_truncated_suffix_is_refused() {
    let bytes = synth(W, H, MIPS, &[(1, 2, 30)]);
    let head = TbdbHeader::new(W, H, MIPS, SCALE);
    let (before, after) = payload_span(&head, 1).expect("span");
    let start = size_of::<TbdbHeader>() + before;
    let mut bad_magic = head;
    bad_magic.magic = *b"vers";
    let short = &bytes[start..start + after - 4];
    #[rustfmt::skip]
        let cases = [
            ("whole suffix", head, 1_u16, &bytes[start..], "ok"),
            ("4 B short", head, 1, short, "truncated"),
            ("no such level", head, MIPS, &bytes[start..], "length"),
            ("LFS header", bad_magic, 0, &bytes[32..], "magic"),
        ];
    for (what, h, first, tail, want) in cases {
        let got = refusal(Bathymetry::from_level_suffix(h, first, tail));
        assert_eq!(got, want, "{what}");
    }
}

#[test]
fn the_world_to_texel_mapping_is_a_vertex_grid() {
    let m = mask_4x4();
    assert_eq!(m.texel(0.0, 0.0), Some((0, 0)));
    assert_eq!(m.texel(4.0, 4.0), Some((3, 3)));
    for t in 0..W {
        let x = world_of(t, W, 4.0);
        assert_eq!(m.texel(x, 0.0), Some((t, 0)), "sample {t} at x={x}");
    }

    let mid = (world_of(1, W, 4.0) + world_of(2, W, 4.0)) / 2.0;
    assert_eq!(m.texel(mid + 0.01, 0.0), Some((2, 0)));
    assert_eq!(m.texel(mid - 0.01, 0.0), Some((1, 0)));
}

#[test]
fn odd_dimensions_fold_without_running_off_the_end() {
    for (src, dst_dim, want) in [
        (0, 2, 0),
        (1, 2, 0),
        (2, 2, 1),
        (3, 2, 1),
        (4, 2, 1),
        (9, 1, 0),
    ] {
        assert_eq!(downsample_index(src, dst_dim), want, "{src} into {dst_dim}");
    }
    let bytes = synth(5, 5, 3, &[(4, 4, 9)]);
    let b = Bathymetry::from_bytes(&bytes).expect("parse 5x5");
    for level in 0..3 {
        let g = b.level(level).expect("level");
        let want = (5_u32 >> level).max(1);
        assert_eq!((g.width, g.height), (want, want), "level {level}");
        assert!(
            g.mask.iter().any(|&m| m != 0),
            "level {level} lost the corner lake"
        );
    }
}

#[test]
fn a_malformed_container_or_extent_is_refused_not_guessed() {
    assert_eq!(HEADER_BYTES, 32);
    let good = synth(W, H, MIPS, &[(1, 2, 30)]);
    #[rustfmt::skip]
        let extents = [
            [0.0, 0.0, 0.0, 4.0], [0.0, 0.0, 4.0, 0.0], [4.0, 0.0, 0.0, 4.0],
            [0.0, 0.0, f64::NAN, 4.0], [0.0, 0.0, f64::INFINITY, 4.0],
        ];
    for bounds in extents {
        let b = Bathymetry::from_bytes(&good).expect("parse");
        assert!(WaterMask::new(b, bounds).is_none(), "{bounds:?}");
        assert!(WaterMask::from_bytes(&good, bounds).is_err(), "{bounds:?}");
    }
    let patch = |at: std::ops::Range<usize>, v: &[u8]| {
        let mut b = good.clone();
        b[at].copy_from_slice(v);
        b
    };
    let cut = good[..good.len() - 4].to_vec();
    #[rustfmt::skip]
        let cases: [(&str, Vec<u8>, &str); 7] = [
            ("head cut off", good[..8].to_vec(), "truncated"),
            ("last level cut", cut, "truncated"),
            ("an LFS pointer", patch(0..4, b"vers"), "magic"),
            ("a future version", patch(4..6, &9_u16.to_le_bytes()), "version"),
            ("mip_count 0", patch(6..8, &0_u16.to_le_bytes()), "length"),
            ("width 0", patch(8..12, &0_u32.to_le_bytes()), "length"),
            ("height 0", patch(12..16, &0_u32.to_le_bytes()), "length"),
        ];
    for (what, bytes, want) in cases {
        assert_eq!(refusal(Bathymetry::from_bytes(&bytes)), want, "{what}");
    }
    let mut big = patch(8..12, &40_000_u32.to_le_bytes());
    big[12..16].copy_from_slice(&40_000_u32.to_le_bytes());
    assert_eq!(refusal(Bathymetry::from_bytes(&big)), "truncated", "40000²");
}

#[test]
fn a_misaligned_file_buffer_parses_identically() {
    let good = synth(W, H, MIPS, &[(1, 2, 30)]);
    let mut shifted = vec![0_u8];
    shifted.extend_from_slice(&good);
    let a = Bathymetry::from_bytes(&good).expect("aligned");
    let b = Bathymetry::from_bytes(&shifted[1..]).expect("misaligned");
    for level in 0..MIPS {
        let (ga, gb) = (a.level(level).expect("a"), b.level(level).expect("b"));
        assert_eq!(ga.depth, gb.depth, "level {level}");
        assert_eq!(ga.mask, gb.mask, "level {level}");
    }
}

#[rustfmt::skip]
    fn vectors_archive(schema_version: u16) -> rkyv::util::AlignedVec {
        let lake = WaterBody { id: "lake_1".into(), surface_y: 84.762,
                               ring: vec![[0.0, 0.0], [4.0, 0.0], [4.0, 4.0]] };
        let river = WaterLine { id: "river_1".into(), width_m: 12.5,
                                centerline: vec![[0.0, 1.0], [4.0, 1.0]] };
        to_bytes(&WaterVectorsArchive { schema_version, lakes: vec![lake],
                                        rivers: vec![river], ponds: Vec::new() })
            .expect("serialise")
    }

#[test]
fn water_vectors_read_in_place_and_refuse_a_foreign_or_corrupt_file() {
    let good = vectors_archive(ARCHIVE_SCHEMA_VERSION);
    let mut shifted = vec![0_u8];
    shifted.extend_from_slice(&good);
    let v = WaterVectors::from_bytes(&shifted[1..]).expect("parse");
    assert_eq!(v.counts().expect("counts"), (1, 1, 0));
    let a = v.archive().expect("archive");
    assert_eq!(a.lakes[0].id.as_str(), "lake_1");
    assert_eq!(a.lakes[0].surface_y.to_native(), 84.762);
    assert_eq!(a.lakes[0].ring.len(), 3);
    assert_eq!(a.rivers[0].width_m.to_native(), 12.5);
    assert_eq!(a.rivers[0].centerline.len(), 2);

    let future = vectors_archive(ARCHIVE_SCHEMA_VERSION + 1);
    assert!(
        access_checked::<WaterVectorsArchive>(&future).is_ok(),
        "the future-schema buffer must be structurally valid, or this proves nothing"
    );
    let mut flipped = good.to_vec();
    let n = flipped.len();
    flipped[n - 3] ^= 0xFF;
    #[rustfmt::skip]
        let cases = [
            ("a future schema", future.to_vec(), "version"),
            ("half a file", good[..good.len() / 2].to_vec(), "archive"),
            ("no file", Vec::new(), "archive"),
        ];
    for (what, bytes, want) in cases {
        assert_eq!(refusal(WaterVectors::from_bytes(&bytes)), want, "{what}");
    }
    assert_ne!(
        refusal(WaterVectors::from_bytes(&flipped)),
        "ok",
        "bit flip"
    );
}
