//! Role: compose tests.
//! Position: `renderers/primitives/tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::renderers::primitives::compose::*;
// T-0xx Phase 1D: `compose.rs` dropped its four `pub use crate::terrain::{roads,water}::mesh`
// re-exports, so the road and sea cases below name their own module directly.
use crate::terrain::dem::grid::DemVectorGrid;
use crate::terrain::relief::sea_band::SeaBandGeometry;
use crate::terrain::relief::sea_band::build_sea_band_geometry;
use crate::terrain::roads::mesh::RoadInput;
use crate::terrain::roads::mesh::compose_roads_mesh;
use crate::terrain::water::mesh::compose_sea_mesh;

#[test]
fn roads_gated_by_zoom() {
    let pts = [[0.0, 0.0], [100.0, 0.0]];
    let roads = [RoadInput {
        road_class: "path",
        points: &pts,
        width_m: 1.0,
    }];
    let hidden = compose_roads_mesh(&roads, 3.0, false);
    assert_eq!(hidden.segment_count, 0);
    let shown = compose_roads_mesh(&roads, 4.0, false);
    assert_eq!(shown.segment_count, 1);
    assert!(!shown.centerline.is_empty());
}

#[test]
fn sea_compose_from_empty_is_empty() {
    let g = SeaBandGeometry::default();
    let m = compose_sea_mesh(&g, 1.0);
    assert!(m.indices.is_empty());
}

#[test]
fn sea_compose_all_ocean_has_tris() {
    let data = vec![-10.0_f32; 9];
    let g = DemVectorGrid {
        data,
        cols: 3,
        rows: 3,
        cell_x: 1.0,
        cell_y: 1.0,
        origin_x: 0.0,
        origin_y: 0.0,
        max_elev_m: -10.0,
    };
    let geo = build_sea_band_geometry(&g);
    assert!(geo.polygon_count > 0);
    let m = compose_sea_mesh(&geo, 1.0);
    assert!(!m.indices.is_empty());
    assert_eq!(m.colors.len(), m.positions.len() / 2 * 4);
}

#[test]
fn retint_fill_alpha_sets_every_fourth_only() {
    let mut cols = vec![0.2, 0.4, 0.6, 1.0, 0.1, 0.3, 0.5, 1.0];
    retint_fill_alpha(&mut cols, 0.35);
    assert_eq!(cols, vec![0.2, 0.4, 0.6, 0.35, 0.1, 0.3, 0.5, 0.35]);

    retint_fill_alpha(&mut cols, 2.0);
    assert!((cols[3] - 1.0).abs() < f32::EPSILON);
    retint_fill_alpha(&mut cols, -1.0);
    assert_eq!(cols[7], 0.0);
}

#[test]
fn two_tone_contours_split_colour_by_summit_index() {
    use crate::terrain::relief::contours::ContourRing;

    const BASE: [u8; 4] = [188, 150, 100, 235];
    const SUMMIT: [u8; 4] = [174, 145, 123, 235];

    let rings = vec![
        ContourRing {
            level: 10.0,
            closed: false,
            points: vec![(0.0, 0.0), (1.0, 0.0), (2.0, 0.0)],
        },
        ContourRing {
            level: 30.0,
            closed: true,
            points: vec![(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)],
        },
    ];
    let hair = compose_two_tone_contours(&rings, &[1], BASE, SUMMIT);

    assert_eq!(hair.segment_count, 5);
    assert_eq!(hair.verts.len(), 5 * 2 * 6);

    let base_f = u8_rgba_to_f32(BASE, 1.0);
    let summit_f = u8_rgba_to_f32(SUMMIT, 1.0);

    for v in 0..10 {
        let rgba = &hair.verts[v * 6 + 2..v * 6 + 6];
        let want = if v < 4 { &base_f } else { &summit_f };
        assert_eq!(rgba, &want[..], "vertex {v} carries the wrong tone");
    }

    assert_ne!(base_f[0], summit_f[0]);
    assert_eq!(i32::from(BASE[0]) - i32::from(BASE[2]), 88);
    assert_eq!(i32::from(SUMMIT[0]) - i32::from(SUMMIT[2]), 51);
}
