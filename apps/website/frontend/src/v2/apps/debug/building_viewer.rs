//! The single-prefab building-blueprint bench served at `/debug/building-viewer`.
//!
//! **Role:** a hyper-focused instrument for eyeballing what the Workbench extractor produced for
//! one building — floor plates, thickness walls, apertures, furniture cover, stairs — and for
//! driving `evaluate_los` interactively: the BVH raycast over the building's `.bvh` occlusion
//! sidecar, attributed through the blueprint, with a draggable observer and target, elevation
//! sliders, and the ray coloured along the ordered [`LosHit`] trace. Alt+click instead casts the
//! multi-floor viewshed wash — `website_map_engine::spatial::los::interior::wash::level_washes`
//! fires one BVH ray at every 0.25 m cell at eye height on every level — and the floor rail swaps
//! which level's raster the engine's `Viewshed` texture lane shows.
//!
//! **Position:** a routed workspace under `v2::apps::debug`, mounted by `app_routes.rs`, public by
//! URL and absent from the navigation. It drives the map and graphics engines directly and shares
//! none of the editor's boot machinery — no IndexedDB, no hydration, no DEM, satellite or world
//! loaders. Its interior lane tessellation lives in [`super::building_interior`].
//!
//! **Signals & state:** Leptos `RwSignal`s hold the fetched blueprint, the occlusion sidecar, the
//! assembled compound, the per-door open/closed states, the viewed floor, the observer and target,
//! the camera, and the current wash. The `RenderEngine` sits in an `Rc<RefCell<…>>` handle the
//! wasm mount owns; everything decidable is a pure function in [`geom`] — world mapping, camera
//! fit, lane tessellation, ray-span colouring, point-in-polygon — so the native test suite proves
//! the geometry with no browser and the wasm block only wires signals, listeners and the engine.
//!
//! **Invariants:** the building sits at world `ANCHOR` (6400, 6400) with blueprint +z, which is
//! game north, mapped up on screen, so f32 lane coordinates stay small; the camera is the engine's
//! own ortho camera (`zoom` = log2 px/m, capped at 6, so 64 px/m ≈ 18 px for a 0.28 m log wall).
//! Lanes are addressed by `role_id::*` constants and never by a re-copied integer. Every run is
//! reproducible from the URL: `?prefab=<map-assets path>` overrides the default farmhouse golden,
//! `?scene=1` adds exterior trees from `<slug>.scene.json`, `?doors=open` opens every leaf on load,
//! `?a=x,y,z&b=x,y,z` fixes the ray ends, and `?force=webgl` selects the headless capture backend.
//!
//! The blueprint JSON is fetched from `/map-assets/everon/prefabs/buildings/…`, served by the API
//! and proxied by Trunk in development, and the sidecar from the same path with `.json` → `.bvh`.
//! Without a sidecar the plan still draws, but LOS and the viewshed stay off and the header says
//! so. With `<slug>.instances.json` beside the blueprint the bench assembles the
//! `CompoundBuilding` — shell plus every door leaf, frame, pane and furniture BLAS under its
//! socket transform — and LOS, the wash and the section cuts all run over that instead; a click on
//! a leaf swings it.
//!
//! The 2D drawing is the mesh's own: per-floor section cuts through the collision triangles at eye
//! height (walls as true double-line outlines, windows as gaps, mullions, columns and mesh
//! furniture as outlines), plus a dim low cut for sills, the slab faces as the floor, the roof
//! faces on the Roof view, and — through a floor's voids — the floors below. The blueprint's
//! walls, plates and RoofGrid are the no-sidecar fallback; its apertures, furniture, stairs, swing
//! arcs and rings stay as annotations over the mesh. The blueprint reaches the GPU through the
//! generic upload API — `upload_polygon_mesh`, `upload_strip_tris`, `upload_hairline_segments` —
//! on these lanes:
//!
//! | lane (draw order ↑)                 | content |
//! |-------------------------------------|---------|
//! | `INTERIOR_SLABS` (poly)             | the MESH heightfield clipped below this floor's cut plane, one 0.2 m cell quad per surface, height-ramped (Roof view: the full top surface eave→ridge; fallback: blueprint plate / RoofGrid) |
//! | `INTERIOR_FURNITURE` (+outline)     | furniture / prop footprints — the compound's instances (world AABB, cover-tier colour); blueprint plates without one |
//! | `INTERIOR_WALLS` (strip)            | the MESH section cut at eye height as 0.05 m strips (fallback: blueprint walls at nominal thickness) |
//! | `INTERIOR_WALLS_OUTLINE` (hairline) | the same cut as constant 1 px hairlines, the low cut (sills), lower floors' cuts through voids, window normals, rings, ghosts |
//! | `INTERIOR_PORTALS` (+outline)       | door leaves where they hang (orange closed / green open) + door frames; outline = swing arcs (blueprint overlays / arcs without a compound) |
//! | `INTERIOR_GLAZING` (+outline)       | glass pane cuts (cyan); outline = window-frame jamb ticks |
//! | `INTERIOR_STAIRS` (hairline)        | tread hatch |
//! | `SCENE_VEGETATION` (+outline)       | `?scene=1` trees: trunk disc + canopy, rim + stipple |
//! | `Viewshed` (texture)                | the viewed level's visibility wash (`viewshed_upload`; green where A sees) |
//! | `INTERIOR_PROBE` (strip)            | the LOS ray, split + coloured at each `LosHit` (cyan past glass, yellow-green past canopy), plus event dots |
#![allow(dead_code)] // native build: the wasm host wires the live path; tests pin the pure core.

use std::sync::Arc;

use leptos::prelude::*;
use website_map_engine::spatial::bvh::sidecar::BvhSidecar;
use website_map_engine::spatial::los::interior::wash::level_wash;
use website_map_engine::spatial::los::interior::wash::level_wash_compound;
use website_map_engine::spatial::los::interior::wash::LevelWash;
use website_map_engine::spatial::los::interior::wash::WashParams;
use website_map_engine::world::architecture::blueprint::attribution_1::LosHitKind;
use website_map_engine::world::architecture::blueprint::attribution_1::LosResult;
use website_map_engine::world::architecture::blueprint::structure::BuildingBlueprint;
use website_map_engine::world::architecture::compound::assembly::CompoundBuilding;
use website_map_engine::world::architecture::section::cutter::building_drawing;
use website_map_engine::world::architecture::section::cutter::BuildingDrawing;

use super::building_interior::LevelCuts;

/// Default blueprint when no `?prefab=` override is present — the scanned FarmHouse (roof
/// heightfield + verbatim plates + attic). The hand-authored pre-scan asset stays reachable
/// via `?prefab=/map-assets/everon/prefabs/buildings/FarmHouse_E_1L01.json`.
const DEFAULT_PREFAB_PATH: &str = "/map-assets/everon/prefabs/buildings/FarmHouse_E_1L01_Wood.json";

/// One end of the LOS ray in blueprint-local coordinates (x/z plan, y elevation).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RayEnd {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// Mirrored camera state (the engine owns the truth; these re-render the DOM overlay).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cam {
    pub tx: f64,
    pub ty: f64,
    pub zoom: f64,
}

/// What a pointer-drag currently moves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Drag {
    None,
    Observer,
    Target,
    Pan,
}

/// Which plan the bench displays: one architectural floor, or the roof plate above them all.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewFloor {
    Level(usize),
    Roof,
}

impl ViewFloor {
    /// The elevation band this view claims, plus whether it is the topmost band (closed upper
    /// bound — the same half-open rule the raycaster's `clip_t_to_band` uses). Roof owns
    /// everything from the top level's ceiling to the building's total height.
    #[must_use]
    pub fn band(self, bp: &BuildingBlueprint) -> ([f64; 2], bool) {
        match self {
            ViewFloor::Level(i) => {
                let last = i + 1 == bp.levels.len();
                let band = bp.levels.get(i).map_or([0.0, 0.0], |l| l.elevation_range);
                // A floor below the roof band is never "topmost": the roof view owns the space
                // above the last ceiling, so every level band stays half-open.
                (band, last && bp.vertical_profile.total_height_m <= band[1])
            }
            ViewFloor::Roof => {
                let floor = bp.levels.last().map_or(0.0, |l| l.elevation_range[1]);
                ([floor, bp.vertical_profile.total_height_m.max(floor)], true)
            }
        }
    }
}

// ═════════════════════════════════════════════════════════════════════════════════════════════
// Pure geometry — native-tested, no leptos/web/engine types.
// ═════════════════════════════════════════════════════════════════════════════════════════════
pub mod geom {
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
        /// Door swing arcs (hairlines) — `INTERIOR_PORTALS_OUTLINE` (T-090.11.6).
        pub arcs: Vec<f32>,
        pub arc_count: u32,
        /// Stairs tread hatch (hairlines) — `INTERIOR_STAIRS` (T-090.11.6).
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

    /// A mesh heightfield onto the floor lane: one quad per cell with a surface, tinted through
    /// [`ramp`]; cells above `accent_above` take the chimney accent. Relief reads at a glance —
    /// the same stepped gradient the RoofGrid cells had, now from the mesh on every view.
    fn paint_heightfield(
        out: &mut StaticLanes,
        hf: &HeightField,
        stops: &[(f64, [f32; 4])],
        accent_above: Option<f64>,
    ) {
        for row in 0..hf.rows {
            for col in 0..hf.cols {
                let Some(y) = hf.at(col, row) else {
                    continue;
                };
                let color = if accent_above.is_some_and(|a| y > a) {
                    COL_ROOF_CHIMNEY
                } else {
                    ramp(stops, y)
                };
                append_polygon(
                    &mut out.floor_pos,
                    &mut out.floor_col,
                    &mut out.floor_idx,
                    &rect_corners(hf.cell_center(col, row), [hf.cell_m, hf.cell_m], 0.0),
                    color,
                );
                out.mesh_cell_count += 1;
            }
        }
    }

