//! Role: scene tests.
//! Position: `renderers/batching/tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::renderers::batching::scene::*;

#[test]
fn calibration_instance_bytes_exact() {
    let instances = calibration_instances();
    let got: &[u8] = bytemuck::cast_slice(&instances);

    let mut expect = Vec::with_capacity(64);
    for v in [
        -100.0_f32, -100.0, 100.0, 100.0, 0.0, 1.0, 0.0, 1.0, 50.0, 50.0, 90.0, 90.0, 1.0, 0.0,
        0.0, 1.0,
    ] {
        expect.extend_from_slice(&v.to_le_bytes());
    }
    assert_eq!(core::mem::size_of::<QuadInstance>(), 32);
    assert_eq!(got, expect.as_slice());
}

#[test]
fn icon_instance_layout_is_20_bytes() {
    assert_eq!(core::mem::size_of::<IconInstance>(), 20);
    assert_eq!(core::mem::align_of::<IconInstance>(), 4);
    let inst = IconInstance {
        pos: [1.5, -2.5],
        size: 3.0,
        yaw: -16384,
        glyph: 7,
        tint: 0xFF27_5A2D,
    };
    let got: &[u8] = bytemuck::bytes_of(&inst);
    assert_eq!(got.len(), 20);
    assert_eq!(f32::from_le_bytes(got[0..4].try_into().unwrap()), 1.5);
    assert_eq!(f32::from_le_bytes(got[4..8].try_into().unwrap()), -2.5);
    assert_eq!(f32::from_le_bytes(got[8..12].try_into().unwrap()), 3.0);
    assert_eq!(i16::from_le_bytes(got[12..14].try_into().unwrap()), -16384);
    assert_eq!(u16::from_le_bytes(got[14..16].try_into().unwrap()), 7);
}

#[test]
fn building_instance_layout_and_bytes_exact() {
    assert_eq!(core::mem::size_of::<BuildingInstance>(), 40);
    let inst = BuildingInstance {
        center: [1.5, -2.5],
        half: [40.0, 20.0],
        basis: [0.25, 0.75],
        color: [38.0 / 255.0, 38.0 / 255.0, 44.0 / 255.0, 1.0],
    };
    let got: &[u8] = bytemuck::cast_slice(core::slice::from_ref(&inst));
    let mut expect = Vec::with_capacity(40);
    for v in [
        1.5_f32,
        -2.5,
        40.0,
        20.0,
        0.25,
        0.75,
        38.0 / 255.0,
        38.0 / 255.0,
        44.0 / 255.0,
        1.0,
    ] {
        expect.extend_from_slice(&v.to_le_bytes());
    }
    assert_eq!(got, expect.as_slice());
}

const SEED: u64 = 0x1234_5678;

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

#[test]
fn shader_uv_table_tracks_atlas_glyph_count() {
    let src = include_str!("../../../shaders/shader.wgsl");
    let arr = format!("array<vec4<f32>, {ATLAS_GLYPH_COUNT}>");
    assert!(
        src.contains(&arr),
        "shader.wgsl must declare the icon UV table as `{arr}`"
    );
    let clamp = format!("min(in.glyph, {}u)", ATLAS_GLYPH_COUNT - 1);
    assert!(
        src.contains(&clamp),
        "shader.wgsl must clamp the glyph index with `{clamp}`"
    );
}

#[test]
fn marker_glyph_mapping_is_distinct_and_folds_case() {
    assert_eq!(marker_glyph_for_alias("attack"), MarkerGlyph::TriangleUp);
    assert_eq!(marker_glyph_for_alias("defend"), MarkerGlyph::TriangleDown);
    assert_eq!(marker_glyph_for_alias("flag"), MarkerGlyph::Flag);

    let three = [
        marker_glyph_for_alias("attack") as u16,
        marker_glyph_for_alias("defend") as u16,
        marker_glyph_for_alias("flag") as u16,
    ];
    assert_eq!(
        three.iter().collect::<std::collections::HashSet<_>>().len(),
        3,
        "the three acceptance icons must map to three distinct glyphs"
    );

    assert_eq!(marker_glyph_for_alias("Waypoint"), MarkerGlyph::Chevron);
    assert_eq!(marker_glyph_for_alias("waypoint"), MarkerGlyph::Chevron);
    assert_eq!(marker_glyph_for_alias("PHASE-LINE"), MarkerGlyph::Chevron);
    assert_eq!(marker_glyph_for_alias("rally point"), MarkerGlyph::Flag);

    assert_eq!(marker_glyph_for_alias(""), MarkerGlyph::Disc);
    assert_eq!(marker_glyph_for_alias("not-a-real-icon"), MarkerGlyph::Disc);
    assert_eq!(marker_glyph_for_alias("dot"), MarkerGlyph::Disc);
}

