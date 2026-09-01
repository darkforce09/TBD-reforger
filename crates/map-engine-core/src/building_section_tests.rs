//! Tests for [`super`] — section cuts of vertical faces, the clipped heightfield, void
//! see-through — on the synthetic two-level box room from `building_blueprint_tests`, split
//! out per the `#[path]` precedent.

use super::*;
use crate::building_blueprint::tests::{room_blueprint, room_sidecar, slab};
use crate::bvh::tests::Scene;

/// Does some segment lie on the line `z == z_line` (both ends) and span `x`?
fn covers_x_on(segs: &[Seg2], z_line: f64, x: f64) -> bool {
    segs.iter().any(|s| {
        (s[0][1] - z_line).abs() < 1e-6
            && (s[1][1] - z_line).abs() < 1e-6
            && s[0][0].min(s[1][0]) <= x
            && s[0][0].max(s[1][0]) >= x
    })
}

/// Ceiling between the two levels (y ∈ [2.95, 3.05]) with a 1 × 1 m stairwell hole at
/// x ∈ [1, 2], z ∈ [1, 2].
fn ceiling_with_stairwell() -> Vec<Scene> {
    vec![
        slab([0.0, 6.0], [2.95, 3.05], [0.0, 1.0]),
        slab([0.0, 6.0], [2.95, 3.05], [2.0, 6.0]),
        slab([0.0, 1.0], [2.95, 3.05], [1.0, 2.0]),
        slab([2.0, 6.0], [2.95, 3.05], [1.0, 2.0]),
    ]
}

/// Four solid treads rising 0.2 m per 0.25 m along x ∈ [4, 5], z ∈ [1, 2].
fn treads() -> Vec<Scene> {
    (0..4)
        .map(|k| {
            let x0 = 4.0 + 0.25 * f64::from(k);
            slab([x0, x0 + 0.25], [0.0, 0.2 + 0.2 * f64::from(k)], [1.0, 2.0])
        })
        .collect()
}

/// A single sloped triangle `y = z` over x ∈ [0, 6], z ∈ [0, 6] (|n.y| ≈ 0.71).
fn sloped() -> Scene {
    (
        vec![[0.0, 0.0, 0.0], [6.0, 0.0, 0.0], [0.0, 6.0, 6.0]],
        vec![[0, 1, 2]],
    )
}

#[test]
fn section_cut_outlines_walls_and_opens_the_window() {
    let sc = room_sidecar(&[]);
    let main = section_at(&sc, 1.5, CUT_MAX_NY);
    assert!(covers_x_on(&main, -0.1, 1.0), "outer face west of the hole");
    assert!(covers_x_on(&main, 0.1, 1.0), "inner face west of the hole");
    assert!(covers_x_on(&main, -0.1, 5.0), "outer face east of the hole");
    assert!(
        !covers_x_on(&main, -0.1, 3.0) && !covers_x_on(&main, 0.1, 3.0),
        "the window hole must open the section"
    );
    assert!(
        main.iter()
            .any(|s| (s[0][0] - 2.5).abs() < 1e-6 && (s[1][0] - 2.5).abs() < 1e-6),
        "jamb at x = 2.5"
    );
    let low = section_at(&sc, 0.5, CUT_MAX_NY);
    assert!(covers_x_on(&low, -0.1, 3.0) && covers_x_on(&low, 0.1, 3.0));
    assert!(section_at(&sc, 9.0, CUT_MAX_NY).is_empty());
}

/// Roof pitches and treads must not draw lines: only near-vertical faces are cut.
#[test]
fn sloped_faces_are_not_cut() {
    let sc = room_sidecar(&[sloped()]);
    let on_slope = |segs: &[Seg2]| {
        segs.iter().any(|s| {
            (s[0][1] - 3.0).abs() < 1e-9
                && (s[1][1] - 3.0).abs() < 1e-9
                && s[0][0].min(s[1][0]) > -0.01
                && s[0][0].max(s[1][0]) < 3.01
        })
    };
    assert!(
        !on_slope(&section_at(&sc, 3.0, CUT_MAX_NY)),
        "slope cut leaked"
    );
    assert!(
        on_slope(&section_at(&sc, 3.0, 1.0)),
        "unfiltered cut sees the slope"
    );
}

#[test]
fn heightfield_clips_at_the_plane_and_sees_the_stairwell() {
    let bp = room_blueprint();
    let sc = room_sidecar(&ceiling_with_stairwell());
    let d = building_drawing(&bp, &sc);
    assert_eq!(d.levels.len(), 2);
    let up = &d.levels[1];
    let top = up
        .surface
        .value_at(5.0, 5.0)
        .expect("ceiling slab under the upper floor");
    assert!(
        (top - 3.05).abs() < 1e-9,
        "slab top, not the wall tops above the clip: {top}"
    );
    assert_eq!(up.surface.value_at(1.5, 1.5), None, "the stairwell hole");
    assert_eq!(up.surface.value_at(-3.0, -3.0), None, "outside the room");
    assert_eq!(up.surface.value_at(-50.0, 0.0), None, "outside the raster");
    assert!(up.surface.covered(5.0, 5.0, up.floor_min_y()));
    assert!(!up.surface.covered(1.5, 1.5, up.floor_min_y()));
    // Nothing above a level's cut plane survives in its field.
    for l in &d.levels {
        assert!(
            l.surface
                .h
                .iter()
                .flatten()
                .all(|&y| y <= l.cut_main_y + 1e-9)
        );
    }
    // The ground floor has no slab: only the wall footprints at y = 0 are surfaces.
    let ground = &d.levels[0];
    assert_eq!(ground.surface.value_at(3.0, 3.0), None);
    assert!(
        ground
            .surface
            .value_at(3.0, 0.0)
            .is_some_and(|y| (y - 1.0).abs() < 1e-9),
        "the sill top under the window is a ground-floor surface"
    );
}

