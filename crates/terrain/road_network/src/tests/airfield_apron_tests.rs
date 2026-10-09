//! The airfield box, the structure class gate and the apron over synthetic vector grids.
//!
//! **Role:** pins the behaviour of `airfield.rs` named in each case.
//! **Position:** mounted by `airfield.rs` under `cfg(test)`.
//! **Signals & state:** none.
//! **Invariants:** every case states the exact value or event it expects.

use crate::airfield::*;
use crate::network::RoadSegment;
use terrain_elevation::grid::DemVectorGrid;

fn flat_grid(elev: f32, cols: usize, rows: usize, cell: f64) -> DemVectorGrid {
    DemVectorGrid {
        data: vec![elev; cols * rows],
        cols,
        rows,
        cell_x: cell,
        cell_y: cell,
        origin_x: 0.0,
        origin_y: 0.0,
        max_elev_m: f64::from(elev),
    }
}

#[test]
fn flat_pad_inside_bbox_produces_apron() {
    let grid = flat_grid(100.0, 20, 20, 50.0);
    let bbox = [100.0, 100.0, 800.0, 800.0];
    let mesh = build_airfield_apron_mesh(&grid, bbox);
    assert!(!mesh.indices.is_empty());
    assert!(mesh.polygon_count > 0);
}

#[test]
fn rough_terrain_outside_flat_gate_is_empty() {
    let mut grid = flat_grid(100.0, 10, 10, 100.0);
    for (i, v) in grid.data.iter_mut().enumerate() {
        *v = if i % 2 == 0 { 50.0 } else { 150.0 };
    }
    grid.max_elev_m = 150.0;
    let bbox = [0.0, 0.0, 1000.0, 1000.0];
    let mesh = build_airfield_apron_mesh(&grid, bbox);
    assert!(mesh.indices.is_empty());
}

fn seg(id: &str, pts: &[[f64; 2]]) -> RoadSegment {
    RoadSegment {
        id: id.into(),
        road_class: "runway".to_string(),
        points: pts.to_vec(),
        width_m: 20.0,
    }
}

#[test]
fn bbox_expands_runway_union_by_margin() {
    let runways = vec![seg("r0", &[[100.0, 200.0], [300.0, 400.0]])];
    let b = compute_airfield_bbox(&runways).unwrap();
    assert_eq!(b[0], 70.0);
    assert_eq!(b[1], 170.0);
    assert_eq!(b[2], 330.0);
    assert_eq!(b[3], 430.0);
}

#[test]
fn point_in_bbox_edges() {
    let b = [0.0, 0.0, 100.0, 100.0];
    assert!(point_in_bbox(0.0, 0.0, b));
    assert!(point_in_bbox(100.0, 100.0, b));
    assert!(!point_in_bbox(-0.1, 50.0, b));
}

#[test]
fn airfield_structure_classes() {
    assert!(is_airfield_structure_class("hangar"));
    assert!(is_airfield_structure_class("tower"));
    assert!(!is_airfield_structure_class("military"));
}