    /// Door swing arc as a hairline polyline: quarter circle of radius `width` around the hinge,
    /// starting on the wall line and sweeping toward the side whose arc midpoint lies inside the
    /// floor footprint for `inward` doors (outside for `outward`).
    #[allow(clippy::too_many_arguments)] // one call site; a params struct would be pure ceremony
    fn swing_arc(
        out: &mut Vec<f32>,
        count: &mut u32,
        lvl: &BuildingLevel,
        pos: [f64; 2],
        width: f64,
        hinge_side: &str,
        swing: &str,
        wall_dir: [f64; 2],
    ) {
        let u = wall_dir;
        let hinge = if hinge_side == "right" {
            [pos[0] + u[0] * width * 0.5, pos[1] + u[1] * width * 0.5]
        } else {
            [pos[0] - u[0] * width * 0.5, pos[1] - u[1] * width * 0.5]
        };
        // Leaf direction when closed = along the wall toward the door center.
        let leaf0 = [pos[0] - hinge[0], pos[1] - hinge[1]];
        let leaf_len = (leaf0[0] * leaf0[0] + leaf0[1] * leaf0[1]).sqrt().max(1e-9);
        let l0 = [leaf0[0] / leaf_len, leaf0[1] / leaf_len];
        let a0 = l0[1].atan2(l0[0]);
        for sign in [1.0f64, -1.0] {
            // Candidate quarter sweep; keep the one matching the swing side.
            let mid = a0 + sign * std::f64::consts::FRAC_PI_4;
            let mid_pt = [hinge[0] + width * mid.cos(), hinge[1] + width * mid.sin()];
            let inside = point_in_polygon(mid_pt, &lvl.footprint_polygon);
            let want_inside = swing != "outward";
            if inside != want_inside {
                continue;
            }
            let mut prev = [hinge[0] + width * a0.cos(), hinge[1] + width * a0.sin()];
            for i in 1..=16 {
                let a = a0 + sign * std::f64::consts::FRAC_PI_2 * (f64::from(i) / 16.0);
                let p = [hinge[0] + width * a.cos(), hinge[1] + width * a.sin()];
                seg(out, to_world(prev), to_world(p), COL_ARC);
                *count += 1;
                prev = p;
            }
            // Leaf line at full-open.
            seg(out, to_world(hinge), to_world(prev), COL_ARC);
            *count += 1;
            return;
        }
    }