#[test]
fn stairs_read_as_rising_heights() {
    let bp = room_blueprint();
    let sc = room_sidecar(&treads());
    let d = building_drawing(&bp, &sc);
    let g = &d.levels[0].surface;
    let heights: Vec<f64> = [4.1, 4.35, 4.6, 4.85]
        .iter()
        .map(|&x| g.value_at(x, 1.5).expect("tread surface"))
        .collect();
    for (k, h) in heights.iter().enumerate() {
        assert!((h - (0.2 + 0.2 * k as f64)).abs() < 1e-9, "tread {k}: {h}");
    }
    assert!(heights.windows(2).all(|w| w[1] > w[0]));
    // The cut plane at 1.2 m is above every tread: no riser lines from the stairs.
    let stair_lines = d.levels[0]
        .cut_main
        .iter()
        .filter(|s| s[0][0] > 3.9 && s[0][0] < 5.1 && s[0][1] > 0.9 && s[0][1] < 2.1)
        .count();
    assert_eq!(stair_lines, 0);
}

#[test]
fn through_voids_keeps_only_uncovered_pieces() {
    // A floor at y = 0 over x < 3 of [0, 6]².
    let mut hf = HeightField::empty([0.0, 0.0], [6.0, 6.0], 0.25);
    for row in 0..hf.rows {
        for col in 0..hf.cols {
            if hf.cell_center(col, row)[0] < 3.0 {
                hf.h[row * hf.cols + col] = Some(0.0);
            }
        }
    }
    let pieces = through_voids(&[[[0.0, 0.5], [6.0, 0.5]]], &hf, -0.25, 0.25);
    assert!(!pieces.is_empty());
    let total: f64 = pieces
        .iter()
        .map(|p| (p[1][0] - p[0][0]).hypot(p[1][1] - p[0][1]))
        .sum();
    assert!((2.75..=3.25).contains(&total), "kept length {total}");
    for p in &pieces {
        assert!(
            0.5 * (p[0][0] + p[1][0]) >= 3.0 - 1e-9,
            "piece under floor: {p:?}"
        );
    }
    // A surface deep below the floor window does not cover.
    let deep = through_voids(&[[[0.0, 0.5], [2.0, 0.5]]], &hf, 1.0, 0.25);
    assert_eq!(deep.len(), 8);
    // Entirely outside the raster: entirely visible.
    assert_eq!(
        through_voids(&[[[10.0, 10.0], [11.0, 10.0]]], &hf, -0.25, 0.25).len(),
        4
    );
}

#[test]
fn roof_field_is_the_top_surface() {
    let bp = room_blueprint();
    let sc = room_sidecar(&[]);
    let d = building_drawing(&bp, &sc);
    let top = d.roof.value_at(3.0, 3.0).expect("roof slab over the room");
    assert!(
        (top - 6.2).abs() < 1e-9,
        "slab top is the highest surface: {top}"
    );
    assert!((d.roof_y[1] - 6.2).abs() < 1e-9, "roof_y {:?}", d.roof_y);
    assert_eq!(
        d.roof.value_at(-3.0, -3.0),
        None,
        "no surface beside the building"
    );
    assert!(d.roof.range().is_some());
    // The raster spans the mesh (the slab overhangs the footprint by 0.1) plus the pad.
    assert!(d.roof.min_x <= -1.1 + 1e-9 && d.roof.min_z <= -1.1 + 1e-9);
}

#[test]
fn drawing_levels_and_clamps() {
    let mut bp = room_blueprint();
    let sc = room_sidecar(&[]);
    let d = building_drawing(&bp, &sc);
    assert_eq!(d.levels.len(), 2);
    for (i, l) in d.levels.iter().enumerate() {
        assert_eq!(l.level_index, i);
        let [lo, hi] = bp.levels[i].elevation_range;
        assert!((l.lo - lo).abs() < 1e-9 && (l.hi - hi).abs() < 1e-9);
        assert!((l.cut_main_y - (lo + CUT_MAIN_M)).abs() < 1e-9);
        assert!((l.cut_low_y - (lo + CUT_LOW_M)).abs() < 1e-9);
        assert!(!l.cut_main.is_empty() && !l.cut_low.is_empty());
    }
    bp.levels[0].elevation_range = [0.0, 1.0];
    let d = building_drawing(&bp, &sc);
    assert!((d.levels[0].cut_main_y - 0.6).abs() < 1e-9);
    assert!((d.levels[0].cut_low_y - 0.25).abs() < 1e-9);
    bp.levels.clear();
    let d = building_drawing(&bp, &sc);
    assert!(d.levels.is_empty() && d.roof.range().is_some());
}

/// Any mesh, no blueprint: one band spanning the mesh gives a full drawing.
#[test]
fn drawing_for_without_a_blueprint() {
    let sc = room_sidecar(&[]);
    let (lo, hi) = mesh_bounds(&sc).expect("bounds");
    let specs = [LevelSpec {
        index: 0,
        lo: -0.5,
        hi: 7.0,
    }];
    let d = drawing_for(
        &sc,
        &specs,
        [lo[0] - 1.0, lo[1] - 1.0],
        [hi[0] + 1.0, hi[1] + 1.0],
    );
    assert_eq!(d.levels.len(), 1);
    assert!((d.levels[0].cut_main_y - 0.7).abs() < 1e-9, "−0.5 + 1.2");
    assert!(!d.levels[0].cut_main.is_empty());
    assert!(d.levels[0].surface.range().is_some());
    assert!((d.roof_y[1] - 6.2).abs() < 1e-9);
}
