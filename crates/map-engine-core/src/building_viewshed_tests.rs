//! Tests for [`super`] — the per-level visibility rasters — on the synthetic two-level box room
//! from `building_blueprint_tests` (hand-built blueprint + COLL-style slab mesh with the real
//! holes), split out per the `#[path]` precedent.

use super::*;
use crate::building_blueprint::tests::{room_blueprint, room_sidecar, slab};
use crate::bvh::tests::Scene;

/// Test radius: an 8 m disc → a 64 × 64 grid at 0.25 m around the observer.
fn params() -> WashParams {
    WashParams {
        radius_m: 8.0,
        ..WashParams::default()
    }
}

fn washes_for(obs: [f64; 3], extra: &[Scene]) -> Vec<LevelWash> {
    let bp = room_blueprint();
    let sc = room_sidecar(extra);
    level_washes(&bp, &sc, obs, &params())
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
fn grid_is_an_observer_centred_square_rows_north_first() {
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
        assert!((lw.radius_m - 8.0).abs() < 1e-12);
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
fn cells_outside_the_radius_are_unknown() {
    let w = washes_for([3.0, 1.4, 3.0], &[]);
    let g = &w[0];
    // The square's corners lie 11.3 m out: no ray, `Unknown`.
    assert_eq!(g.at(0, 0), Visibility::Unknown);
    assert_eq!(g.at(63, 63), Visibility::Unknown);
    // Just inside the disc along an axis: judged.
    assert_ne!(g.visibility_at(10.5, 3.0), Visibility::Unknown);
    // Inside the square but outside the disc (diagonal): not.
    assert_eq!(g.visibility_at(9.5, 9.5), Visibility::Unknown);
    let (v, h, u) = g.class_counts();
    assert_eq!(v + h + u, g.cells.len());
    // A disc in a square: ~21 % of the cells are outside it.
    assert!(
        u * 100 > g.cells.len() * 15 && u * 100 < g.cells.len() * 30,
        "unknown {u}"
    );
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
    let (v, h, _) = g.class_counts();
    assert!(v > 0 && h > 0);
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
    let (vis, total) = interior_lit(up);
    assert!(vis > 0, "nothing lit through the stairwell");
    assert!(
        vis * 100 < total * 15,
        "{vis}/{total} lit — the ceiling is not occluding"
    );
    assert_eq!(with[0].cells, without[0].cells);
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
    let (col, row) = g.cell_at(-4.0, 3.0).expect("observer cell");
    let c = g.cell_center(col, row);
    let w2 = washes_for([c[0], 1.0, c[1]], &[]);
    assert_eq!(w2[0].visibility_at(c[0], c[1]), Visibility::Visible);
}

#[test]
fn oversized_radius_coarsens_the_cell_to_the_cap() {
    let (min_x, min_z, n, cell) = grid_rect([0.0, 0.0], 1000.0, WASH_CELL_M);
    assert!((min_x - -1000.0).abs() < 1e-9 && (min_z - -1000.0).abs() < 1e-9);
    assert_eq!(n, MAX_WASH_DIM, "the side fills the cap exactly");
    assert!(cell > WASH_CELL_M, "cell {cell}");
    assert!(min_x + n as f64 * cell >= 1000.0 - 1e-9);
    let (_, _, n, cell) = grid_rect([3.0, 3.0], 8.0, WASH_CELL_M);
    assert_eq!(n, 64);
    assert!((cell - WASH_CELL_M).abs() < 1e-12);
}

#[test]
fn level_wash_picks_one_level() {
    let bp = room_blueprint();
    let sc = room_sidecar(&[]);
    let obs = [3.0, 1.4, 3.0];
    let up = level_wash(&bp, &sc, obs, 1, &params()).expect("level 1");
    assert_eq!(up.level_index, 1);
    assert!((up.eye_y - 4.0).abs() < 1e-9);
    assert!(level_wash(&bp, &sc, obs, 7, &params()).is_none());
    let mut none = room_blueprint();
    none.levels.clear();
    assert!(level_washes(&none, &sc, obs, &params()).is_empty());
}

/// Assertion-free timing print for the report (`--nocapture`): two 64 × 64 discs.
#[test]
fn wash_timing_envelope() {
    let bp = room_blueprint();
    let sc = room_sidecar(&ceiling_with_stairwell());
    let t0 = std::time::Instant::now();
    let w = level_washes(&bp, &sc, [3.0, 1.4, 3.0], &params());
    let dt = t0.elapsed();
    let rays: usize = w
        .iter()
        .map(|l| {
            let (v, h, _) = l.class_counts();
            v + h
        })
        .sum();
    eprintln!(
        "wash_timing_envelope: {rays} rays in {:.1} ms ({:.2} µs/ray)",
        dt.as_secs_f64() * 1e3,
        dt.as_secs_f64() * 1e6 / rays.max(1) as f64
    );
}