    /// Tessellate one (blueprint, drawing, view-floor) state into lane payloads. With a mesh
    /// `drawing` the structure is the mesh's: eye-height section cuts are the walls, slab faces
    /// the floor, roof faces the roof, and lower floors show through this floor's voids; the
    /// blueprint contributes plates, apertures, furniture, stairs and rings as annotations.
    /// Without one (no sidecar) the blueprint draws everything as before. The Roof view draws
    /// the overall footprint plate plus every floor's section (or centerlines) as ghosts — the
    /// plan of what you stand ON, not a floor with its own walls.
    #[must_use]
    pub fn build_static_lanes(
        bp: &BuildingBlueprint,
        drawing: Option<&BuildingDrawing>,
        view: ViewFloor,
    ) -> StaticLanes {
        let mut out = StaticLanes::default();
        let ViewFloor::Level(active) = view else {
            // Roof.
            append_polygon(
                &mut out.floor_pos,
                &mut out.floor_col,
                &mut out.floor_idx,
                &bp.overall_footprint.polygon2_d,
                COL_FLOOR,
            );
            // Heightfield cells over the plate: dark at the eave, light at the ridge, chimney
            // spikes (above ridge + 0.3) in the accent color. This is the emitted RoofGrid drawn
            // verbatim — ridge lines, hips, valleys and dormer pits are eyeballable directly.
            if let Some(roof) = bp
                .roof
                .as_ref()
                .filter(|r| r.is_valid() && drawing.is_none())
            {
                let vp = &bp.vertical_profile;
                let span = (vp.ridge_height_m - vp.eave_height_m).max(0.1);
                let cell = roof.cell_size_m;
                for cx in 0..roof.nx {
                    for cz in 0..roof.nz {
                        let Some(h) = roof.heights_m[cx * roof.nz + cz] else {
                            continue;
                        };
                        let col = if h > vp.ridge_height_m + 0.3 {
                            COL_ROOF_CHIMNEY
                        } else {
                            let t = (((h - vp.eave_height_m) / span).clamp(0.0, 1.0)) as f32;
                            [
                                COL_ROOF_LO[0] + (COL_ROOF_HI[0] - COL_ROOF_LO[0]) * t,
                                COL_ROOF_LO[1] + (COL_ROOF_HI[1] - COL_ROOF_LO[1]) * t,
                                COL_ROOF_LO[2] + (COL_ROOF_HI[2] - COL_ROOF_LO[2]) * t,
                                COL_ROOF_LO[3] + (COL_ROOF_HI[3] - COL_ROOF_LO[3]) * t,
                            ]
                        };
                        let center = [
                            roof.origin[0] + (cx as f64 + 0.5) * cell,
                            roof.origin[1] + (cz as f64 + 0.5) * cell,
                        ];
                        append_polygon(
                            &mut out.floor_pos,
                            &mut out.floor_col,
                            &mut out.floor_idx,
                            &rect_corners(center, [cell, cell], 0.0),
                            col,
                        );
                        out.roof_cell_count += 1;
                    }
                }
            }
            // The mesh's full top surface as a stepped heightfield — the roof plan. Eave→ridge
            // ramp from the vertical profile (the field's own range when the profile is flat),
            // chimney accent above the ridge; replaces the RoofGrid cells when present.
            if let Some(d) = drawing.filter(|d| d.roof.range().is_some()) {
                let vp = &bp.vertical_profile;
                let profiled = vp.eave_height_m < vp.ridge_height_m;
                let (eave, ridge) = if profiled {
                    (vp.eave_height_m, vp.ridge_height_m)
                } else {
                    (d.roof_y[0], d.roof_y[1])
                };
                paint_heightfield(
                    &mut out,
                    &d.roof,
                    &[(eave, COL_ROOF_LO), (ridge, COL_ROOF_HI)],
                    profiled.then_some(vp.ridge_height_m + 0.3),
                );
            }
            // Every floor's wall centerlines as ghosts — few and clean, the plan through the
            // roof (the mesh cuts would be hundreds of segments of noise here).
            for ghost in &bp.levels {
                for wall in &ghost.walls {
                    seg(
                        &mut out.hairlines,
                        to_world(wall.start),
                        to_world(wall.end),
                        COL_GHOST,
                    );
                    out.hairline_count += 1;
                }
            }
            return out;
        };
        let Some(lvl) = bp.levels.get(active) else {
            return out;
        };
        let mesh_level = drawing.and_then(|d| d.levels.get(active));

        if let Some(l) = mesh_level {
            // The mesh's top surface below this floor's cut plane as a stepped heightfield:
            // floor mid-tone, treads / sills / lower roofs brighter with height, stairwell
            // pits and the floors below darker with depth. Replaces the blueprint plate, its
            // traced rings and its stairs plate — the mesh now shows all of that for real.
            paint_heightfield(
                &mut out,
                &l.surface,
                &[
                    (l.lo - PIT_DEPTH_M, COL_PIT),
                    (l.floor_min_y(), COL_PLATE_LO),
                    (l.lo + FLOOR_WINDOW_M[1], COL_PLATE_HI),
                    (l.cut_main_y, COL_RAISED),
                ],
                None,
            );
        } else {
            // Active floor plate: the verbatim PlateGrid when present (one tinted quad per covered
            // cell — partial mezzanines and double-height voids render exactly as measured, and
            // landings read via the height ramp), else the traced-polygon fill for pre-plate assets.
            match lvl.plate.as_ref().filter(|g| g.is_valid()) {
                Some(plate) => {
                    let base = lvl.elevation_range[0];
                    let cell = plate.cell_size_m;
                    for cx in 0..plate.nx {
                        for cz in 0..plate.nz {
                            let Some(h) = plate.heights_m[cx * plate.nz + cz] else {
                                continue;
                            };
                            // ±0.4 m ramp around the level base between the two floor shades.
                            let t = (((h - base) / 0.8 + 0.5).clamp(0.0, 1.0)) as f32;
                            let col = [
                                COL_PLATE_LO[0] + (COL_PLATE_HI[0] - COL_PLATE_LO[0]) * t,
                                COL_PLATE_LO[1] + (COL_PLATE_HI[1] - COL_PLATE_LO[1]) * t,
                                COL_PLATE_LO[2] + (COL_PLATE_HI[2] - COL_PLATE_LO[2]) * t,
                                COL_PLATE_LO[3] + (COL_PLATE_HI[3] - COL_PLATE_LO[3]) * t,
                            ];
                            let center = [
                                plate.origin[0] + (cx as f64 + 0.5) * cell,
                                plate.origin[1] + (cz as f64 + 0.5) * cell,
                            ];
                            append_polygon(
                                &mut out.floor_pos,
                                &mut out.floor_col,
                                &mut out.floor_idx,
                                &rect_corners(center, [cell, cell], 0.0),
                                col,
                            );
                            out.plate_cell_count += 1;
                        }
                    }
                }
                None => {
                    append_polygon(
                        &mut out.floor_pos,
                        &mut out.floor_col,
                        &mut out.floor_idx,
                        &lvl.footprint_polygon,
                        COL_FLOOR,
                    );
                }
            }
            // Traced plate boundary rings (outer + holes) as hairline loops over the grid — the
            // derived polygon contract drawn against the verbatim cells, so ring-vs-grid
            // coincidence is directly eyeballable.
            for piece in &lvl.floor_polygons {
                for ring in std::iter::once(&piece.outer).chain(piece.holes.iter()) {
                    let n = ring.len();
                    for i in 0..n {
                        seg(
                            &mut out.hairlines,
                            to_world(ring[i]),
                            to_world(ring[(i + 1) % n]),
                            COL_PLATE_EDGE,
                        );
                        out.hairline_count += 1;
                    }
                }
            }
            for st in &lvl.stairs {
                let ring = [
                    [st.bounds[0][0], st.bounds[0][1]],
                    [st.bounds[1][0], st.bounds[0][1]],
                    [st.bounds[1][0], st.bounds[1][1]],
                    [st.bounds[0][0], st.bounds[1][1]],
                ];
                append_polygon(
                    &mut out.floor_pos,
                    &mut out.floor_col,
                    &mut out.floor_idx,
                    &ring,
                    COL_STAIRS,
                );
                // Tread hatch: lines across the short axis.
                let (w, d) = (
                    st.bounds[1][0] - st.bounds[0][0],
                    st.bounds[1][1] - st.bounds[0][1],
                );
                let n = st.step_count.clamp(4, 24);
                for i in 1..n {
                    let f = f64::from(i) / f64::from(n);
                    let (a, b) = if w >= d {
                        let x = st.bounds[0][0] + w * f;
                        ([x, st.bounds[0][1]], [x, st.bounds[1][1]])
                    } else {
                        let z = st.bounds[0][1] + d * f;
                        ([st.bounds[0][0], z], [st.bounds[1][0], z])
                    };
                    seg(&mut out.stairs, to_world(a), to_world(b), COL_HATCH);
                    out.stairs_count += 1;
                }
            }
        }

        match mesh_level {
            // The mesh's section at eye height IS the wall drawing: true double-line outlines,
            // window gaps, mullions, columns, collision furniture — one strip per segment for
            // weight when zoomed in (ROADS_CASING) plus a constant 1 px hairline
            // (FOREST_OUTLINE) so the outline never thins out at low zoom. The low cut draws
            // the wall continuous under the window gaps (sills), dim.
            Some(l) => {
                for s in &l.cut_main {
                    let pts = [to_world(s[0]), to_world(s[1])];
                    let verts = expand_polyline_strip(&pts, CUT_STRIP_M, COL_WALL_EXT);
                    push_strip(&mut out.walls, &verts);
                    out.wall_count += 1;
                    seg(&mut out.cuts, pts[0], pts[1], COL_CUT);
                    out.cut_count += 1;
                }
                for s in &l.cut_low {
                    seg(
                        &mut out.hairlines,
                        to_world(s[0]),
                        to_world(s[1]),
                        COL_CUT_LOW,
                    );
                    out.hairline_count += 1;
                }
            }
            // Fallback: the blueprint's walls at nominal thickness.
            None => {
                for wall in &lvl.walls {
                    let col = if wall.is_exterior {
                        COL_WALL_EXT
                    } else {
                        COL_WALL_INT
                    };
                    let pts = [to_world(wall.start), to_world(wall.end)];
                    let verts = expand_polyline_strip(&pts, wall.thickness.max(0.06), col);
                    push_strip(&mut out.walls, &verts);
                    out.wall_count += 1;
                }
            }
        }

        // Aperture overlays along the wall direction, slightly wider than the wall.
        let wall_of = |id: &str| lvl.walls.iter().find(|w| w.id == id);
        for win in &lvl.windows {
            let dir = wall_of(&win.wall_id)
                .map(|w| {
                    let d = [w.end[0] - w.start[0], w.end[1] - w.start[1]];
                    let l = (d[0] * d[0] + d[1] * d[1]).sqrt().max(1e-9);
                    [d[0] / l, d[1] / l]
                })
                .unwrap_or([1.0, 0.0]);
            let th = wall_of(&win.wall_id).map_or(0.3, |w| w.thickness) + 0.10;
            let a = [
                win.pos2_d[0] - dir[0] * win.width_m * 0.5,
                win.pos2_d[1] - dir[1] * win.width_m * 0.5,
            ];
            let b = [
                win.pos2_d[0] + dir[0] * win.width_m * 0.5,
                win.pos2_d[1] + dir[1] * win.width_m * 0.5,
            ];
            let verts = expand_polyline_strip(&[to_world(a), to_world(b)], th, COL_WINDOW);
            push_strip(&mut out.apertures, &verts);
            out.aperture_count += 1;
            // Facing normal tick.
            let n = [win.normal[0], win.normal[1]];
            let tip = [win.pos2_d[0] + n[0] * 0.9, win.pos2_d[1] + n[1] * 0.9];
            seg(
                &mut out.hairlines,
                to_world(win.pos2_d),
                to_world(tip),
                COL_NORMAL,
            );
            out.hairline_count += 1;
        }
        for door in &lvl.doors {
            let wall = wall_of(&door.wall_id);
            let dir = wall
                .map(|w| {
                    let d = [w.end[0] - w.start[0], w.end[1] - w.start[1]];
                    let l = (d[0] * d[0] + d[1] * d[1]).sqrt().max(1e-9);
                    [d[0] / l, d[1] / l]
                })
                .unwrap_or([1.0, 0.0]);
            let th = wall.map_or(0.3, |w| w.thickness) + 0.10;
            let col = if door.default_state == "open" {
                COL_DOOR_OPEN
            } else {
                COL_DOOR_CLOSED
            };
            let a = [
                door.pos2_d[0] - dir[0] * door.width_m * 0.5,
                door.pos2_d[1] - dir[1] * door.width_m * 0.5,
            ];
            let b = [
                door.pos2_d[0] + dir[0] * door.width_m * 0.5,
                door.pos2_d[1] + dir[1] * door.width_m * 0.5,
            ];
            let verts = expand_polyline_strip(&[to_world(a), to_world(b)], th, col);
            push_strip(&mut out.apertures, &verts);
            out.aperture_count += 1;
            swing_arc(
                &mut out.arcs,
                &mut out.arc_count,
                lvl,
                door.pos2_d,
                door.width_m,
                &door.hinge_side,
                &door.swing_direction,
                dir,
            );
        }

        // Furniture plates.
        for f in &lvl.furniture {
            let col = match f.los_cover.as_str() {
                "full_cover" => COL_FURN_FULL,
                "low_cover" => COL_FURN_LOW,
                _ => COL_FURN_NONE,
            };
            let ring: Vec<[f64; 2]> = rect_corners(f.pos2_d, f.size2_d, f.rotation_deg).to_vec();
            append_polygon(
                &mut out.furn_pos,
                &mut out.furn_col,
                &mut out.furn_idx,
                &ring,
                col,
            );
        }

        match (drawing, mesh_level) {
            // Lower floors show ONLY through this floor's voids (stairwells, double-height
            // spaces): their eye-height cuts, clipped by this level's floor-coverage raster,
            // dimmer the deeper they are.
            (Some(d), Some(l)) => {
                for (j, below) in d.levels.iter().enumerate().take(active) {
                    let col = if j + 1 == active {
                        COL_GHOST
                    } else {
                        COL_GHOST_DEEP
                    };
                    for s in
                        through_voids(&below.cut_main, &l.surface, l.floor_min_y(), PLAN_CELL_M)
                    {
                        seg(&mut out.hairlines, to_world(s[0]), to_world(s[1]), col);
                        out.hairline_count += 1;
                    }
                }
            }
            // Fallback: every other floor's wall centerlines as ghosts.
            _ => {
                for (i, ghost) in bp.levels.iter().enumerate() {
                    if i == active {
                        continue;
                    }
                    for wall in &ghost.walls {
                        seg(
                            &mut out.hairlines,
                            to_world(wall.start),
                            to_world(wall.end),
                            COL_GHOST,
                        );
                        out.hairline_count += 1;
                    }
                }
            }
        }

        out
    }

    /// Wash palette: GREEN where the observer sees (the old fan's colour, so the disc reads
    /// at a glance), nothing where it does not or beyond the disc — the inverse of the terrain
    /// viewshed's ink-on-hidden language, on purpose: inside a building the walls already carry
    /// the dark, and the operator asked for the green disc back.
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
}

// ═════════════════════════════════════════════════════════════════════════════════════════════
// The page.
// ═════════════════════════════════════════════════════════════════════════════════════════════