#[test]
fn every_schema_alias_maps() {
    for a in ["dot", "dot2", "point", "mark", "marker"] {
        assert_eq!(marker_glyph_for_alias(a), MarkerGlyph::Disc, "{a}");
    }
    for a in ["objective_marker", "obj", "target", "task"] {
        assert_eq!(marker_glyph_for_alias(a), MarkerGlyph::Square, "{a}");
    }
    for a in ["observation_post", "op", "overwatch", "recon"] {
        assert_eq!(marker_glyph_for_alias(a), MarkerGlyph::Target, "{a}");
    }

    for a in [
        "dot",
        "dot2",
        "objective_marker",
        "objective_marker2",
        "point_of_interest",
        "point_of_interest2",
        "observation_post",
        "observation_post2",
        "destroy",
        "destroy2",
        "attack",
        "defend",
        "defend2",
        "waypoint",
        "waypoint2",
        "ambush",
        "ambush2",
        "flag",
        "flag2",
        "cross",
        "cross2",
        "circle",
        "circle2",
        "objective",
        "obj",
        "target",
        "task",
        "assault",
        "capture",
        "seize",
        "advance",
        "hold",
        "garrison",
        "fallback",
        "demolish",
        "demo",
        "sabotage",
        "move",
        "wp",
        "route",
        "phase_line",
        "poi",
        "intel",
        "contact",
        "op",
        "observe",
        "overwatch",
        "recon",
        "rally",
        "rally_point",
        "base",
        "hq",
        "spawn",
        "medical",
        "medic",
        "aid",
        "casevac",
        "medevac",
        "area",
        "zone",
        "ao",
        "point",
        "mark",
        "marker",
    ] {
        assert!(
            (marker_glyph_for_alias(a) as usize) < MARKER_GLYPH_COUNT,
            "{a}"
        );
    }
}

#[test]
fn marker_atlas_cells_0_and_1_match_slot_atlas() {
    let (rgba, w, h, uv) = build_marker_slot_atlas();
    assert_eq!(h, 64);
    assert_eq!(w, 64 * MARKER_GLYPH_COUNT as u32);
    assert_eq!(uv.len(), MARKER_GLYPH_COUNT * 4);

    assert_eq!(&uv[0..4], &[0.0, 0.0, 1.0 / MARKER_GLYPH_COUNT as f32, 1.0]);

    let slot = crate::symbology::atlas::raster::build_slot_atlas();

    let sw = slot.width as usize;
    let mw = w as usize;
    for y in 0..64usize {
        for x in 0..64usize {
            let s = (y * sw + x) * 4 + 3;
            let m = (y * mw + x) * 4 + 3;
            assert_eq!(rgba[m], slot.rgba[s], "ring alpha @({x},{y})");

            let s2 = (y * sw + 64 + x) * 4 + 3;
            let m2 = (y * mw + 64 + x) * 4 + 3;
            assert_eq!(rgba[m2], slot.rgba[s2], "disc alpha @({x},{y})");
        }
    }
}

#[test]
fn every_marker_glyph_cell_has_distinct_ink() {
    let (rgba, w, _h, _uv) = build_marker_slot_atlas();
    let mw = w as usize;
    let cell_alpha = |cell: usize| -> Vec<u8> {
        let mut out = Vec::with_capacity(64 * 64);
        for y in 0..64usize {
            for x in 0..64usize {
                out.push(rgba[(y * mw + cell * 64 + x) * 4 + 3]);
            }
        }
        out
    };
    let mut prints = Vec::new();
    for cell in 0..MARKER_GLYPH_COUNT {
        let a = cell_alpha(cell);
        let ink: u32 = a.iter().map(|&v| u32::from(v)).sum();
        assert!(ink > 0, "glyph cell {cell} is blank");
        prints.push(a);
    }

    assert_ne!(
        prints[4], prints[5],
        "attack vs defend footprints identical"
    );
    assert_ne!(prints[4], prints[8], "attack vs flag footprints identical");
    assert_ne!(prints[5], prints[8], "defend vs flag footprints identical");

    for a in 0..MARKER_GLYPH_COUNT {
        for b in (a + 1)..MARKER_GLYPH_COUNT {
            assert_ne!(
                prints[a], prints[b],
                "glyph cells {a} and {b} raster identically"
            );
        }
    }
}

