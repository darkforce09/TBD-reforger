//! Role: mesh.
//! Position: `world/terrain/roads` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::world::terrain::roads::styling::{
    StripVertex, compose_road_segment, pack_strip_verts, road_class_visible,
};

/// Road strip pair (casing under, centerline over).
#[derive(Clone, Debug, Default)]
pub struct RoadMeshGpu {
    /// Casing.
    pub casing: Vec<f32>,

    /// Centerline.
    pub centerline: Vec<f32>,

    /// Segment count.
    pub segment_count: u32,
}

/// Road segment input for compose.
pub struct RoadInput<'a> {
    /// Road class.
    pub road_class: &'a str,

    /// Points.
    pub points: &'a [[f64; 2]],

    /// Width m.
    pub width_m: f64,
}

/// Compose roads mesh.
#[must_use]
pub fn compose_roads_mesh(
    roads: &[RoadInput<'_>],
    deck_zoom: f64,
    airfield_polish: bool,
) -> RoadMeshGpu {
    let mut casing_v: Vec<StripVertex> = Vec::new();
    let mut center_v: Vec<StripVertex> = Vec::new();
    let mut segment_count = 0_u32;
    for r in roads {
        if !road_class_visible(r.road_class, deck_zoom) {
            continue;
        }
        let polish = airfield_polish && r.road_class == "runway";
        let (c, n) = compose_road_segment(r.points, r.width_m, r.road_class, polish);
        if c.is_empty() && n.is_empty() {
            continue;
        }
        casing_v.extend(c);
        center_v.extend(n);
        segment_count += 1;
    }
    RoadMeshGpu {
        casing: pack_strip_verts(&casing_v),
        centerline: pack_strip_verts(&center_v),
        segment_count,
    }
}