/// Building-blueprint viewer + interactive LOS bench (BVH raycast, blueprint-attributed).
#[component]
pub fn BuildingViewerPage() -> impl IntoView {
    let blueprint = RwSignal::new(None::<BuildingBlueprint>);
    // The `.bvh` occlusion sidecar fetched beside the JSON — `Arc` because the parsed sidecar
    // is `!Clone` (`RwSignal::get` clones) and the signal store wants `Send + Sync`.
    let sidecar = RwSignal::new(None::<Arc<BvhSidecar>>);
    let sidecar_err = RwSignal::new(None::<String>);
    // Per-level visibility rasters while the viewshed is on (one `LevelWash` per level, level
    // order); `Arc` for the same reason as the sidecar. `None` = off / nothing to trace.
    let wash = RwSignal::new(None::<Arc<LevelWash>>);
    // The mesh's 2D drawing (section cuts, floor / roof faces, void coverage) — computed once
    // per (blueprint, sidecar); `None` = no sidecar → the blueprint draws everything.
    let drawing = RwSignal::new(None::<Arc<BuildingDrawing>>);
    let load_err = RwSignal::new(None::<String>);
    let engine_err = RwSignal::new(None::<String>);
    let view_floor = RwSignal::new(ViewFloor::Level(0));
    let floors_open = RwSignal::new(false);
    let viewshed_on = RwSignal::new(false);
    let obs = RwSignal::new(RayEnd {
        x: -3.8,
        y: 1.4,
        z: -8.0,
    });
    let tgt = RwSignal::new(RayEnd {
        x: -3.5,
        y: 1.2,
        z: -1.2,
    });
    let los = RwSignal::new(None::<LosResult>);
    let cam = RwSignal::new(Cam {
        tx: geom::ANCHOR[0],
        ty: geom::ANCHOR[1],
        zoom: 4.5,
    });
    let css = RwSignal::new((1200.0f64, 800.0f64));
    let drag = RwSignal::new(Drag::None);
    // T-090.11.6 — the furnished compound (shell + instances, door states inside), the
    // per-level owned cuts of its flattened mesh (recomputed on every door toggle) and the
    // instances-load error line. `None` = the shell-only path (blueprint annotations).
    let compound = RwSignal::new(None::<CompoundBuilding>);
    let compound_err = RwSignal::new(None::<String>);
    let cuts = RwSignal::new(None::<Arc<Vec<LevelCuts>>>);
    let canvas_ref = NodeRef::<leptos::html::Canvas>::new();

    // Pure LOS evaluation — reruns on any ray/blueprint/sidecar change; the wasm host mirrors
    // `los` into the ray lane. Ungated: harmless on native (never mounted there). No sidecar →
    // no verdict: the blueprint alone cannot block a ray.
    Effect::new(move |_| {
        let (o, t) = (obs.get(), tgt.get());
        let sc = sidecar.get();
        blueprint.with(|bp| {
            los.set(match (bp.as_ref(), sc.as_deref()) {
                (Some(bp), Some(occl)) => Some(compound.with(|c| match c.as_ref() {
                    // T-090.11.6: the compound walks doors, glass, foliage and props.
                    Some(c) => c.evaluate_los(Some(bp), [o.x, o.y, o.z], [t.x, t.y, t.z]),
                    None => bp.evaluate_los(occl, [o.x, o.y, o.z], [t.x, t.y, t.z]),
                })),
                _ => None,
            });
        });
    });

    // Multi-floor viewshed — the VIEWED level's visibility disc follows the observer live while
    // the viewshed is on (Alt+click / Alt+drag on A); a floor-rail change recomputes for that
    // level (the Roof view has no eye plane). Radius = the old fan's range, footprint diagonal
    // + 5 m. Pure core compute, native-safe; the wasm host uploads it to the texture lane.
    // No sidecar → no wash: the blueprint alone cannot stop a ray.
    Effect::new(move |_| {
        if !viewshed_on.get() {
            wash.set(None);
            return;
        }
        let o = obs.get();
        let sc = sidecar.get();
        let view = view_floor.get();
        blueprint.with(|bp| {
            wash.set(match (bp.as_ref(), sc.as_deref(), view) {
                (Some(bp), Some(occl), ViewFloor::Level(i)) => {
                    let bb = &bp.overall_footprint.bounding_box2_d;
                    let p = WashParams {
                        radius_m: bb.width_m.hypot(bb.depth_m) + 5.0,
                        ..WashParams::default()
                    };
                    compound
                        .with(|c| match c.as_ref() {
                            Some(c) => level_wash_compound(bp, c, [o.x, o.y, o.z], i, &p),
                            None => level_wash(bp, occl, [o.x, o.y, o.z], i, &p),
                        })
                        .map(Arc::new)
                }
                _ => None,
            });
        });
    });

    // The mesh drawing follows the blueprint + sidecar pair (pure core compute, native-safe).
    Effect::new(move |_| {
        let sc = sidecar.get();
        blueprint.with(|bp| {
            drawing.set(match (bp.as_ref(), sc.as_deref()) {
                (Some(bp), Some(occl)) => Some(Arc::new(building_drawing(bp, occl))),
                _ => None,
            });
        });
    });

    // T-090.11.6 — the flattened compound's owned section cuts per level: door leaves, frames
    // and panes routed to their own lanes. Reruns on every door toggle (the compound signal).
    Effect::new(move |_| {
        let d = drawing.get();
        cuts.set(compound.with(|c| match (c.as_ref(), d.as_deref()) {
            (Some(c), Some(d)) => Some(Arc::new(LevelCuts::for_drawing(c, d))),
            _ => None,
        }));
    });

    #[cfg(target_arch = "wasm32")]
    live::wire(
        canvas_ref,
        blueprint,
        sidecar,
        sidecar_err,
        wash,
        drawing,
        load_err,
        engine_err,
        view_floor,
        floors_open,
        viewshed_on,
        obs,
        tgt,
        los,
        cam,
        css,
        drag,
        compound,
        compound_err,
        cuts,
    );

    // ── DOM overlay derivations ─────────────────────────────────────────────────────────────
    let marker_px = move |end: RayEnd| {
        let c = cam.get();
        geom::world_to_screen(
            geom::to_world([end.x, end.z]),
            c.tx,
            c.ty,
            c.zoom,
            css.get(),
        )
    };
    let obs_px = move || marker_px(obs.get());
    let tgt_px = move || marker_px(tgt.get());
    // Off-floor markers dim to half opacity — the point exists, just not on the viewed plan.
    let on_floor = move |y: f64| {
        blueprint.with(|bp| {
            bp.as_ref().is_none_or(|bp| {
                let (band, _) = view_floor.get().band(bp);
                y >= band[0] && y <= band[1]
            })
        })
    };
    let marker_wrap = move |y: f64| {
        if on_floor(y) {
            "pointer-events-auto absolute z-20 -translate-x-1/2 -translate-y-1/2 cursor-grab"
        } else {
            "pointer-events-auto absolute z-20 -translate-x-1/2 -translate-y-1/2 cursor-grab opacity-50"
        }
    };

    let verdict_view = move || {
        los.get().map(|r| {
            let badge = if r.is_clear {
                view! { <span class="rounded bg-emerald-500/20 px-2 py-0.5 font-bold text-emerald-400">"CLEAR"</span> }.into_any()
            } else {
                view! { <span class="rounded bg-red-500/20 px-2 py-0.5 font-bold text-red-400">"BLOCKED"</span> }.into_any()
            };
            let pct = (r.concealment * 100.0).round();
            let windows = r.window_ids_traversed.join(", ");
            let doors = r.door_ids_traversed.join(", ");
            let canopy = r
                .hits
                .iter()
                .filter(|h| h.kind == LosHitKind::Foliage)
                .map(|h| format!("{} ({:.0}%)", h.id, h.concealment * 100.0))
                .collect::<Vec<_>>()
                .join(", ");
            let blocker = r.hits.last().and_then(|h| match h.kind {
                LosHitKind::DoorLeaf => Some(format!("door leaf {}", h.id)),
                LosHitKind::DoorFrame => Some(format!("door frame {}", h.id)),
                LosHitKind::WindowFrame => Some(format!("window frame {}", h.id)),
                LosHitKind::Prop => Some(format!("prop {}", h.id)),
                _ => None,
            });
            view! {
                <div class="space-y-1">
                    <div class="flex items-center gap-2">{badge}
                        <span class="text-on-surface-variant">{format!("concealment {pct:.0}%")}</span>
                    </div>
                    {(!windows.is_empty()).then(|| view! { <div>"through glass: "<span class="text-cyan-300">{windows.clone()}</span></div> })}
                    {(!doors.is_empty()).then(|| view! { <div>"through door: "<span class="text-emerald-300">{doors.clone()}</span></div> })}
                    {r.blocked_by_wall_id.clone().map(|w| view! { <div>"blocked by "<span class="text-red-300">{w}</span></div> })}
                    {r.hits.last().filter(|h| h.kind == LosHitKind::Roof).map(|h| view! { <div>"blocked by "<span class="text-red-300">{format!("roof @ {:.1} m", h.pos[1])}</span></div> })}
                    {r.hits.last().filter(|h| h.kind == LosHitKind::Solid).map(|h| view! { <div>"blocked by "<span class="text-red-300">{format!("solid @ {:.1} m", h.pos[1])}</span></div> })}
                    {r.hits.last().filter(|h| h.kind == LosHitKind::Window && h.concealment >= 1.0).map(|h| view! { <div>"blocked by "<span class="text-red-300">{format!("frame of {}", h.id)}</span></div> })}
                    {r.cover_furniture_id.clone().map(|f| view! { <div>"cover: "<span class="text-yellow-300">{f}</span></div> })}
                    {(!canopy.is_empty()).then(|| view! { <div>"through canopy: "<span class="text-lime-300">{canopy.clone()}</span></div> })}
                    {blocker.map(|b| view! { <div>"blocked by "<span class="text-red-300">{b}</span></div> })}
                </div>
            }
        })
    };

    // Floor rail — slides out at the building's RIGHT side on building click (stays once open).
    // Vertical stack, bottom→top = Ground..N with Roof topmost; anchored to the building's
    // screen bbox so it follows pan/zoom.
    let floor_rail = move || {
        if !floors_open.get() {
            return None;
        }
        let c = cam.get();
        let size = css.get();
        blueprint.with(|bp| {
            bp.as_ref().map(|bp| {
                let bb = &bp.overall_footprint.bounding_box2_d;
                let anchor = geom::world_to_screen(
                    geom::to_world([bb.max[0] + 1.0, (bb.min[1] + bb.max[1]) * 0.5]),
                    c.tx,
                    c.ty,
                    c.zoom,
                    size,
                );
                let style = format!("left:{}px;top:{}px", anchor[0] + 12.0, anchor[1]);
                // Top→bottom DOM order = Roof, then floors highest→ground.
                let mut rows: Vec<(ViewFloor, String)> = vec![(ViewFloor::Roof, "Roof".to_string())];
                rows.extend(
                    bp.levels
                        .iter()
                        .rev()
                        .map(|l| (ViewFloor::Level(l.level_index), l.name.clone())),
                );
                view! {
                    <div
                        class="pointer-events-auto absolute z-20 flex -translate-y-1/2 flex-col gap-1 rounded-lg border border-border-subtle bg-surface-container/90 p-1 backdrop-blur"
                        style=style
                    >
                        {rows
                            .into_iter()
                            .map(|(vf, name)| {
                                let is_active = move || view_floor.get() == vf;
                                view! {
                                    <button
                                        type="button"
                                        class=move || {
                                            if is_active() {
                                                "rounded-md bg-primary px-3 py-1 text-left text-sm font-medium text-on-primary"
                                            } else {
                                                "rounded-md px-3 py-1 text-left text-sm text-on-surface-variant hover:text-primary"
                                            }
                                        }
                                        on:click=move |_| view_floor.set(vf)
                                    >
                                        {name}
                                    </button>
                                }
                            })
                            .collect_view()}
                    </div>
                }
            })
        })
    };

    // Roof view: profile chip (the only roof data the blueprint carries today).
    let roof_chip = move || {
        if view_floor.get() != ViewFloor::Roof {
            return None;
        }
        blueprint.with(|bp| {
            bp.as_ref().map(|bp| {
                let vp = &bp.vertical_profile;
                let chimney = vp
                    .chimney_height_m
                    .map_or(String::new(), |h| format!(" · chimney {h:.1} m"));
                let label = format!(
                    "{} · eave {:.1} m · ridge {:.1} m{}",
                    vp.roof_type, vp.eave_height_m, vp.ridge_height_m, chimney
                );
                view! {
                    <div class="pointer-events-none absolute left-1/2 top-3 z-20 -translate-x-1/2 rounded-lg border border-border-subtle bg-surface-container/90 px-3 py-1.5 text-xs text-on-surface-variant backdrop-blur">
                        {label}
                    </div>
                }
            })
        })
    };

    // Sill/height badges for the active floor's windows (visible once zoomed past ~16 px/m).
    let window_badges = move || {
        let c = cam.get();
        if c.zoom < 4.0 {
            return Vec::new();
        }
        let size = css.get();
        blueprint.with(|bp| {
            let Some(bp) = bp.as_ref() else { return Vec::new() };
            let ViewFloor::Level(i) = view_floor.get() else { return Vec::new() };
            let Some(lvl) = bp.levels.get(i) else { return Vec::new() };
            lvl.windows
                .iter()
                .map(|w| {
                    let p = geom::world_to_screen(
                        geom::to_world([w.pos2_d[0] + w.normal[0] * 1.3, w.pos2_d[1] + w.normal[1] * 1.3]),
                        c.tx,
                        c.ty,
                        c.zoom,
                        size,
                    );
                    let label = format!(
                        "sill {:.2} · h {:.2}",
                        w.sill_height_m, w.window_height_m
                    );
                    view! {
                        <div
                            class="pointer-events-none absolute z-10 -translate-x-1/2 -translate-y-1/2 rounded bg-cyan-950/80 px-1.5 py-0.5 text-[10px] text-cyan-300"
                            style=move || format!("left:{}px;top:{}px", p[0], p[1])
                        >
                            {label}
                        </div>
                    }
                })
                .collect::<Vec<_>>()
        })
    };

    let slider = move |label: &'static str, end: RwSignal<RayEnd>| {
        view! {
            <label class="flex items-center gap-2 text-xs text-on-surface-variant">
                <span class="w-24">{label}" "{move || format!("{:.2} m", end.get().y)}</span>
                <input
                    type="range"
                    min="0"
                    max="10"
                    step="0.05"
                    class="w-40 accent-primary"
                    prop:value=move || end.get().y.to_string()
                    on:input=move |ev| {
                        if let Ok(v) = event_target_value(&ev).parse::<f64>() {
                            end.update(|e| e.y = v);
                        }
                    }
                />
            </label>
        }
    };

    view! {
        <div class="relative h-full w-full select-none overflow-hidden bg-[#0b0e13]">
            <canvas node_ref=canvas_ref class="absolute inset-0 h-full w-full touch-none"></canvas>

            {floor_rail}
            {roof_chip}
            {window_badges}

            // Observer / target markers.
            <div
                class=move || marker_wrap(obs.get().y)
                style=move || { let p = obs_px(); format!("left:{}px;top:{}px", p[0], p[1]) }
                on:pointerdown=move |ev| { ev.prevent_default(); drag.set(Drag::Observer); }
            >
                <div class="flex h-7 w-7 items-center justify-center rounded-full border-2 border-emerald-400 bg-emerald-500/30 text-[11px] font-bold text-emerald-200">"A"</div>
                <div class="mt-0.5 rounded bg-black/60 px-1 text-center text-[10px] text-emerald-300">{move || format!("{:.1}m", obs.get().y)}</div>
            </div>
            <div
                class=move || marker_wrap(tgt.get().y)
                style=move || { let p = tgt_px(); format!("left:{}px;top:{}px", p[0], p[1]) }
                on:pointerdown=move |ev| { ev.prevent_default(); drag.set(Drag::Target); }
            >
                <div class="flex h-7 w-7 items-center justify-center rounded-full border-2 border-sky-400 bg-sky-500/30 text-[11px] font-bold text-sky-200">"B"</div>
                <div class="mt-0.5 rounded bg-black/60 px-1 text-center text-[10px] text-sky-300">{move || format!("{:.1}m", tgt.get().y)}</div>
            </div>

            // Header / controls.
            <div class="pointer-events-auto absolute left-3 top-3 z-20 max-w-sm space-y-2 rounded-lg border border-border-subtle bg-surface-container/90 p-3 backdrop-blur">
                <div class="text-sm font-bold">"Building Viewer "<span class="text-on-surface-variant">"(debug bench)"</span></div>
                <div class="text-xs text-on-surface-variant">
                    {move || blueprint.with(|bp| bp.as_ref().map(|b| b.label.clone().unwrap_or_else(|| b.prefab_id.clone())).unwrap_or_else(|| "loading…".into()))}
                </div>
                {slider("Observer Y", obs)}
                {slider("Target Y", tgt)}
                <div class="text-[10px] text-on-surface-variant">"drag A/B markers · drag canvas to pan · wheel zooms · click the building for floors · click a door to swing it · Alt+click moves A and fills its viewshed"</div>
                {move || compound.with(|c| c.as_ref().map(|c| {
                    let (open, closed) = c.doors().fold((0usize, 0usize), |(o, k), d| if d.state.is_open() { (o + 1, k) } else { (o, k + 1) });
                    view! { <div class="text-xs text-on-surface-variant">{format!("furnished: {} instances · doors {open} open · {closed} closed", c.instances.len())}</div> }
                }))}
                {move || compound_err.get().map(|e| view! { <div class="rounded bg-amber-500/15 p-2 text-xs text-amber-300">{e}</div> })}
                {move || viewshed_on.get().then(|| {
                    // Which level's wash is on screen — the floor rail swaps it; the roof
                    // view has no eye plane and shows none.
                    let shown = match view_floor.get() {
                        ViewFloor::Level(i) => blueprint.with(|bp| {
                            bp.as_ref()
                                .and_then(|b| b.levels.iter().find(|l| l.level_index == i))
                                .map_or_else(|| format!("level {i}"), |l| l.name.clone())
                        }),
                        ViewFloor::Roof => "no wash on the roof view".to_string(),
                    };
                    view! {
                    <div class="flex items-center gap-2 rounded bg-emerald-500/10 px-2 py-1 text-xs text-emerald-300">
                        {format!("viewshed from A · {shown}")}
                        <button
                            type="button"
                            class="rounded px-1 text-on-surface-variant hover:text-red-300"
                            on:click=move |_| viewshed_on.set(false)
                        >
                            "✕ clear"
                        </button>
                    </div>
                    }
                })}
                {move || load_err.get().map(|e| view! { <div class="rounded bg-red-500/15 p-2 text-xs text-red-300">{e}</div> })}
                {move || sidecar_err.get().map(|e| view! { <div class="rounded bg-amber-500/15 p-2 text-xs text-amber-300">{e}</div> })}
                {move || engine_err.get().map(|e| view! { <div class="rounded bg-red-500/15 p-2 text-xs text-red-300">{e}</div> })}
            </div>

            // Verdict.
            <div class="pointer-events-none absolute bottom-3 right-3 z-20 min-w-56 rounded-lg border border-border-subtle bg-surface-container/90 p-3 text-xs backdrop-blur">
                {verdict_view}
            </div>

            // Legend.
            <div class="pointer-events-none absolute bottom-3 left-3 z-20 space-y-0.5 rounded-lg border border-border-subtle bg-surface-container/90 p-2 text-[10px] text-on-surface-variant backdrop-blur">
                <div><span class="mr-1 inline-block h-2 w-4 bg-[#cdd4e0]"></span>"wall (exterior)"</div>
                <div><span class="mr-1 inline-block h-2 w-4 bg-[#34c7f2]"></span>"window / glass"</div>
                <div><span class="mr-1 inline-block h-2 w-4 bg-[#4dd973]"></span>"open door · ray clear"</div>
                <div><span class="mr-1 inline-block h-2 w-4 bg-[#f2a133]"></span>"closed door"</div>
                <div><span class="mr-1 inline-block h-2 w-4 bg-[#9ed940]"></span>"canopy · ray through leaves"</div>
                <div><span class="mr-1 inline-block h-2 w-4 bg-[#8c6640]"></span>"tree trunk"</div>
                <div><span class="mr-1 inline-block h-2 w-4 bg-[#808c9e]"></span>"prop footprint"</div>
                <div><span class="mr-1 inline-block h-2 w-4 bg-[#ebcc40]"></span>"low cover · ray past cover"</div>
                <div><span class="mr-1 inline-block h-2 w-4 bg-[#e6574a]"></span>"full cover · ray blocked"</div>
                <div><span class="mr-1 inline-block h-2 w-4 bg-[#7059b3]"></span>"stairs (transparent treads)"</div>
                <div><span class="mr-1 inline-block h-2 w-4 bg-gradient-to-r from-[#2a3854] to-[#d1dbf2]"></span>"roof height (eave → ridge) · "<span class="text-[#f2a133]">"chimney"</span></div>
                <div><span class="mr-1 inline-block h-2 w-4 bg-gradient-to-r from-[#1a212e] to-[#3d4c66]"></span>"floor plate (scanned cells) · "<span class="text-[#739eb8]">"ring edge"</span></div>
            </div>
        </div>
    }
}

