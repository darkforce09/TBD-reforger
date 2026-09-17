//! Pure geometry and lane payloads for the building viewer.

use super::ViewFloor;
use website_map_engine::editing::tools::line_of_sight::viewshed_texture::{
    pack_rgba_256, ViewshedTexture,
};
use website_map_engine::spatial::los::interior::wash::LevelWash;
use website_map_engine::spatial::los::terrain::viewshed::Visibility;
use website_map_engine::world::architecture::blueprint::structure::BuildingBlueprint;
use website_map_engine::world::architecture::blueprint::structure::BuildingLevel;
use website_map_engine::world::architecture::section::cutter::through_voids;
use website_map_engine::world::architecture::section::cutter::BuildingDrawing;
use website_map_engine::world::architecture::section::cutter::HeightField;
use website_map_engine::world::architecture::section::cutter::FLOOR_WINDOW_M;
use website_map_engine::world::architecture::section::cutter::PIT_DEPTH_M;
use website_map_engine::world::architecture::section::cutter::PLAN_CELL_M;
use website_map_engine::world::mesh::triangulate::triangulate_simple;
use website_map_engine::world::terrain::roads::styling::expand_polyline_strip;
use website_map_engine::world::terrain::roads::styling::StripVertex;

/// The building is placed at the engine's world anchor so f32 lane coords stay tiny.
pub const ANCHOR: [f64; 2] = [6400.0, 6400.0];

// Palette (linear RGBA) — tuned for the site's dark surface.
/// Canvas clear colour behind the plan.
pub const COL_BG: [f64; 3] = [0.043, 0.055, 0.075];
/// Floor plate fill.
pub const COL_FLOOR: [f32; 4] = [0.16, 0.20, 0.26, 0.85];
/// Stair plate fill.
pub const COL_STAIRS: [f32; 4] = [0.44, 0.36, 0.70, 0.75];
/// Exterior wall outline.
pub const COL_WALL_EXT: [f32; 4] = [0.80, 0.83, 0.88, 1.0];
/// Interior partition outline.
pub const COL_WALL_INT: [f32; 4] = [0.55, 0.58, 0.66, 1.0];
/// Geometry on a level other than the one in view, shown faintly for context.
pub const COL_GHOST: [f32; 4] = [0.55, 0.60, 0.70, 0.35];
/// Window aperture.
pub const COL_WINDOW: [f32; 4] = [0.20, 0.78, 0.95, 1.0];
/// Door leaf whose state is open.
pub const COL_DOOR_OPEN: [f32; 4] = [0.30, 0.85, 0.45, 1.0];
/// Door leaf whose state is closed.
pub const COL_DOOR_CLOSED: [f32; 4] = [0.95, 0.63, 0.20, 1.0];
/// Furniture offering low cover.
pub const COL_FURN_LOW: [f32; 4] = [0.92, 0.80, 0.25, 0.85];
/// Furniture offering full cover.
pub const COL_FURN_FULL: [f32; 4] = [0.90, 0.34, 0.28, 0.90];
/// Furniture offering no cover.
pub const COL_FURN_NONE: [f32; 4] = [0.45, 0.48, 0.55, 0.55];
/// A door's swing arc.
pub const COL_ARC: [f32; 4] = [0.70, 0.75, 0.85, 0.65];
/// Surface-normal tick drawn off an aperture.
pub const COL_NORMAL: [f32; 4] = [0.20, 0.78, 0.95, 0.80];
/// Hatching over a stair flight.
pub const COL_HATCH: [f32; 4] = [0.75, 0.70, 0.95, 0.55];
/// Probe-ray span nothing occludes.
pub const RAY_CLEAR: [f32; 4] = [0.25, 0.90, 0.40, 1.0];
/// Probe-ray span crossing glazing.
pub const RAY_GLASS: [f32; 4] = [0.20, 0.80, 0.95, 1.0];
/// Probe-ray span crossing cover that degrades but does not block sight.
pub const RAY_COVER: [f32; 4] = [0.95, 0.85, 0.20, 1.0];
/// Probe-ray span an opaque surface blocks.
pub const RAY_BLOCKED: [f32; 4] = [0.95, 0.25, 0.20, 1.0];
/// Roof heightfield ramp (eave → ridge) + the above-ridge chimney accent.
pub const COL_ROOF_LO: [f32; 4] = [0.16, 0.22, 0.33, 0.92];
/// Ridge end of the roof heightfield ramp.
pub const COL_ROOF_HI: [f32; 4] = [0.82, 0.86, 0.95, 0.95];
/// Accent for roof geometry standing above the ridge, chimneys above all.
pub const COL_ROOF_CHIMNEY: [f32; 4] = [0.95, 0.63, 0.20, 0.95];
/// Floor plate ramp (±0.4 m around the level base — landings read) + ring edge accent.
pub const COL_PLATE_LO: [f32; 4] = [0.10, 0.13, 0.18, 0.85];
/// High end of the floor plate ramp.
pub const COL_PLATE_HI: [f32; 4] = [0.24, 0.30, 0.40, 0.90];
/// Accent along a floor plate's ring edge.
pub const COL_PLATE_EDGE: [f32; 4] = [0.45, 0.62, 0.72, 0.80];
/// Mesh section cuts: the eye-height outline (bright hairline), the low cut (dim), and the
/// deeper-than-one-floor ghost seen through voids.
pub const COL_CUT: [f32; 4] = [0.92, 0.94, 0.98, 1.0];
/// The dim low cut that picks up sills below eye height.
pub const COL_CUT_LOW: [f32; 4] = [0.55, 0.60, 0.72, 0.60];
/// Geometry more than one floor down, seen through a void.
pub const COL_GHOST_DEEP: [f32; 4] = [0.55, 0.60, 0.70, 0.20];
/// Width of the eye-height cut strips (m): weight when zoomed in; the hairline carries it
/// at low zoom.
pub const CUT_STRIP_M: f64 = 0.05;
/// Heightfield ramp ends beyond the plate pair: a pit (a void down to the floor below)
/// and a raised surface (treads, sills, a lower roof seen from above).
pub const COL_PIT: [f32; 4] = [0.05, 0.07, 0.10, 0.85];
/// The raised end of that ramp.
pub const COL_RAISED: [f32; 4] = [0.50, 0.58, 0.72, 0.92];
/// Viewshed wash: green where A sees (α 0.27), nothing elsewhere.
pub const WASH_VISIBLE_RGBA: [u8; 4] = [64, 230, 102, 70];
/// Fully transparent texel written wherever the viewshed wash says A sees nothing.
pub const WASH_CLEAR_RGBA: [u8; 4] = [0, 0, 0, 0];

