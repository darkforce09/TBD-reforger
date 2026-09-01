//! Tests for [`super`] — the per-level visibility rasters — on the synthetic two-level box room
//! from `building_blueprint_tests` (hand-built blueprint + COLL-style slab mesh with the real
//! holes), split out per the `#[path]` precedent.

use super::*;
use crate::building_blueprint::tests::{room_blueprint, room_sidecar, slab};
use crate::bvh::tests::Scene;

fn washes_for(obs: [f64; 3], extra: &[Scene]) -> Vec<LevelWash> {
    let bp = room_blueprint();
    let sc = room_sidecar(extra);
    level_washes(&bp, &sc, obs, &WashParams::default())
}

/// Ceiling between the two levels (y ∈ [2.95, 3.05]) with a 1 × 1 m stairwell hole at
/// x ∈ [1, 2], z ∈ [1, 2]. The room mesh has no floor/ceiling slabs of its own.
fn ceiling_with_stairwell() -> Vec<Scene> {
    vec![
        slab([0.0, 6.0], [2.95, 3.05], [0.0, 1.0]),
        slab([0.0, 6.0], [2.95, 3.05], [2.0, 6.0]),
        slab([0.0, 1.0], [2.95, 3.05], [1.0, 2.0]),
        slab([2.0, 6.0], [2.95, 3.05], [1.0, 2.0]),
    ]
}

/// `(visible, total)` over the cells strictly inside the room's footprint.
fn interior_lit(w: &LevelWash) -> (usize, usize) {
    let (mut vis, mut total) = (0usize, 0usize);
    for row in 0..w.rows {
        for col in 0..w.cols {
            let c = w.cell_center(col, row);
            if c[0] > 0.2 && c[0] < 5.8 && c[1] > 0.2 && c[1] < 5.8 {
                total += 1;
                if w.at(col, row) == Visibility::Visible {
                    vis += 1;
                }
            }
        }
    }
    (vis, total)
}

#[test]
fn grid_is_padded_bbox_at_eye_height_rows_north_first() {
    let w = washes_for([3.0, 1.4, 3.0], &[]);
    assert_eq!(w.len(), 2);
    for (i, lw) in w.iter().enumerate() {
        assert_eq!(lw.level_index, i);
        assert_eq!((lw.cols, lw.rows), (64, 64));
        assert_eq!(lw.cells.len(), 64 * 64);
        assert!((lw.min_x - -5.0).abs() < 1e-9 && (lw.min_z - -5.0).abs() < 1e-9);
        assert!((lw.max_x - 11.0).abs() < 1e-9 && (lw.max_z - 11.0).abs() < 1e-9);
        assert!((lw.cell_m - 0.25).abs() < 1e-12);
        assert_eq!(lw.obs, [3.0, 1.4, 3.0]);
    }
    assert!((w[0].eye_y - 1.0).abs() < 1e-9 && (w[1].eye_y - 4.0).abs() < 1e-9);
    // Row 0 is the NORTH edge, col 0 the WEST edge — the texture contract.
    let c00 = w[0].cell_center(0, 0);
    assert!(
        (c00[0] - -4.875).abs() < 1e-9 && (c00[1] - 10.875).abs() < 1e-9,
        "{c00:?}"
    );
    let c_last = w[0].cell_center(63, 63);
    assert!((c_last[0] - 10.875).abs() < 1e-9 && (c_last[1] - -4.875).abs() < 1e-9);
    // `cell_at` inverts `cell_center` for every cell; outside the rect is `None` / `Unknown`.
    for row in 0..64 {
        for col in 0..64 {
            let c = w[0].cell_center(col, row);
            assert_eq!(w[0].cell_at(c[0], c[1]), Some((col, row)));
        }
    }
    assert_eq!(w[0].cell_at(-5.1, 0.0), None);
    assert_eq!(w[0].visibility_at(-5.1, 0.0), Visibility::Unknown);
    assert_eq!(w[0].at(64, 0), Visibility::Unknown);
}

#[test]
fn ground_wash_reads_the_room() {
    let w = washes_for([3.0, 1.4, 3.0], &[]);
    let g = &w[0];
    assert_eq!(g.visibility_at(1.5, 1.5), Visibility::Visible, "open room");
    assert_eq!(g.visibility_at(3.0, 7.5), Visibility::Hidden, "north wall");
    assert_eq!(
        g.visibility_at(3.0, -3.0),
        Visibility::Visible,
        "through the window hole (ray crosses the wall at y ≈ 1.2)"
    );
    assert_eq!(
        g.visibility_at(1.0, -3.0),
        Visibility::Hidden,
        "south wall beside the hole"
    );
    assert_eq!(g.visibility_at(-1.0, 3.0), Visibility::Hidden, "west wall");
    let (v, h) = g.class_counts();
    assert!(v > 0 && h > 0 && v + h == g.cells.len());
}