// ═════════════════════════════════════════════════════════════════════════════════════════════
// wasm host: engine mount, lane uploads, listeners.
// ═════════════════════════════════════════════════════════════════════════════════════════════
#[cfg(target_arch = "wasm32")]
mod live {
    use super::super::building_interior::{self, InteriorLanes, LevelCuts};
    use super::{geom, Cam, Drag, RayEnd, ViewFloor, DEFAULT_PREFAB_PATH};
    use leptos::prelude::*;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;
    use std::sync::Arc;
    use wasm_bindgen::prelude::*;
    use wasm_bindgen::JsCast;
    use website_map_engine::frame::engine::RenderEngine;
    use website_map_engine::frame::RafPump;
    use website_map_engine::overlay::lanes::role_id;
    use website_map_engine::spatial::bvh::sidecar::BvhSidecar;
    use website_map_engine::spatial::los::interior::wash::LevelWash;
    use website_map_engine::world::architecture::blueprint::attribution_1::LosResult;
    use website_map_engine::world::architecture::blueprint::structure::BuildingBlueprint;
    use website_map_engine::world::architecture::compound::assembly::CompoundBuilding;
    use website_map_engine::world::architecture::compound::instances::InstancesFile;
    use website_map_engine::world::architecture::section::cutter::BuildingDrawing;

