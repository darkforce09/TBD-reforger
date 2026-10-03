//! The casing and centreline buffers of the roads visible at a zoom.
//!
//! **Role:** packs every visible road's casing strip under its centreline strip as
//! `[x, y, r, g, b, a]` vertices in world metres ([`compose_roads_mesh`]).
//! **Position:** reads [`crate::styling`]'s class table, gates and strip expansion; the map
//! engine's road loader and buffers call it.
//! **Signals & state:** none; pure functions.
//! **Invariants:** the casing of every road is packed before any centreline, so centrelines draw
//! over casings.

use crate::styling::{StripVertex, compose_road_segment, pack_strip_verts, road_class_visible};

/// Road strip pair (casing under, centerline over).
#[derive(Clone, Debug, Default)]
pub struct RoadMeshGpu {
    /// Casing vertices of every visible road, packed `[x, y, r, g, b, a]` in world metres;
    /// drawn first, under the centrelines.
    pub casing: Vec<f32>,

    /// Centreline vertices of every visible road, packed `[x, y, r, g, b, a]` in world metres;
    /// drawn over the casings.
    pub centerline: Vec<f32>,

    /// Number of roads that passed the zoom gate and produced at least one strip vertex.
    pub segment_count: u32,
}

/// Road segment input for compose.
pub struct RoadInput<'a> {
    /// Road class name, one of [`crate::road_class::ROAD_CLASSES`]; any other draws nothing.
    pub road_class: &'a str,

    /// Centreline vertices `[x, y]` in world metres.
    pub points: &'a [[f64; 2]],

    /// Measured road width in metres; outside 0.3–40 m the class's fallback width applies.
    pub width_m: f64,
}

/// Strokes every road visible at `deck_zoom` into casing and centreline buffers; with
/// `airfield_polish`, runways take the fixed-width cartographic runway style.
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