fn glyph_ascii(g: u16) -> String {
    let mut out = String::with_capacity(33 * 32);
    for y in (0..64).step_by(2) {
        for x in (0..64).step_by(2) {
            let a = (marker_glyph_coverage(g, f64::from(x), f64::from(y))
                + marker_glyph_coverage(g, f64::from(x + 1), f64::from(y))
                + marker_glyph_coverage(g, f64::from(x), f64::from(y + 1))
                + marker_glyph_coverage(g, f64::from(x + 1), f64::from(y + 1)))
                / 4.0;
            #[allow(clippy::cast_possible_truncation)]
            out.push(match (a * 4.0).round() as i32 {
                0 => '.',
                1 => ':',
                2 => '+',
                3 => '#',
                _ => '@',
            });
        }
        out.push('\n');
    }
    out
}

fn glyph_ink_stats(g: u16) -> (f64, f64, f64, f64, f64) {
    let (mut sum, mut sx, mut sy, mut border) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
    let (mut x0, mut x1, mut y0, mut y1) = (i32::MAX, i32::MIN, i32::MAX, i32::MIN);
    for y in 0..64i32 {
        for x in 0..64i32 {
            let a = marker_glyph_coverage(g, f64::from(x), f64::from(y));

            if a > 1.0 / 255.0 {
                x0 = x0.min(x);
                x1 = x1.max(x);
                y0 = y0.min(y);
                y1 = y1.max(y);
                if x <= 1 || y <= 1 || x >= 62 || y >= 62 {
                    border += a;
                }
            }
            sum += a;
            sx += a * (f64::from(x) + 0.5);
            sy += a * (f64::from(y) + 0.5);
        }
    }
    assert!(sum > 0.0, "glyph {g} has no ink at all");
    (
        sx / sum - 32.0,
        sy / sum - 32.0,
        f64::from(x0 + x1) / 2.0 + 0.5 - 32.0,
        f64::from(y0 + y1) / 2.0 + 0.5 - 32.0,
        border,
    )
}

#[test]
fn marker_glyphs_are_centred_and_unclipped() {
    const TRIANGLES: [u16; 2] = [
        MarkerGlyph::TriangleUp as u16,
        MarkerGlyph::TriangleDown as u16,
    ];
    #[allow(clippy::cast_possible_truncation)]
    for g in 0..MARKER_GLYPH_COUNT as u16 {
        let (cdx, cdy, bdx, bdy, border) = glyph_ink_stats(g);
        assert!(
            border == 0.0,
            "T-808: glyph {g} puts {border:.2} units of ink in the 2 px border frame — it is \
                 clipped by its own atlas cell and bleeds into the neighbouring cell's UV:\n{}",
            glyph_ascii(g)
        );
        if TRIANGLES.contains(&g) {
            assert!(
                bdx.abs() <= 1.0 && bdy.abs() <= 1.0,
                "T-808: triangle {g} bounding box is off-anchor by ({bdx:+.2}, {bdy:+.2}) \
                     px:\n{}",
                glyph_ascii(g)
            );

            let want = if g == MarkerGlyph::TriangleUp as u16 {
                7.33
            } else {
                -7.33
            };
            assert!(
                cdx.abs() <= 0.5 && (cdy - want).abs() <= 0.5,
                "T-808: triangle {g} centroid ({cdx:+.2}, {cdy:+.2}) is not the intrinsic \
                     1/6-height offset ({want:+.2}) of the picker polygon:\n{}",
                glyph_ascii(g)
            );
        } else {
            let off = cdx.hypot(cdy);
            assert!(
                off <= 2.0,
                "T-808: glyph {g} ink centroid is {off:.2} px off the anchor \
                     ({cdx:+.2}, {cdy:+.2}) — a marker whose ink does not sit on the point it \
                     marks:\n{}",
                glyph_ascii(g)
            );
        }
    }
}

#[test]
fn marker_captions_pack_beside_their_marker() {
    let xy = [100.0_f32, 200.0, 300.0, 400.0];
    let caps = vec!["AB".to_string(), String::new()];
    let zoom = 0.0;
    let bytes = pack_marker_caption_bytes(&xy, &caps, zoom);

    assert_eq!(bytes.len(), 2 * 20);

    let gx0 = f32::from_le_bytes(bytes[0..4].try_into().unwrap());
    let gy0 = f32::from_le_bytes(bytes[4..8].try_into().unwrap());
    assert!(gx0 > 100.0, "caption starts right of the marker");
    assert!(
        (gy0 - 200.0).abs() < 1.0,
        "caption centred on the marker row"
    );

    assert!((gx0 - 100.0) < 40.0, "first glyph within the 40 px window");

    assert!(pack_marker_caption_bytes(&xy, &[String::new(), String::new()], zoom).is_empty());
    assert!(pack_marker_caption_bytes(&[], &[], zoom).is_empty());
}