    type EngineHandle = Rc<RefCell<Option<RenderEngine>>>;

    fn sync_cam(e: &RenderEngine, cam: RwSignal<Cam>) {
        cam.set(Cam {
            tx: e.target_x(),
            ty: e.target_y(),
            zoom: e.zoom(),
        });
    }

    /// T-090.11.6 — every static lane of the bench on its OWN role (`InteriorLanes::ROLES`);
    /// the probe (`INTERIOR_PROBE`) is `upload_ray`'s. An empty payload drops its lane.
    fn upload_static(
        e: &mut RenderEngine,
        bp: &BuildingBlueprint,
        drawing: Option<&BuildingDrawing>,
        compound: Option<&CompoundBuilding>,
        cuts: Option<&[LevelCuts]>,
        view: ViewFloor,
    ) {
        let l: InteriorLanes =
            building_interior::build_interior_lanes(bp, drawing, compound, cuts, view);
        e.upload_polygon_mesh(
            role_id::INTERIOR_SLABS,
            &l.slabs_pos,
            &l.slabs_col,
            &l.slabs_idx,
            1,
            true,
        );
        e.upload_polygon_mesh(
            role_id::INTERIOR_FURNITURE,
            &l.furniture_pos,
            &l.furniture_col,
            &l.furniture_idx,
            1,
            true,
        );
        e.upload_hairline_segments(
            role_id::INTERIOR_FURNITURE_OUTLINE,
            &l.furniture_outline,
            l.furniture_outline_count,
            true,
        );
        e.upload_strip_tris(role_id::INTERIOR_WALLS, &l.walls, l.wall_count, true);
        e.upload_hairline_segments(
            role_id::INTERIOR_WALLS_OUTLINE,
            &l.walls_outline,
            l.walls_outline_count,
            true,
        );
        e.upload_strip_tris(role_id::INTERIOR_PORTALS, &l.portals, l.portal_count, true);
        e.upload_hairline_segments(
            role_id::INTERIOR_PORTALS_OUTLINE,
            &l.portals_outline,
            l.portals_outline_count,
            true,
        );
        e.upload_strip_tris(role_id::INTERIOR_GLAZING, &l.glazing, l.glazing_count, true);
        e.upload_hairline_segments(
            role_id::INTERIOR_GLAZING_OUTLINE,
            &l.glazing_outline,
            l.glazing_outline_count,
            true,
        );
        e.upload_hairline_segments(role_id::INTERIOR_STAIRS, &l.stairs, l.stairs_count, true);
        e.upload_polygon_mesh(
            role_id::SCENE_VEGETATION,
            &l.vegetation_pos,
            &l.vegetation_col,
            &l.vegetation_idx,
            1,
            true,
        );
        e.upload_hairline_segments(
            role_id::SCENE_VEGETATION_OUTLINE,
            &l.vegetation_outline,
            l.vegetation_outline_count,
            true,
        );
        e.mark_dirty();
    }

    fn upload_ray(
        e: &mut RenderEngine,
        obs: RayEnd,
        tgt: RayEnd,
        los: &LosResult,
        band: [f64; 2],
        band_last: bool,
    ) {
        let (packed, n) = building_interior::build_ray_lane(
            [obs.x, obs.y, obs.z],
            [tgt.x, tgt.y, tgt.z],
            &los.hits,
            los.is_clear,
            band,
            band_last,
        );
        e.upload_strip_tris(role_id::INTERIOR_PROBE, &packed, n, true);
        e.mark_dirty();
    }

    /// The viewed level's wash → the engine's single viewshed texture slot; `None` (viewshed
    /// off, Roof view, no sidecar) clears the lane — a wash with nothing to stop it would be a
    /// lie.
    fn upload_wash(e: &mut RenderEngine, wash: Option<&LevelWash>) {
        match wash {
            Some(w) => {
                let t = geom::wash_texture(w);
                if let Err(err) = e.viewshed_upload(
                    t.min_x,
                    t.min_y,
                    t.max_x,
                    t.max_y,
                    t.tex_w,
                    t.tex_h,
                    &t.rgba,
                    t.stride_bytes,
                ) {
                    let err: JsValue = err.into();
                    web_sys::console::warn_2(
                        &"building-viewer: viewshed wash upload failed".into(),
                        &err,
                    );
                    e.viewshed_clear();
                }
            }
            None => e.viewshed_clear(),
        }
        e.mark_dirty();
    }

    fn prefab_path() -> String {
        web_sys::window()
            .and_then(|w| w.location().search().ok())
            .and_then(|s| {
                web_sys::UrlSearchParams::new_with_str(&s)
                    .ok()
                    .and_then(|p| p.get("prefab"))
            })
            .filter(|p| !p.is_empty())
            .unwrap_or_else(|| DEFAULT_PREFAB_PATH.to_string())
    }

    /// One query parameter, if present and non-empty.
    fn query(name: &str) -> Option<String> {
        web_sys::window()
            .and_then(|w| w.location().search().ok())
            .and_then(|s| {
                web_sys::UrlSearchParams::new_with_str(&s)
                    .ok()
                    .and_then(|p| p.get(name))
            })
            .filter(|v| !v.is_empty())
    }

    /// `?a=x,y,z` / `?b=x,y,z` — the ray ends in building-local metres (a reproducible LOS state
    /// for screenshots and bug reports).
    fn ray_end_query(name: &str) -> Option<RayEnd> {
        let v = query(name)?;
        let mut it = v.split(',').map(|s| s.trim().parse::<f64>().ok());
        let (x, y, z) = (it.next()??, it.next()??, it.next()??);
        (x.is_finite() && y.is_finite() && z.is_finite()).then_some(RayEnd { x, y, z })
    }

    /// `?scene=1` — also load `<slug>.scene.json` (hand-placed exterior trees) into the compound.
    fn scene_mode() -> bool {
        web_sys::window()
            .and_then(|w| w.location().search().ok())
            .and_then(|s| {
                web_sys::UrlSearchParams::new_with_str(&s)
                    .ok()
                    .and_then(|p| p.get("scene"))
            })
            .is_some_and(|v| matches!(v.as_str(), "1" | "true" | "on"))
    }

    async fn fetch_bytes(url: &str) -> Result<Vec<u8>, String> {
        match gloo_net::http::Request::get(url).send().await {
            Ok(resp) if resp.ok() => resp
                .binary()
                .await
                .map_err(|e| format!("{url}: read failed — {e}")),
            Ok(resp) => Err(format!("{url}: HTTP {}", resp.status())),
            Err(e) => Err(format!("{url}: {e}")),
        }
    }

