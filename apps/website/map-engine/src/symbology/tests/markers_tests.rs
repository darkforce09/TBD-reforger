//! Role: markers tests.
//! Position: `symbology/tests` in the map engine.
//! Signals & state: the marker vocabulary, its atlas raster, and caption placement.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::symbology::markers::*;

// T-0xx Phase 1D: these six cases moved here with their subjects when
// `renderers/batching/scene.rs` split. Every one of them names a schema alias, an ORBAT slot
// cell, or a caption — none of it is anything the renderer is allowed to know.

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