/// Blueprint-local plan `[x, z]` → engine world `[x, y]`. The ortho camera is deck.gl
/// `flipY:false` — world **+y renders UP** — so mapping game +z (north) straight onto +y
/// puts north at the top of the screen. (The A.1 build assumed +y down and mirrored every
/// DOM overlay about the viewport center; screenshots pinned the engine at y-up.)
#[must_use]
pub fn to_world(p: [f64; 2]) -> [f64; 2] {
    [ANCHOR[0] + p[0], ANCHOR[1] + p[1]]
}

/// Inverse of [`to_world`].
#[must_use]
pub fn from_world(w: [f64; 2]) -> [f64; 2] {
    [w[0] - ANCHOR[0], w[1] - ANCHOR[1]]
}

/// World → CSS-pixel screen position for the camera state (`zoom` = log2 px/m, target at the
/// viewport center). Screen y grows DOWN while world y renders UP, hence the flip.
#[must_use]
pub fn world_to_screen(w: [f64; 2], tx: f64, ty: f64, zoom: f64, css: (f64, f64)) -> [f64; 2] {
    let s = zoom.exp2();
    [(w[0] - tx) * s + css.0 * 0.5, css.1 * 0.5 - (w[1] - ty) * s]
}

/// Inverse of [`world_to_screen`].
#[must_use]
pub fn screen_to_world(p: [f64; 2], tx: f64, ty: f64, zoom: f64, css: (f64, f64)) -> [f64; 2] {
    let s = zoom.exp2();
    [(p[0] - css.0 * 0.5) / s + tx, ty + (css.1 * 0.5 - p[1]) / s]
}