    /// T-090.11.6 — `<slug>.instances.json` + every BLAS it references (paths relative to the
    /// prefabs root, the parent of `buildings/`) assembled onto the shell; with `scene`, the
    /// `<slug>.scene.json` trees too. Returns the compound and a non-fatal warning (scene file
    /// missing).
    async fn load_compound(
        json_path: &str,
        shell: Arc<BvhSidecar>,
        scene: bool,
    ) -> Result<(CompoundBuilding, Option<String>), String> {
        let stem = json_path
            .strip_suffix(".json")
            .ok_or_else(|| format!("{json_path}: not a .json path"))?;
        let root = stem
            .rsplit_once('/')
            .and_then(|(dir, _)| dir.rsplit_once('/'))
            .map_or_else(|| "/".to_string(), |(parent, _)| format!("{parent}/"));
        let url = format!("{stem}.instances.json");
        let bytes = fetch_bytes(&url).await?;
        let file: InstancesFile =
            serde_json::from_slice(&bytes).map_err(|e| format!("{url}: parse failed — {e}"))?;
        let mut records = file.instances;
        let mut warning = None;
        if scene {
            let surl = format!("{stem}.scene.json");
            match fetch_bytes(&surl).await {
                Ok(b) => match serde_json::from_slice::<InstancesFile>(&b) {
                    Ok(sf) => records.extend(sf.instances),
                    Err(e) => warning = Some(format!("{surl}: parse failed — {e} — scene off")),
                },
                Err(e) => warning = Some(format!("{e} — scene off")),
            }
        }
        let mut paths: Vec<String> = Vec::new();
        for r in &records {
            if !paths.contains(&r.blas) {
                paths.push(r.blas.clone());
            }
        }
        let fetched = futures::future::join_all(paths.iter().map(|p| {
            let url = format!("{root}{p}");
            async move {
                let res = fetch_bytes(&url).await;
                (url, res)
            }
        }))
        .await;
        let mut map: HashMap<String, Arc<BvhSidecar>> = HashMap::new();
        for (p, (url, res)) in paths.iter().zip(fetched) {
            let b = res?;
            let sc =
                BvhSidecar::parse(&b).map_err(|e| format!("{url}: BLAS parse failed — {e}"))?;
            map.insert(p.clone(), Arc::new(sc));
        }
        let c =
            CompoundBuilding::assemble(shell, &records, &map).map_err(|e| format!("{url}: {e}"))?;
        Ok((c, warning))
    }

