//! Role: triangulate tests.
//! Position: `renderers/primitives/tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::draw::triangulate::*;

fn assert_area_conserved(expected: f64, mesh: &TriMesh) {
    let ta = triangle_area_sum(mesh);
    let tol = area_tolerance(expected);
    assert!(
        (expected - ta).abs() <= tol,
        "area conservation failed: expected={expected} tri={ta} tol={tol} |Δ|={}",
        (expected - ta).abs()
    );
}

#[test]
fn unit_square_area_and_two_tris() {
    let ring = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    let mesh = triangulate_simple(&ring);
    assert_eq!(mesh.indices.len(), 6);
    assert_area_conserved(1.0, &mesh);
}

#[test]
fn closed_ring_stripped() {
    let ring = [[0.0, 0.0], [2.0, 0.0], [2.0, 3.0], [0.0, 3.0], [0.0, 0.0]];
    let mesh = triangulate_simple(&ring);
    assert_area_conserved(6.0, &mesh);
}

#[test]
fn ccw_triangle() {
    let ring = [[0.0, 0.0], [4.0, 0.0], [0.0, 3.0]];
    let mesh = triangulate_simple(&ring);
    assert_eq!(mesh.indices.len(), 3);
    assert_area_conserved(6.0, &mesh);
}

#[test]
fn polygon_with_hole_area_conserved() {
    let outer = [[0.0, 0.0], [4.0, 0.0], [4.0, 4.0], [0.0, 4.0], [0.0, 0.0]];
    let hole = vec![[1.0, 1.0], [1.0, 2.0], [2.0, 2.0], [2.0, 1.0], [1.0, 1.0]];
    let mesh = triangulate_with_holes(&outer, std::slice::from_ref(&hole));
    let expected = ring_area(&outer) - ring_area(&hole);
    assert!((expected - 15.0).abs() < 1e-12);
    assert_area_conserved(expected, &mesh);
    assert!(!mesh.indices.is_empty());
}

#[test]
fn ring_buffer_sea_style_rects() {
    let positions: Vec<f32> = vec![
        0.0, 0.0, 2.0, 0.0, 2.0, 1.0, 0.0, 1.0, 0.0, 0.0, 3.0, 0.0, 5.0, 0.0, 5.0, 2.0, 3.0, 2.0,
        3.0, 0.0,
    ];
    let starts = [0_u32, 5];
    let colors = [72_u8, 118, 160, 255].repeat(10);
    let (mesh, cols) = triangulate_ring_buffer(&positions, &starts, Some(&colors));
    assert_eq!(mesh.indices.len() / 3, 4);
    assert_eq!(cols.len(), mesh.positions.len() / 2 * 4);
    assert_area_conserved(6.0, &mesh);
}
