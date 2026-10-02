//! Role: mass tests.
//! Position: `world/environment/vegetation/tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::world::environment::vegetation::mass::*;

#[test]
fn all_below_iso_is_empty() {
    let corners = vec![0u16; 9];
    let g = forest_mass_from_corners(&corners, 3, 3, 0.0, 0.0, 32.0, DENSITY_ISO);
    assert!(g.fill_positions.is_empty());
    assert!(g.fill_start_indices.is_empty());
    assert!(g.outline_segments.is_empty());
}

#[test]
fn outline_only_emits_segments_without_fill() {
    let mut corners = vec![0u16; 4];
    corners[0] = 5;
    let segs = forest_outline_segments_from_corners(&corners, 2, 2, 0.0, 0.0, 8.0, CANOPY_MASS_ISO);
    assert!(!segs.is_empty());
    assert_eq!(segs.len() % 4, 0);
}

#[test]
fn full_cell_is_a_closed_quad_with_no_contour() {
    let corners = vec![5u16; 4];
    let g = forest_mass_from_corners(&corners, 2, 2, 0.0, 0.0, 32.0, DENSITY_ISO);
    assert_eq!(g.fill_start_indices.len(), 1);
    assert_eq!(g.fill_positions.len(), 5 * 2);
    assert!(g.outline_segments.is_empty());
}

#[test]
fn single_inside_corner_emits_triangle_with_one_contour_edge() {
    let corners = vec![5u16, 0, 0, 0];

    let g = forest_mass_from_corners(&corners, 2, 2, 0.0, 0.0, 32.0, DENSITY_ISO);
    assert_eq!(g.fill_start_indices.len(), 1);

    assert_eq!(g.fill_positions.len(), 8);

    assert_eq!(g.outline_segments.len(), 4);
}

#[test]
fn alpha_ladder() {
    assert_eq!(forest_fill_alpha(-3.0), 0.45);
    assert_eq!(forest_fill_alpha(0.0), 0.35);
    assert_eq!(forest_fill_alpha(2.0), 0.12);
    assert_eq!(forest_fill_alpha(9.0), 0.0);
    assert_eq!(FOREST_FILL_RGB, [34, 120, 60]);
}

#[test]
fn density_iso_is_two() {
    assert!((DENSITY_ISO - 2.0).abs() < f64::EPSILON);
}

#[test]
fn count_one_corner_empty_at_default_iso() {
    let corners = vec![1u16, 0, 0, 0];
    let g = forest_mass_from_corners(&corners, 2, 2, 0.0, 0.0, 32.0, DENSITY_ISO);
    assert!(g.fill_positions.is_empty());
    assert!(g.outline_segments.is_empty());
}