    /// Boots the render engine on the bench canvas and wires the whole live surface to it: the
    /// blueprint, sidecar and compound fetches, the floor rail, the draggable observer and target,
    /// the viewshed wash upload, and the pointer, wheel and keyboard handling.
    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    pub fn wire(
        canvas_ref: NodeRef<leptos::html::Canvas>,
        blueprint: RwSignal<Option<BuildingBlueprint>>,
        sidecar: RwSignal<Option<Arc<BvhSidecar>>>,
        sidecar_err: RwSignal<Option<String>>,
        wash: RwSignal<Option<Arc<LevelWash>>>,
        drawing: RwSignal<Option<Arc<BuildingDrawing>>>,
        load_err: RwSignal<Option<String>>,
        engine_err: RwSignal<Option<String>>,
        view_floor: RwSignal<ViewFloor>,
        floors_open: RwSignal<bool>,
        viewshed_on: RwSignal<bool>,
        obs: RwSignal<RayEnd>,
        tgt: RwSignal<RayEnd>,
        los: RwSignal<Option<LosResult>>,
        cam: RwSignal<Cam>,
        css: RwSignal<(f64, f64)>,
        drag: RwSignal<Drag>,
        compound: RwSignal<Option<CompoundBuilding>>,
        compound_err: RwSignal<Option<String>>,
        cuts: RwSignal<Option<Arc<Vec<LevelCuts>>>>,
    ) {
        let engine: EngineHandle = Rc::new(RefCell::new(None));
        let disposed = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        // T-090.11.6 URL flags: reproducible ray ends; `?force=webgl` = the headless capture
        // backend (the editor's convention — software WebGPU wedges headless Chrome).
        if let Some(a) = ray_end_query("a") {
            obs.set(a);
        }
        if let Some(b) = ray_end_query("b") {
            tgt.set(b);
        }
        let force_webgl = query("force").is_some_and(|v| v == "webgl");
        let doors_open = query("doors").is_some_and(|v| matches!(v.as_str(), "open" | "1"));
        let fitted = Rc::new(std::cell::Cell::new(false));
        // Bumped once the engine lands so the upload effects rerun — without it, a blueprint that
        // arrives before `RenderEngine::create` resolves would never get its first upload.
        let engine_ready = RwSignal::new(false);

        // Blueprint + sidecar fetch (once). The sidecar is the JSON path with `.bvh` in place of
        // `.json`; a missing one (the hand-authored Green/plain assets ship none) is not an error
        // for the plan view — LOS just stays off, and the header says why.
        leptos::task::spawn_local(async move {
            let path = prefab_path();
            match gloo_net::http::Request::get(&path).send().await {
                Ok(resp) if resp.ok() => match resp.text().await {
                    Ok(body) => match serde_json::from_str::<BuildingBlueprint>(&body) {
                        Ok(bp) => blueprint.set(Some(bp)),
                        Err(e) => load_err.set(Some(format!("{path}: parse failed — {e}"))),
                    },
                    Err(e) => load_err.set(Some(format!("{path}: read failed — {e}"))),
                },
                Ok(resp) => load_err.set(Some(format!("{path}: HTTP {}", resp.status()))),
                Err(e) => load_err.set(Some(format!("{path}: {e}"))),
            }

            let Some(url) = path.strip_suffix(".json").map(|s| format!("{s}.bvh")) else {
                sidecar_err.set(Some(format!(
                    "{path}: not a .json path, no occlusion sidecar — LOS, viewshed and mesh drawing off (blueprint fallback)"
                )));
                return;
            };
            let outcome = match gloo_net::http::Request::get(&url).send().await {
                Ok(resp) if resp.ok() => match resp.binary().await {
                    Ok(bytes) => BvhSidecar::parse(&bytes)
                        .map_err(|e| format!("{url}: sidecar parse failed — {e} — LOS, viewshed and mesh drawing off (blueprint fallback)")),
                    Err(e) => Err(format!("{url}: read failed — {e} — LOS, viewshed and mesh drawing off (blueprint fallback)")),
                },
                Ok(resp) => Err(format!(
                    "{url}: HTTP {} — no occlusion sidecar, LOS, viewshed and mesh drawing off (blueprint fallback)",
                    resp.status()
                )),
                Err(e) => Err(format!("{url}: {e} — LOS, viewshed and mesh drawing off (blueprint fallback)")),
            };
            match outcome {
                Ok(sc) => {
                    let shell = Arc::new(sc);
                    sidecar.set(Some(Arc::clone(&shell)));
                    // T-090.11.6: the instances + BLAS closure on top of the shell.
                    match load_compound(&path, shell, scene_mode()).await {
                        Ok((mut c, warning)) => {
                            if doors_open {
                                let ids: Vec<String> =
                                    c.doors().map(|d| d.record.id.clone()).collect();
                                for id in ids {
                                    c.set_door(
                                        &id,
                                        website_map_engine::world::architecture::compound::doors::DoorState::OPEN,
                                    );
                                }
                            }
                            compound_err.set(warning);
                            compound.set(Some(c));
                        }
                        Err(msg) => compound_err.set(Some(format!(
                            "{msg} — shell-only bench (no doors, glass, furniture)"
                        ))),
                    }
                }
                Err(msg) => sidecar_err.set(Some(msg)),
            }
        });

        // Engine mount once the canvas exists.
        Effect::new({
            let engine = engine.clone();
            let disposed = disposed.clone();
            move |_| {
                let Some(canvas) = canvas_ref.get() else {
                    return;
                };
                if engine.borrow().is_some() || disposed.load(std::sync::atomic::Ordering::Relaxed)
                {
                    return;
                }
                let canvas: web_sys::HtmlCanvasElement = canvas;
                let rect = canvas.get_bounding_client_rect();
                let (cw, ch) = (rect.width().max(64.0), rect.height().max(64.0));
                let dpr = web_sys::window().map_or(1.0, |w| w.device_pixel_ratio());
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                {
                    canvas.set_width(((cw * dpr + 0.5).floor().max(1.0)) as u32);
                    canvas.set_height(((ch * dpr + 0.5).floor().max(1.0)) as u32);
                }
                css.set((cw, ch));
                let engine = engine.clone();
                let disposed = disposed.clone();
                leptos::task::spawn_local(async move {
                    match RenderEngine::create(canvas, force_webgl).await {
                        Ok(mut e) => {
                            let _ = e.resize(cw, ch, dpr);
                            // Camera roam box: a generous pad around the anchor-placed building.
                            e.set_camera_bounds(
                                geom::ANCHOR[0] - 200.0,
                                geom::ANCHOR[1] - 200.0,
                                geom::ANCHOR[0] + 200.0,
                                geom::ANCHOR[1] + 200.0,
                            );
                            e.set_view(geom::ANCHOR[0], geom::ANCHOR[1], 4.5);
                            e.hide_calibration();
                            e.disable_frame_timing();
                            e.set_continuous_render(false);
                            let (r, g, b) = (geom::COL_BG[0], geom::COL_BG[1], geom::COL_BG[2]);
                            e.set_clear_color(r, g, b);
                            sync_cam(&e, cam);
                            *engine.borrow_mut() = Some(e);
                            // The shared frame pump, with no per-frame hook: this page has no
                            // HUD and no signal to publish, so render → poll → repeat until
                            // dispose is the whole loop.
                            RafPump::new(engine.clone(), disposed.clone()).start();
                            engine_ready.set(true);
                        }
                        Err(err) => {
                            engine_err.set(Some(format!("engine create failed: {err:?}")));
                        }
                    }
                });
            }
        });

        // Static lanes: (blueprint, view floor) → upload; first arrival also fits the camera.
        Effect::new({
            let engine = engine.clone();
            let fitted = fitted.clone();
            move |_| {
                if !engine_ready.get() {
                    return;
                }
                let view = view_floor.get();
                let d = drawing.get();
                let cu = cuts.get();
                blueprint.with(|bp| {
                    let Some(bp) = bp.as_ref() else { return };
                    if let Ok(mut guard) = engine.try_borrow_mut() {
                        if let Some(e) = guard.as_mut() {
                            if !fitted.get() {
                                let (tx, ty, zoom) = geom::fit_camera(bp, css.get_untracked());
                                e.set_view(tx, ty, zoom);
                                sync_cam(e, cam);
                                fitted.set(true);
                            }
                            compound.with(|c| {
                                upload_static(
                                    e,
                                    bp,
                                    d.as_deref(),
                                    c.as_ref(),
                                    cu.as_deref().map(Vec::as_slice),
                                    view,
                                );
                            });
                        }
                    }
                });
            }
        });

        // Ray lane: follows the LOS result, clipped to the viewed floor's band.
        Effect::new({
            let engine = engine.clone();
            move |_| {
                if !engine_ready.get() {
                    return;
                }
                let Some(r) = los.get() else { return };
                let view = view_floor.get();
                let (o, t) = (obs.get_untracked(), tgt.get_untracked());
                blueprint.with_untracked(|bp| {
                    let Some(bp) = bp.as_ref() else { return };
                    let (band, band_last) = view.band(bp);
                    if let Ok(mut guard) = engine.try_borrow_mut() {
                        if let Some(e) = guard.as_mut() {
                            upload_ray(e, o, t, &r, band, band_last);
                        }
                    }
                });
            }
        });

        // Wash lane: mirrors the page's `wash` signal (already the viewed level's disc; the
        // page recomputes it on observer / floor-rail change, `None` clears).
        Effect::new({
            let engine = engine.clone();
            move |_| {
                if !engine_ready.get() {
                    return;
                }
                let w = wash.get();
                if let Ok(mut guard) = engine.try_borrow_mut() {
                    if let Some(e) = guard.as_mut() {
                        upload_wash(e, w.as_deref());
                    }
                }
            }
        });

        // Pointer + wheel listeners on the canvas; move/up on the window (drag escapes the rect).
        Effect::new({
            let engine = engine.clone();
            move |_| {
                let Some(canvas) = canvas_ref.get() else {
                    return;
                };
                let canvas: web_sys::HtmlCanvasElement = canvas;

                let down = {
                    Closure::wrap(Box::new(move |ev: web_sys::PointerEvent| {
                        // Alt+LMB: teleport observer A here and light its viewshed; the held
                        // drag keeps moving A (fill follows live).
                        if ev.alt_key() && ev.button() == 0 {
                            ev.prevent_default();
                            let c = cam.get_untracked();
                            let size = css.get_untracked();
                            let w = geom::screen_to_world(
                                [f64::from(ev.client_x()), f64::from(ev.client_y())],
                                c.tx,
                                c.ty,
                                c.zoom,
                                size,
                            );
                            let l = geom::from_world(w);
                            obs.update(|e| {
                                e.x = l[0];
                                e.z = l[1];
                            });
                            viewshed_on.set(true);
                            drag.set(Drag::Observer);
                            return;
                        }
                        if drag.get_untracked() == Drag::None {
                            drag.set(Drag::Pan);
                        }
                    }) as Box<dyn FnMut(_)>)
                };
                canvas.set_onpointerdown(Some(down.as_ref().unchecked_ref()));
                down.forget();

                let wheel = {
                    let engine = engine.clone();
                    Closure::wrap(Box::new(move |ev: web_sys::WheelEvent| {
                        ev.prevent_default();
                        if let Ok(mut guard) = engine.try_borrow_mut() {
                            if let Some(e) = guard.as_mut() {
                                e.zoom_at(
                                    -ev.delta_y() * 0.0015,
                                    ev.offset_x().into(),
                                    ev.offset_y().into(),
                                );
                                sync_cam(e, cam);
                            }
                        }
                    }) as Box<dyn FnMut(_)>)
                };
                canvas.set_onwheel(Some(wheel.as_ref().unchecked_ref()));
                wheel.forget();

                let Some(win) = web_sys::window() else { return };
                let last = Rc::new(std::cell::Cell::new((0.0f64, 0.0f64)));
                let moved = Rc::new(std::cell::Cell::new(0.0f64));

                let mv = {
                    let engine = engine.clone();
                    let last = last.clone();
                    let moved = moved.clone();
                    Closure::wrap(Box::new(move |ev: web_sys::PointerEvent| {
                        let (px, py) = (f64::from(ev.client_x()), f64::from(ev.client_y()));
                        let (lx, ly) = last.get();
                        let (dx, dy) = (px - lx, py - ly);
                        last.set((px, py));
                        let mode = drag.get_untracked();
                        if mode == Drag::None {
                            return;
                        }
                        moved.set(moved.get() + dx.abs() + dy.abs());
                        match mode {
                            Drag::Pan => {
                                if let Ok(mut guard) = engine.try_borrow_mut() {
                                    if let Some(e) = guard.as_mut() {
                                        e.pan(dx, dy);
                                        sync_cam(e, cam);
                                    }
                                }
                            }
                            Drag::Observer | Drag::Target => {
                                let c = cam.get_untracked();
                                let size = css.get_untracked();
                                let w = geom::screen_to_world([px, py], c.tx, c.ty, c.zoom, size);
                                let l = geom::from_world(w);
                                let s = if mode == Drag::Observer { obs } else { tgt };
                                s.update(|e| {
                                    e.x = l[0];
                                    e.z = l[1];
                                });
                            }
                            _ => {}
                        }
                    }) as Box<dyn FnMut(_)>)
                };
                let _ = win
                    .add_event_listener_with_callback("pointermove", mv.as_ref().unchecked_ref());
                mv.forget();

                let up = {
                    let moved = moved.clone();
                    Closure::wrap(Box::new(move |ev: web_sys::PointerEvent| {
                        let mode = drag.get_untracked();
                        drag.set(Drag::None);
                        // A pan that never moved = a CLICK: open the floor selector when it
                        // lands inside the building footprint.
                        if mode == Drag::Pan && moved.get() < 4.0 {
                            let c = cam.get_untracked();
                            let size = css.get_untracked();
                            let w = geom::screen_to_world(
                                [f64::from(ev.client_x()), f64::from(ev.client_y())],
                                c.tx,
                                c.ty,
                                c.zoom,
                                size,
                            );
                            let l = geom::from_world(w);
                            // T-090.11.6: a click on a door leaf (or its closed footprint) swings it;
                            // LOS, wash, cuts and lanes follow through the compound signal.
                            let toggled = blueprint.with_untracked(|bp| {
                                let Some(bp) = bp.as_ref() else { return false };
                                let (band, _) = view_floor.get_untracked().band(bp);
                                let id = compound.with_untracked(|c| {
                                    c.as_ref()
                                        .and_then(|c| building_interior::door_at(c, l, band))
                                });
                                match id {
                                    Some(id) => {
                                        compound.update(|c| {
                                            if let Some(c) = c.as_mut() {
                                                if let Some(s) = c.door_state(&id) {
                                                    c.set_door(&id, s.toggled());
                                                }
                                            }
                                        });
                                        true
                                    }
                                    None => false,
                                }
                            });
                            if !toggled {
                                blueprint.with_untracked(|bp| {
                                    if let Some(bp) = bp.as_ref() {
                                        if geom::point_in_polygon(
                                            l,
                                            &bp.overall_footprint.polygon2_d,
                                        ) {
                                            floors_open.set(true);
                                        }
                                    }
                                });
                            }
                        }
                        moved.set(0.0);
                    }) as Box<dyn FnMut(_)>)
                };
                let _ =
                    win.add_event_listener_with_callback("pointerup", up.as_ref().unchecked_ref());
                up.forget();

                // Seed `last` on every pointerdown anywhere (markers set their own drag mode
                // before this bubbles).
                let seed = {
                    let last = last.clone();
                    let moved = moved.clone();
                    Closure::wrap(Box::new(move |ev: web_sys::PointerEvent| {
                        last.set((f64::from(ev.client_x()), f64::from(ev.client_y())));
                        moved.set(0.0);
                    }) as Box<dyn FnMut(_)>)
                };
                let _ = win
                    .add_event_listener_with_callback("pointerdown", seed.as_ref().unchecked_ref());
                seed.forget();
            }
        });

        // Window resize → engine resize + css mirror.
        Effect::new({
            let engine = engine.clone();
            move |_| {
                let Some(_canvas) = canvas_ref.get() else {
                    return;
                };
                let Some(win) = web_sys::window() else { return };
                let engine = engine.clone();
                let resize = Closure::wrap(Box::new(move || {
                    let Some(canvas) = canvas_ref.get_untracked() else {
                        return;
                    };
                    let canvas: web_sys::HtmlCanvasElement = canvas;
                    let rect = canvas.get_bounding_client_rect();
                    let (cw, ch) = (rect.width().max(64.0), rect.height().max(64.0));
                    let dpr = web_sys::window().map_or(1.0, |w| w.device_pixel_ratio());
                    css.set((cw, ch));
                    if let Ok(mut guard) = engine.try_borrow_mut() {
                        if let Some(e) = guard.as_mut() {
                            let _ = e.resize(cw, ch, dpr);
                            sync_cam(e, cam);
                        }
                    }
                }) as Box<dyn FnMut()>);
                let _ =
                    win.add_event_listener_with_callback("resize", resize.as_ref().unchecked_ref());
                resize.forget();
            }
        });

        // Dispose on unmount.
        on_cleanup(move || disposed.store(true, std::sync::atomic::Ordering::Relaxed));
    }
}