#[test]
fn upstairs_wash_visible_only_through_stairwell() {
    let obs = [1.5, 1.4, 1.5];
    let with = washes_for(obs, &ceiling_with_stairwell());
    let without = washes_for(obs, &[]);
    let up = &with[1];
    assert_eq!(
        up.visibility_at(1.5, 1.5),
        Visibility::Visible,
        "straight up the stairwell"
    );
    assert_eq!(
        up.visibility_at(5.0, 5.0),
        Visibility::Hidden,
        "ceiling between"
    );
    // Only the stairwell cone lights the upstairs plane.
    let (vis, total) = interior_lit(up);
    assert!(vis > 0, "nothing lit through the stairwell");
    assert!(
        vis * 100 < total * 15,
        "{vis}/{total} lit — the ceiling is not occluding"
    );
    // The ceiling changes nothing on the ground floor (its rays never reach y = 3).
    assert_eq!(with[0].cells, without[0].cells);
    // Without a ceiling in the mesh the upstairs plane is wide open from below.
    let (vis, total) = interior_lit(&without[1]);
    assert!(vis * 100 > total * 90, "{vis}/{total} lit with no ceiling");
}

#[test]
fn observer_cell_and_open_air_are_visible() {
    let obs = [-4.0, 1.4, 3.0];
    let w = washes_for(obs, &[]);
    let g = &w[0];
    assert_eq!(
        g.visibility_at(-4.0, 3.0),
        Visibility::Visible,
        "the observer's own cell"
    );
    assert_eq!(g.visibility_at(-3.0, 3.0), Visibility::Visible, "open air");
    assert_eq!(
        g.visibility_at(3.0, 3.0),
        Visibility::Hidden,
        "the west wall between"
    );
    // Exactly on a cell centre at eye height: the zero-length segment is clear by definition.
    let c = g.cell_center(4, 60);
    let w2 = washes_for([c[0], 1.0, c[1]], &[]);
    assert_eq!(w2[0].at(4, 60), Visibility::Visible);
}

#[test]
fn oversized_footprint_coarsens_the_cell_to_the_cap() {
    let bb = BBox2D {
        min: [0.0, 0.0],
        max: [1000.0, 500.0],
        width_m: 1000.0,
        depth_m: 500.0,
    };
    let (min_x, min_z, cols, rows, cell) = grid_rect(&bb, WASH_CELL_M, WASH_PAD_M);
    assert!((min_x - -5.0).abs() < 1e-9 && (min_z - -5.0).abs() < 1e-9);
    assert!(
        cols <= MAX_WASH_DIM && rows <= MAX_WASH_DIM,
        "{cols} × {rows}"
    );
    assert_eq!(cols, MAX_WASH_DIM, "the long axis fills the cap exactly");
    assert!(cell > WASH_CELL_M, "cell {cell}");
    // The rect still covers the padded bbox.
    assert!(min_x + cols as f64 * cell >= 1005.0 - 1e-9);
    assert!(min_z + rows as f64 * cell >= 505.0 - 1e-9);
    // A small footprint keeps the requested cell.
    let small = room_blueprint().overall_footprint.bounding_box2_d;
    let (_, _, c, r, cell) = grid_rect(&small, WASH_CELL_M, WASH_PAD_M);
    assert_eq!((c, r), (64, 64));
    assert!((cell - WASH_CELL_M).abs() < 1e-12);
}

#[test]
fn no_levels_no_washes() {
    let mut bp = room_blueprint();
    bp.levels.clear();
    let sc = room_sidecar(&[]);
    assert!(level_washes(&bp, &sc, [3.0, 1.4, 3.0], &WashParams::default()).is_empty());
}

/// Assertion-free timing print for the report (`--nocapture`): two 64 × 64 levels = 8192 rays.
#[test]
fn wash_timing_envelope() {
    let bp = room_blueprint();
    let sc = room_sidecar(&ceiling_with_stairwell());
    let t0 = std::time::Instant::now();
    let w = level_washes(&bp, &sc, [3.0, 1.4, 3.0], &WashParams::default());
    let dt = t0.elapsed();
    let rays: usize = w.iter().map(|l| l.cells.len()).sum();
    eprintln!(
        "wash_timing_envelope: {rays} rays in {:.1} ms ({:.2} µs/ray)",
        dt.as_secs_f64() * 1e3,
        dt.as_secs_f64() * 1e6 / rays as f64
    );
}
