//! Zones panel zone geometry.

use super::*;

/// Quantizes a coordinate to the document grid.
pub(super) fn round_coord(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
}

/// Checks whether a radius remains positive after quantization.
#[must_use]
pub fn radius_survives_compile(r: f64) -> bool {
    r.is_finite() && r > 0.0 && round_coord(r) > 0.0
}

/// Smallest circle radius that survives document quantization.
pub const MIN_AUTHORABLE_RADIUS_M: f64 = 0.05;

/// Zone coordinate grid size in metres.
pub const ZONE_GRID_M: f64 = 0.1;

/// Builds a quantized circle from a center and rim click.
#[must_use]
pub fn circle_from_clicks(cx: f64, cz: f64, rim_x: f64, rim_z: f64) -> Option<(f64, f64, f64)> {
    if ![cx, cz, rim_x, rim_z].iter().all(|v| v.is_finite()) {
        return None;
    }
    let r = (rim_x - cx).hypot(rim_z - cz);
    radius_survives_compile(r).then_some((cx, cz, r))
}

/// Checks whether a polygon has enough distinct vertices.
#[must_use]
pub fn polygon_is_committable(verts: &[(f64, f64)]) -> bool {
    verts.len() >= 3 && verts.iter().all(|(x, z)| x.is_finite() && z.is_finite())
}

/// Flattens polygon vertices into document coordinate order.
#[must_use]
pub fn polygon_flat(verts: &[(f64, f64)]) -> Vec<f64> {
    let mut out = Vec::with_capacity(verts.len() * 2);
    for (x, z) in verts {
        out.push(*x);
        out.push(*z);
    }
    out
}

/// Label assigned to a whole-terrain play area.
pub const WHOLE_TERRAIN_ZONE_LABEL: &str = "Play Area";

/// Finds the schema type used for a play area.
#[must_use]
pub fn whole_terrain_zone_type() -> Option<String> {
    zone_types().into_iter().find(|t| t == "boundary")
}

/// Checks whether terrain bounds form a finite rectangle.
#[must_use]
pub fn terrain_rect_is_authorable(bounds: [f64; 4]) -> bool {
    let [min_x, min_z, max_x, max_z] = bounds;
    let has_area = round_coord(max_x - min_x) > 0.0 && round_coord(max_z - min_z) > 0.0;
    has_area && polygon_is_committable(&terrain_rect_corners(bounds))
}

fn terrain_rect_corners(bounds: [f64; 4]) -> [(f64, f64); 4] {
    let [min_x, min_z, max_x, max_z] = bounds;
    [
        (min_x, min_z),
        (max_x, min_z),
        (max_x, max_z),
        (min_x, max_z),
    ]
}

/// Builds the terrain boundary ring for the current terrain.
#[must_use]
pub fn terrain_rect_ring(terrain: &str, bounds: [f64; 4]) -> Option<Vec<f64>> {
    if bounds != website_map_engine::data::scenario::compile::terrain_bounds(terrain) {
        return None;
    }
    if !terrain_rect_is_authorable(bounds) {
        return None;
    }
    Some(polygon_flat(&terrain_rect_corners(bounds)))
}

pub use website_map_engine::data::store::operations::zones::ZoneShape;

/// Projected screen endpoints of an owner link.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProjectedOwnerLine {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
}

/// Projects an owner link from world to screen coordinates.
#[must_use]
pub fn project_owner_line<F>(a: (f64, f64), b: (f64, f64), project: F) -> ProjectedOwnerLine
where
    F: Fn(f64, f64) -> (f64, f64),
{
    let (x1, y1) = project(a.0, a.1);
    let (x2, y2) = project(b.0, b.1);
    ProjectedOwnerLine { x1, y1, x2, y2 }
}

/// Authors one play area covering the current terrain.
#[cfg(target_arch = "wasm32")]
#[must_use]
pub fn add_whole_terrain_zone() -> Option<String> {
    use website_map_engine::data::store::operations::entity::{terrain_bounds_of, terrain_key_of};
    use website_map_engine::editing::hosted_commands as engine_ops;

    let (terrain, bounds) = website_map_engine::editing::host::with_doc(|core| {
        (terrain_key_of(core), terrain_bounds_of(core))
    })?;
    let ring = terrain_rect_ring(&terrain, bounds)?;

    let kind = whole_terrain_zone_type()?;
    engine_ops::add_authored_row(DrawTarget::Zone, |core, id| {
        core.add_polygon_zone_labelled(id, &kind, &ring, Some(WHOLE_TERRAIN_ZONE_LABEL));
    })
}