/// Camera that fits the blueprint's overall bbox with 25% margin (zoom capped at the ortho
/// camera's `MAX_ZOOM` so tiny sheds don't over-magnify).
#[must_use]
pub fn fit_camera(bp: &BuildingBlueprint, css: (f64, f64)) -> (f64, f64, f64) {
    let bb = &bp.overall_footprint.bounding_box2_d;
    let w = (bb.max[0] - bb.min[0]).max(1.0);
    let d = (bb.max[1] - bb.min[1]).max(1.0);
    let cx = (bb.min[0] + bb.max[0]) * 0.5;
    let cz = (bb.min[1] + bb.max[1]) * 0.5;
    let target = to_world([cx, cz]);
    let zoom = ((css.0 / (w * 1.25)).min(css.1 / (d * 1.25)))
        .log2()
        .min(website_map_engine::camera::ortho::state::MAX_ZOOM);
    (target[0], target[1], zoom)
}

/// Even-odd point-in-polygon over blueprint-local plan coords.
#[must_use]
pub fn point_in_polygon(p: [f64; 2], ring: &[[f64; 2]]) -> bool {
    let mut inside = false;
    let n = ring.len();
    if n < 3 {
        return false;
    }
    let mut j = n - 1;
    for i in 0..n {
        let (a, b) = (ring[i], ring[j]);
        if ((a[1] > p[1]) != (b[1] > p[1]))
            && (p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0])
        {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// Appends an expanded polyline strip to a lane as interleaved `x, y, r, g, b, a` vertices.
pub(crate) fn push_strip(out: &mut Vec<f32>, verts: &[StripVertex]) {
    for v in verts {
        out.extend_from_slice(&[
            v.pos[0], v.pos[1], v.color[0], v.color[1], v.color[2], v.color[3],
        ]);
    }
}

/// Appends one flat-coloured line segment, already in world coordinates, to a line lane.
pub(crate) fn seg(out: &mut Vec<f32>, a: [f64; 2], b: [f64; 2], c: [f32; 4]) {
    for p in [a, b] {
        out.extend_from_slice(&[p[0] as f32, p[1] as f32, c[0], c[1], c[2], c[3]]);
    }
}

/// Axis-aligned-in-local-frame rectangle (center/size/rotation°) → 4 world-space corners.
#[must_use]
pub fn rect_corners(center: [f64; 2], size: [f64; 2], rot_deg: f64) -> [[f64; 2]; 4] {
    let (hw, hd) = (size[0] * 0.5, size[1] * 0.5);
    let (s, c) = rot_deg.to_radians().sin_cos();
    let rot = |x: f64, z: f64| [center[0] + x * c - z * s, center[1] + x * s + z * c];
    [rot(-hw, -hd), rot(hw, -hd), rot(hw, hd), rot(-hw, hd)]
}

/// Appends a flat-coloured quad, given as four blueprint-local corners, as two triangles.
pub(crate) fn quad(out: &mut Vec<f32>, corners: [[f64; 2]; 4], col: [f32; 4]) {
    // Two triangles, corners in local plan coords → world.
    for idx in [0usize, 1, 2, 0, 2, 3] {
        let w = to_world(corners[idx]);
        out.extend_from_slice(&[w[0] as f32, w[1] as f32, col[0], col[1], col[2], col[3]]);
    }
}

/// Everything the static lanes need for one (blueprint, active-floor) state.
#[derive(Default)]
pub struct StaticLanes {
    /// `LANDCOVER` polygon mesh: floor plate + stairs plates.
    pub floor_pos: Vec<f32>,
    pub floor_col: Vec<f32>,
    pub floor_idx: Vec<u32>,
    /// `AIRFIELD_APRON` polygon mesh: furniture plates.
    pub furn_pos: Vec<f32>,
    pub furn_col: Vec<f32>,
    pub furn_idx: Vec<u32>,
    /// `ROADS_CASING` strip tris: thickness walls.
    pub walls: Vec<f32>,
    pub wall_count: u32,
    /// `ROADS` strip tris: aperture overlays.
    pub apertures: Vec<f32>,
    pub aperture_count: u32,
    /// `CONTOURS` hairlines: low cut + void ghosts + arcs + normals + hatch + rings.
    pub hairlines: Vec<f32>,
    pub hairline_count: u32,
    /// `FOREST_OUTLINE` hairlines: the mesh's eye-height section (0 on the fallback path).
    pub cuts: Vec<f32>,
    pub cut_count: u32,
    /// Door swing arcs (hairlines) — `INTERIOR_PORTALS_OUTLINE`.
    pub arcs: Vec<f32>,
    pub arc_count: u32,
    /// Stairs tread hatch (hairlines) — `INTERIOR_STAIRS`.
    pub stairs: Vec<f32>,
    pub stairs_count: u32,
    /// Covered `RoofGrid` cells painted on the Roof view (0 elsewhere / roofless).
    pub roof_cell_count: u32,
    /// Covered `PlateGrid` cells painted on the active Level view (0 on plate-less levels).
    pub plate_cell_count: u32,
    /// Mesh faces painted on the floor lane (floor faces on a Level view, roof faces on
    /// the Roof view; 0 without a drawing).
    pub mesh_cell_count: u32,
}

/// Triangulates one blueprint-local ring and appends it to an indexed polygon lane, offsetting
/// the new indices past whatever the lane already holds.
pub(crate) fn append_polygon(
    pos: &mut Vec<f32>,
    col: &mut Vec<f32>,
    idx: &mut Vec<u32>,
    ring_local: &[[f64; 2]],
    color: [f32; 4],
) {
    let ring_world: Vec<[f64; 2]> = ring_local.iter().map(|&p| to_world(p)).collect();
    let mesh = triangulate_simple(&ring_world);
    let base = (pos.len() / 2) as u32;
    pos.extend_from_slice(&mesh.positions);
    for _ in 0..mesh.positions.len() / 2 {
        col.extend_from_slice(&color);
    }
    idx.extend(mesh.indices.iter().map(|i| base + i));
}

/// Piecewise-linear colour ramp over ascending `(height, colour)` stops, clamped at both
/// ends. Pure; the stepped-gradient look comes from applying it per 0.2 m cell.
#[must_use]
pub fn ramp(stops: &[(f64, [f32; 4])], y: f64) -> [f32; 4] {
    let Some(first) = stops.first() else {
        return [0.0; 4];
    };
    if y <= first.0 {
        return first.1;
    }
    for w in stops.windows(2) {
        let ((y0, c0), (y1, c1)) = (w[0], w[1]);
        if y <= y1 {
            let t = if y1 > y0 {
                ((y - y0) / (y1 - y0)) as f32
            } else {
                1.0
            };
            return [
                c0[0] + (c1[0] - c0[0]) * t,
                c0[1] + (c1[1] - c0[1]) * t,
                c0[2] + (c1[2] - c0[2]) * t,
                c0[3] + (c1[3] - c0[3]) * t,
            ];
        }
    }
    stops.last().map_or([0.0; 4], |s| s.1)
}

mod static_lanes;
pub use static_lanes::build_static_lanes;

/// Visible wash cells are green; hidden and unknown cells are transparent.
/// Building walls supply the contrast, so the overlay highlights visibility.
#[must_use]
pub fn wash_cell_rgba(v: Visibility) -> [u8; 4] {
    match v {
        Visibility::Visible => WASH_VISIBLE_RGBA,
        Visibility::Hidden | Visibility::Unknown => WASH_CLEAR_RGBA,
    }
}

/// One level's visibility raster → the engine's viewshed texture payload: [`wash_cell_rgba`]
/// per cell, rows straight through (the raster is already north-first, which IS the
/// texture's row-0 = world max-y contract), rows padded to 256 bytes for `write_texture`,
/// world rect from the local plan rect. Pure: native tests pin the bytes and the rect.
/// Replaces the 720-ray centre-fan of the single-floor viewshed, whose fan topology leaked
/// light around occluders — a per-cell raster cannot.
#[must_use]
pub fn wash_texture(w: &LevelWash) -> ViewshedTexture {
    let mut tight = Vec::with_capacity(w.cols * w.rows * 4);
    for &cell in &w.cells {
        tight.extend_from_slice(&wash_cell_rgba(cell));
    }
    let (rgba, stride_bytes) = pack_rgba_256(&tight, w.cols, w.rows);
    let lo = to_world([w.min_x, w.min_z]);
    let hi = to_world([w.max_x, w.max_z]);
    ViewshedTexture {
        min_x: lo[0],
        min_y: lo[1],
        max_x: hi[0],
        max_y: hi[1],
        tex_w: w.cols as u32,
        tex_h: w.rows as u32,
        rgba,
        stride_bytes,
    }
}

#[cfg(test)]
#[path = "tests/geometry_and_lanes.rs"]
mod geometry_and_lanes_tests;
