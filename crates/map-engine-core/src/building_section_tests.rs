//! Tests for [`super`] — section cuts, floor / roof faces, coverage and void see-through — on
//! the synthetic two-level box room from `building_blueprint_tests`, split out per the
//! `#[path]` precedent.

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

#[test]
fn section_cut_outlines_walls_and_opens_the_window() {
    let sc = room_sidecar(&[]);
    // Eye height: the south wall reads as its two faces, with the window hole open.
    let main = section_at(&sc, 1.5);
    assert!(covers_x_on(&main, -0.1, 1.0), "outer face west of the hole");
    assert!(covers_x_on(&main, 0.1, 1.0), "inner face west of the hole");
    assert!(covers_x_on(&main, -0.1, 5.0), "outer face east of the hole");
    assert!(
        !covers_x_on(&main, -0.1, 3.0) && !covers_x_on(&main, 0.1, 3.0),
        "the window hole must open the section"
    );
    // The hole's jambs appear as short cross-wall segments at x = 2.5 / 3.5.
    assert!(
        main.iter()
            .any(|s| (s[0][0] - 2.5).abs() < 1e-6 && (s[1][0] - 2.5).abs() < 1e-6),
        "jamb at x = 2.5"
    );
    // Below the sill the wall is continuous.
    let low = section_at(&sc, 0.5);
    assert!(covers_x_on(&low, -0.1, 3.0) && covers_x_on(&low, 0.1, 3.0));
    // Nothing above the roof slab; nothing at a plane no triangle crosses.
    assert!(section_at(&sc, 9.0).is_empty());
}

#[test]
fn floor_faces_and_coverage_see_the_stairwell() {
    let bp = room_blueprint();
    let sc = room_sidecar(&ceiling_with_stairwell());
    let d = building_drawing(&bp, &sc);
    assert_eq!(d.levels.len(), 2);
    let up = &d.levels[1];
    assert!(!up.floor.is_empty(), "the ceiling slab is the upper floor");
    assert!(up.floor.iter().all(|f| (f.y - 3.0).abs() < 0.1));
    assert!(up.coverage.covered(5.0, 5.0), "solid floor");
    assert!(!up.coverage.covered(1.5, 1.5), "the stairwell hole");
    assert!(!up.coverage.covered(-3.0, -3.0), "outside the room");
    assert!(!up.coverage.covered(-50.0, 0.0), "outside the raster");
    // The ground floor has no slab in this mesh: its only "floor" faces are the wall
    // footprints, so the room's interior is uncovered.
    let ground = &d.levels[0];
    assert!(ground.floor.iter().all(|f| f.y.abs() < 1e-9));
    assert!(!ground.coverage.covered(3.0, 3.0));
    assert!(up.coverage.covered_count() > ground.coverage.covered_count());
}

#[test]
fn through_voids_keeps_only_uncovered_pieces() {
    // Cover x < 3 over [0, 6]².
    let mut cover = CoverageGrid::empty([0.0, 0.0], [6.0, 6.0], 0.25);
    for row in 0..cover.rows {
        for col in 0..cover.cols {
            if cover.cell_center(col, row)[0] < 3.0 {
                cover.cells[row * cover.cols + col] = true;
            }
        }
    }
    let pieces = through_voids(&[[[0.0, 0.5], [6.0, 0.5]]], &cover, 0.25);
    assert!(!pieces.is_empty());
    let total: f64 = pieces
        .iter()
        .map(|p| (p[1][0] - p[0][0]).hypot(p[1][1] - p[0][1]))
        .sum();
    assert!((2.75..=3.25).contains(&total), "kept length {total}");
    for p in &pieces {
        let mid_x = 0.5 * (p[0][0] + p[1][0]);
        assert!(mid_x >= 3.0 - 1e-9, "piece under covered floor: {p:?}");
    }
    // A segment entirely outside the raster is entirely visible.
    let outside = through_voids(&[[[10.0, 10.0], [11.0, 10.0]]], &cover, 0.25);
    assert_eq!(outside.len(), 4);
}

#[test]
fn roof_faces_above_the_top_level() {
    let bp = room_blueprint();
    let sc = room_sidecar(&[]);
    let d = building_drawing(&bp, &sc);
    assert!(!d.roof.is_empty());
    // Every roof face sits above the top level's base + 0.6; the slab top is the highest.
    assert!(d.roof.iter().all(|f| f.y > 3.6));
    assert!((d.roof_y[1] - 6.2).abs() < 1e-9, "roof_y {:?}", d.roof_y);
    assert!(d.roof_y[0] >= 3.6);
    let top: Vec<&FaceFill> = d.roof.iter().filter(|f| (f.y - 6.2).abs() < 1e-9).collect();
    assert_eq!(top.len(), 2, "the roof slab's top face is two triangles");
    // A roofless mesh has an empty roof with a zero range.
    let mut bare = room_blueprint();
    bare.levels.truncate(1);
    bare.levels[0].elevation_range = [10.0, 20.0];
    let d = building_drawing(&bare, &sc);
    assert!(d.roof.is_empty() && d.roof_y == [0.0, 0.0]);
}

#[test]
fn drawing_has_one_entry_per_level_with_clamped_cuts() {
    let mut bp = room_blueprint();
    let sc = room_sidecar(&[]);
    let d = building_drawing(&bp, &sc);
    assert_eq!(d.levels.len(), 2);
    for (i, l) in d.levels.iter().enumerate() {
        assert_eq!(l.level_index, i);
        let lo = bp.levels[i].elevation_range[0];
        assert!((l.lo - lo).abs() < 1e-9);
        assert!((l.cut_main_y - (lo + CUT_MAIN_M)).abs() < 1e-9);
        assert!((l.cut_low_y - (lo + CUT_LOW_M)).abs() < 1e-9);
        assert!(!l.cut_main.is_empty() && !l.cut_low.is_empty());
    }
    // A short band clamps both cuts inside it.
    bp.levels[0].elevation_range = [0.0, 1.0];
    let d = building_drawing(&bp, &sc);
    assert!((d.levels[0].cut_main_y - 0.6).abs() < 1e-9);
    assert!((d.levels[0].cut_low_y - 0.25).abs() < 1e-9);
    // No levels → no level drawings, roof still judged from base 0.
    bp.levels.clear();
    let d = building_drawing(&bp, &sc);
    assert!(d.levels.is_empty());
    assert!(!d.roof.is_empty());
}
