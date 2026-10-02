use super::*;
use crate::blueprint::{slabs, synth};

fn vertical(d: &VoxelDump) -> VerticalScan {
    let m = d.meta();
    let p = Params {
        min_floor_y: -0.5 - m.origin[1],
        ..Default::default()
    };
    slabs::analyze(&d.y_down, m.dims, m.cell, m.span[1], &p)
}

fn band(d: &VoxelDump, v: &VerticalScan, algo: Algo) -> BandWalls {
    let p = Params::default();
    let lo = v.floors[0];
    let hi = v.eave.max(lo + p.top_band_min_m);
    extract_band(d, v, lo, hi, algo, &p, None)
}

#[test]
fn box_room_yields_four_walls_both_algos() {
    let d = synth::box_room(6.0, 4.0, 2.6, 0.15);
    let v = vertical(&d);
    for algo in [Algo::Segments, Algo::Grid] {
        let bw = band(&d, &v, algo);
        assert_eq!(bw.walls.len(), 4, "{algo:?}: {:?}", bw.walls);
        assert!(bw.masses.is_empty(), "{algo:?} masses: {:?}", bw.masses);
        for w in &bw.walls {
            assert!(
                w.thickness > 0.05 && w.thickness < 0.35,
                "{algo:?} thickness {w:?}"
            );
        }
    }
}

#[test]
fn segments_box_walls_are_centerline_accurate() {
    let d = synth::box_room(6.0, 4.0, 2.6, 0.15);
    let v = vertical(&d);
    let bw = band(&d, &v, Algo::Segments);
    // West wall local x in [0, 0.15] → normalized center 0.6 + 0.075.
    let west = bw
        .walls
        .iter()
        .find(|w| (w.start[0] - w.end[0]).abs() < 1e-9 && w.start[0] < 1.0)
        .expect("west wall");
    assert!(
        (west.start[0] - 0.675).abs() <= 0.05,
        "centerline {}",
        west.start[0]
    );
    let all_ext = bw.exterior.iter().all(|e| *e);
    assert!(all_ext, "single box: every wall exterior");
}

#[test]
fn doorway_splits_wall_and_does_not_bridge() {
    let d = synth::box_with_door(6.0, 4.0, 2.6, 0.15, 2.4, 0.9);
    let v = vertical(&d);
    let bw = band(&d, &v, Algo::Segments);
    // The south wall (z-const at z≈0.675) must appear as exactly 2 runs with a ~0.9 m gap.
    let south: Vec<_> = bw
        .walls
        .iter()
        .filter(|w| (w.start[1] - w.end[1]).abs() < 1e-9 && w.start[1] < 1.0)
        .collect();
    assert_eq!(south.len(), 2, "south wall runs: {:?}", bw.walls);
    let mut xs: Vec<(f64, f64)> = south
        .iter()
        .map(|w| (w.start[0].min(w.end[0]), w.start[0].max(w.end[0])))
        .collect();
    xs.sort_by(|a, b| a.0.total_cmp(&b.0));
    let gap = xs[1].0 - xs[0].1;
    assert!((gap - 0.9).abs() <= 0.25, "doorway gap {gap}");
}

#[test]
fn gable_second_band_emits_gable_ends_and_zero_roof_phantoms() {
    let d = synth::gable_box(6.0, 4.0, 2.6, 4.2, 0.15);
    let v = vertical(&d);
    let p = Params::default();
    // Second band: eave slab does not exist — synthesize the attic band [eave, ridge].
    let bw = extract_band(&d, &v, v.eave, v.ridge, Algo::Segments, &p, None);
    let z_running: Vec<_> = bw
        .walls
        .iter()
        .filter(|w| (w.start[0] - w.end[0]).abs() < 1e-9)
        .collect();
    let x_running: Vec<_> = bw
        .walls
        .iter()
        .filter(|w| (w.start[1] - w.end[1]).abs() < 1e-9)
        .collect();
    assert_eq!(z_running.len(), 2, "gable ends: {:?}", bw.walls);
    assert!(
        x_running.is_empty(),
        "sloped roof planes must not become walls: {x_running:?}"
    );
}

#[test]
fn steep_graze_grid_phantoms_segments_clean() {
    // The regression pair that justifies the default: a steep roof graze drifts too little
    // for the live two-height AND to catch (same 0.1 m cell at both probe heights) but far
    // too much for the stationarity veto across the segments slice window. The graze sits at
    // x ≈ 3.0-3.2 normalized — well clear of the real walls at 0.675 and 6.525 — so the
    // assertion isolates the phantom region while both algos keep the real geometry.
    let d = synth::steep_graze();
    let v = vertical(&d);
    let p = Params::default();
    assert_eq!(v.floors.len(), 2, "two-story synth: {:?}", v.floors);
    let lo = v.floors[1];
    let hi = v.eave.max(lo + p.top_band_min_m);
    let in_phantom_zone =
        |w: &WallSeg| (w.start[0] - w.end[0]).abs() < 1e-9 && w.start[0] > 2.5 && w.start[0] < 3.7;
    let grid = extract_band(&d, &v, lo, hi, Algo::Grid, &p, None);
    let seg = extract_band(&d, &v, lo, hi, Algo::Segments, &p, None);
    assert!(
        grid.walls.iter().any(in_phantom_zone),
        "the graze must fool the live AND (else this regression tests nothing): {:?}",
        grid.walls
    );
    assert!(
        !seg.walls.iter().any(in_phantom_zone),
        "segments must reject the graze: {:?}",
        seg.walls
    );
}

/// The phantom counter-guard for the per-column denominator: a column whose roof-clipped
/// window is tiny cannot be captured by 1–2 stationary noise observations —
/// `min_persist_rows` (3) floors the requirement.
#[test]
fn sparse_noise_fails_min_persist_rows_floor() {
    let p = Params::default();
    let avail = |_c: f64, _k: usize| 3usize; // heavily roof-clipped column
    let mk = |rows: &[usize]| -> Vec<Obs> {
        rows.iter()
            .map(|&row| Obs {
                row,
                center: 1.0,
                thick: 0.1,
            })
            .collect()
    };
    // Two stationary observations: need = max(3, ceil(3·0.6) = 2) = 3 → rejected.
    let mut obs = mk(&[0, 1]);
    let cols = cluster_columns(&mut obs, 5, 16, &avail, "z-running", &p, None);
    assert!(cols.is_empty(), "2 rows must fail the absolute floor");
    // Three observations → accepted.
    let mut obs = mk(&[0, 1, 2]);
    let cols = cluster_columns(&mut obs, 5, 16, &avail, "z-running", &p, None);
    assert_eq!(cols.len(), 1);
}
