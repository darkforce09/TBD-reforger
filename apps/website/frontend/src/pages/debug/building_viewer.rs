//! `/debug/building-viewer` — the building-blueprint TEST BENCH (Phase A of the blueprint
//! extraction program; plan `below-is-a-prompt-valiant-piglet`). Public route, no nav entry:
//! reachable by URL only, `?prefab=<map-assets path>` overrides the default FarmHouse golden.
//!
//! Purpose: a hyper-focused single-prefab instrument for eyeballing what the Workbench extractor
//! produced — floor plates, thickness walls, apertures, furniture cover, stairs — and for driving
//! the 2.5D `evaluate_los` raycaster interactively (draggable observer/target + elevation
//! sliders, ray colored by the ordered [`LosHit`] trace). The blueprint JSON is fetched from
//! `/map-assets/everon/prefabs/buildings/…` (served by the API, proxied by Trunk in dev).
//!
//! ── Architecture ───────────────────────────────────────────────────────────────────────────────
//! Rendering is the REAL wgpu engine (`map_engine_render::RenderEngine`) on a page canvas — the
//! same crate the mission editor mounts — but with none of the editor's boot machinery (no IDB,
//! no hydrate, no DEM/sat/world loaders). The blueprint becomes plain vector lanes through the
//! generic upload API (`upload_polygon_mesh` / `upload_strip_tris` / `upload_hairline_segments`
//! with `role_id::*` constants — never re-copied integers, see draw_order.rs):
//!
//! | lane (draw order ↑)      | content |
//! |--------------------------|---------|
//! | `LANDCOVER` (poly)       | active floor plate + stairs plate |
//! | `CONTOURS` (hairline)    | ghost floors (centerlines), door swing arcs, window normals, stair hatch |
//! | `AIRFIELD_APRON` (poly)  | furniture plates (cover-class colored) |
//! | `ROADS_CASING` (strip)   | active-floor walls at true thickness |
//! | `ROADS` (strip)          | window / door aperture overlays on the walls |
//! | `MISSION_ZONES` (strip)  | the LOS ray, split + colored at each `LosHit`, plus event dots |
//!
//! The building sits at world ANCHOR (6400, 6400) with blueprint +z (game north) mapped UP on
//! screen (`to_world` flips z; the engine's y axis points down). Camera = the engine's own ortho
//! camera (`zoom` = log2 px/m, max 6 → 64 px/m ≈ 18 px for a 0.28 m log wall).
//!
//! Everything decidable is a pure function in [`geom`] (world mapping, camera fit, lane
//! tessellation, ray span coloring, point-in-polygon) so native `cargo test -p website-frontend`
//! proves the geometry with no browser — the wasm block only wires signals, listeners and the
//! engine. This is the `los_tool.rs` idiom.
#![allow(dead_code)] // native build: the wasm host wires the live path; tests pin the pure core.

use leptos::prelude::*;
use map_engine_core::building_blueprint::{BuildingBlueprint, LosResult};

/// Default blueprint when no `?prefab=` override is present — the FarmHouse golden.
const DEFAULT_PREFAB_PATH: &str = "/map-assets/everon/prefabs/buildings/FarmHouse_E_1L01.json";

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
    use map_engine_core::building_blueprint::{
        clip_t_to_band, BuildingBlueprint, BuildingLevel, LosHit, LosHitKind,
    };
    use map_engine_core::geometry::polyline_strip::{expand_polyline_strip, StripVertex};
    use map_engine_core::geometry::triangulate::triangulate_simple;

    /// The building is placed at the engine's world anchor so f32 lane coords stay tiny.
    pub const ANCHOR: [f64; 2] = [6400.0, 6400.0];

    // Palette (linear RGBA) — tuned for the site's dark surface.
    pub const COL_BG: [f64; 3] = [0.043, 0.055, 0.075];
    pub const COL_FLOOR: [f32; 4] = [0.16, 0.20, 0.26, 0.85];
    pub const COL_STAIRS: [f32; 4] = [0.44, 0.36, 0.70, 0.75];
    pub const COL_WALL_EXT: [f32; 4] = [0.80, 0.83, 0.88, 1.0];
    pub const COL_WALL_INT: [f32; 4] = [0.55, 0.58, 0.66, 1.0];
    pub const COL_GHOST: [f32; 4] = [0.55, 0.60, 0.70, 0.35];
    pub const COL_WINDOW: [f32; 4] = [0.20, 0.78, 0.95, 1.0];
    pub const COL_DOOR_OPEN: [f32; 4] = [0.30, 0.85, 0.45, 1.0];
    pub const COL_DOOR_CLOSED: [f32; 4] = [0.95, 0.63, 0.20, 1.0];
    pub const COL_FURN_LOW: [f32; 4] = [0.92, 0.80, 0.25, 0.85];
    pub const COL_FURN_FULL: [f32; 4] = [0.90, 0.34, 0.28, 0.90];
    pub const COL_FURN_NONE: [f32; 4] = [0.45, 0.48, 0.55, 0.55];
    pub const COL_ARC: [f32; 4] = [0.70, 0.75, 0.85, 0.65];
    pub const COL_NORMAL: [f32; 4] = [0.20, 0.78, 0.95, 0.80];
    pub const COL_HATCH: [f32; 4] = [0.75, 0.70, 0.95, 0.55];
    pub const RAY_CLEAR: [f32; 4] = [0.25, 0.90, 0.40, 1.0];
    pub const RAY_GLASS: [f32; 4] = [0.20, 0.80, 0.95, 1.0];
    pub const RAY_COVER: [f32; 4] = [0.95, 0.85, 0.20, 1.0];
    pub const RAY_BLOCKED: [f32; 4] = [0.95, 0.25, 0.20, 1.0];

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
            .min(map_engine_core::camera::MAX_ZOOM);
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

    fn push_strip(out: &mut Vec<f32>, verts: &[StripVertex]) {
        for v in verts {
            out.extend_from_slice(&[
                v.pos[0], v.pos[1], v.color[0], v.color[1], v.color[2], v.color[3],
            ]);
        }
    }

    fn seg(out: &mut Vec<f32>, a: [f64; 2], b: [f64; 2], c: [f32; 4]) {
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

    fn quad(out: &mut Vec<f32>, corners: [[f64; 2]; 4], col: [f32; 4]) {
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
        /// `CONTOURS` hairlines: ghosts + arcs + normals + hatch.
        pub hairlines: Vec<f32>,
        pub hairline_count: u32,
    }

    fn append_polygon(
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

    /// Tessellate one (blueprint, view-floor) state into lane payloads. The Roof view draws the
    /// overall footprint plate plus every floor's wall centerlines as ghosts — the plan of what
    /// you stand ON, not a floor with its own walls.
    #[must_use]
    pub fn build_static_lanes(bp: &BuildingBlueprint, view: ViewFloor) -> StaticLanes {
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

        // Active floor plate + stairs plates.
        append_polygon(
            &mut out.floor_pos,
            &mut out.floor_col,
            &mut out.floor_idx,
            &lvl.footprint_polygon,
            COL_FLOOR,
        );
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
                seg(&mut out.hairlines, to_world(a), to_world(b), COL_HATCH);
                out.hairline_count += 1;
            }
        }

        // Walls at true thickness (active floor).
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
                &mut out.hairlines,
                &mut out.hairline_count,
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

        // Ghost floors: wall centerlines only.
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

        out
    }

    /// Ray strip + event dots for `MISSION_ZONES`, clipped to the ACTIVE view's elevation band
    /// (`band`/`band_last` from [`ViewFloor::band`], intersected via the raycaster's own
    /// [`clip_t_to_band`] so display and evaluation cannot disagree). Spans between consecutive
    /// hits are colored by a state machine over the trace: clear → green, after a window → cyan,
    /// after furniture cover → yellow; from a terminal block to the target → red. Dots draw only
    /// where the hit's own elevation lies inside the band. Returns `(packed, item_count)` —
    /// empty when the ray never enters the band (that floor's plan honestly shows no ray).
    #[must_use]
    pub fn build_ray_lane(
        obs: [f64; 3],
        tgt: [f64; 3],
        hits: &[LosHit],
        is_clear: bool,
        band: [f64; 2],
        band_last: bool,
    ) -> (Vec<f32>, u32) {
        let mut packed = Vec::new();
        let mut count = 0u32;
        let Some((band_t0, band_t1)) = clip_t_to_band(obs[1], tgt[1], band, band_last) else {
            return (packed, count);
        };
        let o2 = [obs[0], obs[2]];
        let t2 = [tgt[0], tgt[2]];
        let at = |t: f64| [o2[0] + t * (t2[0] - o2[0]), o2[1] + t * (t2[1] - o2[1])];

        let mut spans: Vec<(f64, f64, [f32; 4])> = Vec::new();
        let mut color = RAY_CLEAR;
        let mut t_prev = 0.0f64;
        for h in hits {
            spans.push((t_prev, h.t, color));
            t_prev = h.t;
            color = match h.kind {
                LosHitKind::Wall => RAY_BLOCKED,
                LosHitKind::Window => RAY_GLASS,
                LosHitKind::Furniture if h.concealment >= 1.0 => RAY_BLOCKED,
                LosHitKind::Furniture => RAY_COVER,
                LosHitKind::DoorOpen | LosHitKind::Stairs => color,
            };
        }
        spans.push((t_prev, 1.0, if is_clear { color } else { RAY_BLOCKED }));

        for (a, b, col) in spans {
            let (a, b) = (a.max(band_t0), b.min(band_t1));
            if b - a < 1e-6 {
                continue;
            }
            let verts = expand_polyline_strip(&[to_world(at(a)), to_world(at(b))], 0.16, col);
            push_strip(&mut packed, &verts);
            count += 1;
        }
        // Event dots — only those inside the viewed band.
        for h in hits {
            if h.pos[1] < band[0] - 1e-9 || h.pos[1] > band[1] + 1e-9 {
                continue;
            }
            let col = match h.kind {
                LosHitKind::Wall => RAY_BLOCKED,
                LosHitKind::Window => RAY_GLASS,
                LosHitKind::DoorOpen => RAY_CLEAR,
                LosHitKind::Furniture => RAY_COVER,
                LosHitKind::Stairs => COL_HATCH,
            };
            let c = [h.pos[0], h.pos[2]];
            quad(&mut packed, rect_corners(c, [0.34, 0.34], 45.0), col);
            count += 1;
        }
        (packed, count)
    }

    /// 360° viewshed from `obs` (horizontal rays at the observer's own elevation): one boundary
    /// point per direction — the terminal hit when blocked, the range cap when clear. Windows and
    /// open doors pass (vision extends beyond them); walls, closed doors, and full-cover stop it —
    /// exactly [`BuildingBlueprint::evaluate_los`]'s rules, because each direction IS one
    /// `evaluate_los` call. Returned points are blueprint-local plan coords, CCW by construction.
    #[must_use]
    pub fn build_viewshed(
        bp: &BuildingBlueprint,
        obs: [f64; 3],
        range_m: f64,
        n_rays: usize,
    ) -> Vec<[f64; 2]> {
        let mut boundary = Vec::with_capacity(n_rays);
        for i in 0..n_rays {
            let a = std::f64::consts::TAU * (i as f64) / (n_rays as f64);
            let tgt = [
                obs[0] + range_m * a.cos(),
                obs[1],
                obs[2] + range_m * a.sin(),
            ];
            let los = bp.evaluate_los(obs, tgt);
            let p = if los.is_clear {
                [tgt[0], tgt[2]]
            } else {
                los.hits
                    .last()
                    .map_or([tgt[0], tgt[2]], |h| [h.pos[0], h.pos[2]])
            };
            boundary.push(p);
        }
        boundary
    }

    /// Viewshed boundary → translucent fill fan for the `FOREST_FILL` strip lane.
    /// Returns `(packed, item_count)`.
    #[must_use]
    pub fn build_viewshed_lane(obs2: [f64; 2], boundary: &[[f64; 2]]) -> (Vec<f32>, u32) {
        let mut packed = Vec::new();
        if boundary.len() < 3 {
            return (packed, 0);
        }
        let col: [f32; 4] = [0.25, 0.90, 0.40, 0.16];
        let c = to_world(obs2);
        let mut count = 0u32;
        for i in 0..boundary.len() {
            let a = to_world(boundary[i]);
            let b = to_world(boundary[(i + 1) % boundary.len()]);
            for p in [c, a, b] {
                packed.extend_from_slice(&[
                    p[0] as f32,
                    p[1] as f32,
                    col[0],
                    col[1],
                    col[2],
                    col[3],
                ]);
            }
            count += 1;
        }
        (packed, count)
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use map_engine_core::building_blueprint::LosHit;

        fn farmhouse() -> BuildingBlueprint {
            serde_json::from_str(include_str!(
                "../../../../../../packages/map-assets/everon/prefabs/buildings/FarmHouse_E_1L01.json"
            ))
            .expect("golden parses")
        }

        #[test]
        fn world_mapping_round_trips_and_points_north_up() {
            let p = [-3.8, -4.5];
            let w = to_world(p);
            let back = from_world(w);
            // The ±6400 anchor shift costs a few low bits — sub-micrometer, irrelevant on screen.
            assert!((back[0] - p[0]).abs() < 1e-9 && (back[1] - p[1]).abs() < 1e-9);
            // The engine renders world +y UP (deck.gl flipY:false — pinned by the A.1 screenshot
            // mirror bug), so north (+z) must INCREASE world y…
            assert!(to_world([0.0, 5.0])[1] > to_world([0.0, 0.0])[1]);
            // …and larger world y must land HIGHER on screen (smaller CSS y).
            let css = (1280.0, 800.0);
            let hi = world_to_screen(to_world([0.0, 5.0]), 6400.0, 6400.0, 4.0, css);
            let lo = world_to_screen(to_world([0.0, 0.0]), 6400.0, 6400.0, 4.0, css);
            assert!(hi[1] < lo[1]);
        }

        #[test]
        fn screen_projection_round_trips() {
            let css = (1280.0, 800.0);
            let (tx, ty, zoom) = (6400.0, 6400.0, 4.2);
            let w = [6403.3, 6396.1];
            let s = world_to_screen(w, tx, ty, zoom, css);
            let back = screen_to_world(s, tx, ty, zoom, css);
            assert!((back[0] - w[0]).abs() < 1e-9 && (back[1] - w[1]).abs() < 1e-9);
        }

        #[test]
        fn fit_camera_centers_the_farmhouse() {
            let bp = farmhouse();
            let (tx, ty, zoom) = fit_camera(&bp, (1280.0, 800.0));
            // Camera target = world-mapped center of overall bbox (the fit_camera contract).
            // v7 note: bounding_box2_d is the ENTITY bounds from the dump meta, not the
            // polygon's extremes — the two legitimately differ, so no cross-check here.
            let bb = &bp.overall_footprint.bounding_box2_d;
            let c = to_world([(bb.min[0] + bb.max[0]) * 0.5, (bb.min[1] + bb.max[1]) * 0.5]);
            assert!((tx - c[0]).abs() < 1e-9, "tx {tx} vs {}", c[0]);
            assert!((ty - c[1]).abs() < 1e-9, "ty {ty} vs {}", c[1]);
            // A farmhouse-sized footprint × 1.25 margin across 1280 px sits inside the zoom clamp.
            assert!(zoom > 4.0 && zoom <= 6.0, "zoom {zoom}");
        }

        #[test]
        fn point_in_polygon_l_shape() {
            let bp = farmhouse();
            let ring = &bp.overall_footprint.polygon2_d;
            assert!(point_in_polygon([0.0, 0.0], ring));
            assert!(point_in_polygon([-3.0, 4.0], ring)); // west wing
            assert!(!point_in_polygon([5.0, 4.0], ring)); // the L notch
            assert!(!point_in_polygon([0.0, -6.0], ring)); // outside south
        }

        #[test]
        fn static_lanes_cover_every_feature_of_the_active_floor() {
            let bp = farmhouse();
            let l0 = build_static_lanes(&bp, ViewFloor::Level(0));
            assert_eq!(l0.wall_count, 7);
            assert_eq!(l0.aperture_count, 3 + 2); // windows + doors
            assert!(!l0.floor_pos.is_empty() && !l0.floor_idx.is_empty());
            assert_eq!(l0.floor_col.len() / 4, l0.floor_pos.len() / 2);
            // Furniture lanes mirror the data: the v7 extract carries none for the FarmHouse
            // (dump meta furniture: 0), and the lane must stay empty exactly when the level is.
            assert_eq!(l0.furn_pos.is_empty(), bp.levels[0].furniture.is_empty());
            // Ghost centerlines for the OTHER floor are present (4 upstairs walls).
            assert!(l0.hairline_count >= 4);
            let l1 = build_static_lanes(&bp, ViewFloor::Level(1));
            assert_eq!(l1.wall_count, 4);
            assert_eq!(l1.aperture_count, 2);
        }

        #[test]
        fn roof_view_is_footprint_plus_all_ghost_walls() {
            let bp = farmhouse();
            let roof = build_static_lanes(&bp, ViewFloor::Roof);
            assert_eq!(roof.wall_count, 0);
            assert_eq!(roof.aperture_count, 0);
            assert!(roof.furn_pos.is_empty());
            assert!(!roof.floor_pos.is_empty());
            // Ghosts: 7 ground + 4 upstairs centerlines.
            assert_eq!(roof.hairline_count, 11);
            // Roof band sits above the top level and reaches the ridge-carrying total height.
            let (band, last) = ViewFloor::Roof.band(&bp);
            assert_eq!(band, [5.6, 7.8]);
            assert!(last);
        }

        const GROUND_BAND: [f64; 2] = [0.0, 2.8];

        #[test]
        fn ray_lane_colors_follow_the_hit_state_machine() {
            let hit = |t: f64, kind: LosHitKind, conceal: f64| LosHit {
                t,
                pos: [0.0, 1.4, -8.0 + t * 7.0],
                kind,
                id: "x".into(),
                concealment: conceal,
            };
            // window pass → span colors [green, cyan]; still clear.
            let (packed, n) = build_ray_lane(
                [0.0, 1.4, -8.0],
                [0.0, 1.4, -1.0],
                &[hit(0.5, LosHitKind::Window, 0.0)],
                true,
                GROUND_BAND,
                false,
            );
            assert!(n >= 3); // 2 spans + 1 dot
            assert!(!packed.is_empty());
            let colors: Vec<[f32; 4]> = packed
                .chunks_exact(6)
                .map(|c| [c[2], c[3], c[4], c[5]])
                .collect();
            assert!(colors.contains(&RAY_CLEAR) && colors.contains(&RAY_GLASS));
            // blocked wall → red span present.
            let (packed, _) = build_ray_lane(
                [0.0, 1.4, -8.0],
                [0.0, 1.4, -1.0],
                &[hit(0.5, LosHitKind::Wall, 1.0)],
                false,
                GROUND_BAND,
                false,
            );
            let colors: Vec<[f32; 4]> = packed
                .chunks_exact(6)
                .map(|c| [c[2], c[3], c[4], c[5]])
                .collect();
            assert!(colors.contains(&RAY_BLOCKED));
        }

        #[test]
        fn ray_lane_is_clipped_to_the_viewed_band() {
            // A flat ground-floor ray on the ATTIC view: nothing to draw.
            let (packed, n) = build_ray_lane(
                [0.0, 1.4, -8.0],
                [0.0, 1.4, -1.0],
                &[],
                true,
                [2.8, 5.6],
                false,
            );
            assert!(packed.is_empty() && n == 0);
            // A climbing ray (0.9 → 4.5 m) split across views: the ground view draws only the
            // early t-range, the attic view only the late one — together they tile the ray.
            let obs = [-3.8, 0.9, -12.0];
            let tgt = [-3.8, 4.5, 1.0];
            let (g, gn) = build_ray_lane(obs, tgt, &[], true, GROUND_BAND, false);
            let (a, an) = build_ray_lane(obs, tgt, &[], true, [2.8, 5.6], false);
            assert!(gn >= 1 && an >= 1);
            // Ground portion must stay south of the attic portion (z increases with t). The
            // strip expander's round caps overshoot each endpoint by the half-width, so the two
            // portions may overlap by up to one strip width (0.16 m) at the shared band edge.
            let max_gz = g.chunks_exact(6).map(|c| c[1]).fold(f32::MIN, f32::max);
            let min_az = a.chunks_exact(6).map(|c| c[1]).fold(f32::MAX, f32::min);
            assert!(max_gz <= min_az + 0.2, "ground {max_gz} vs attic {min_az}");
        }

        #[test]
        fn viewshed_escapes_only_through_apertures() {
            let bp = farmhouse();
            // Observer mid living-room at standing eye height.
            let obs = [-3.8, 1.4, -1.0];
            let boundary = build_viewshed(&bp, obs, 25.0, 720);
            assert_eq!(boundary.len(), 720);
            let mut escaped_south = 0usize;
            for p in &boundary {
                // Anything clearly south of the south wall line exited through the window.
                if p[1] < -4.6 {
                    escaped_south += 1;
                }
                // Nothing may exceed the range cap.
                let d = map_engine_core::building_blueprint::dist_2d([obs[0], obs[2]], *p);
                assert!(d <= 25.0 + 1e-6);
            }
            // The south window lets a cone escape — but only a cone: solid walls stop the rest.
            // (Containment is NOT asserted via point-in-polygon: blocked points sit exactly ON
            // the wall centerline, where even-odd is float noise.)
            assert!(escaped_south > 0, "no rays escaped through the window");
            assert!(
                escaped_south < 200,
                "walls stopped nothing: {escaped_south} rays escaped south"
            );
            // The straight-north ray (i = 180 → 90°) faces solid `w_ext_north2` at z = 5.5:
            // it must REACH the wall (no phantom blocker) and STOP there (no leak).
            let north = boundary[180];
            assert!((north[0] - obs[0]).abs() < 0.1, "north ray bent: {north:?}");
            assert!(
                north[1] > 5.3 && north[1] < 5.6,
                "north ray should end on the north wall: {north:?}"
            );
            // Fan lane packs 3 verts per triangle, 6 f32 per vert.
            let (packed, n) = build_viewshed_lane([obs[0], obs[2]], &boundary);
            assert_eq!(n, 720);
            assert_eq!(packed.len(), 720 * 3 * 6);
        }

        #[test]
        fn rect_corners_rotation_preserves_area_orientation() {
            let c = rect_corners([1.0, 2.0], [2.0, 1.0], 90.0);
            // 90° rotation swaps extents around the center.
            let xs: Vec<f64> = c.iter().map(|p| p[0]).collect();
            let zs: Vec<f64> = c.iter().map(|p| p[1]).collect();
            let (w, d) = (
                xs.iter().cloned().fold(f64::MIN, f64::max)
                    - xs.iter().cloned().fold(f64::MAX, f64::min),
                zs.iter().cloned().fold(f64::MIN, f64::max)
                    - zs.iter().cloned().fold(f64::MAX, f64::min),
            );
            assert!((w - 1.0).abs() < 1e-9 && (d - 2.0).abs() < 1e-9);
        }
    }
}

// ═════════════════════════════════════════════════════════════════════════════════════════════
// The page.
// ═════════════════════════════════════════════════════════════════════════════════════════════

/// Building-blueprint viewer + interactive 2.5D LOS bench.
#[component]
pub fn BuildingViewerPage() -> impl IntoView {
    let blueprint = RwSignal::new(None::<BuildingBlueprint>);
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
    let canvas_ref = NodeRef::<leptos::html::Canvas>::new();

    // Pure LOS evaluation — reruns on any ray/blueprint change; the wasm host mirrors `los`
    // into the ray lane. Ungated: harmless on native (never mounted there).
    Effect::new(move |_| {
        let (o, t) = (obs.get(), tgt.get());
        blueprint.with(|bp| {
            los.set(
                bp.as_ref()
                    .map(|bp| bp.evaluate_los([o.x, o.y, o.z], [t.x, t.y, t.z])),
            );
        });
    });

    #[cfg(target_arch = "wasm32")]
    live::wire(
        canvas_ref,
        blueprint,
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
            view! {
                <div class="space-y-1">
                    <div class="flex items-center gap-2">{badge}
                        <span class="text-on-surface-variant">{format!("concealment {pct:.0}%")}</span>
                    </div>
                    {(!windows.is_empty()).then(|| view! { <div>"through glass: "<span class="text-cyan-300">{windows.clone()}</span></div> })}
                    {(!doors.is_empty()).then(|| view! { <div>"through door: "<span class="text-emerald-300">{doors.clone()}</span></div> })}
                    {r.blocked_by_wall_id.clone().map(|w| view! { <div>"blocked by "<span class="text-red-300">{w}</span></div> })}
                    {r.cover_furniture_id.clone().map(|f| view! { <div>"cover: "<span class="text-yellow-300">{f}</span></div> })}
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
                <div class="text-[10px] text-on-surface-variant">"drag A/B markers · drag canvas to pan · wheel zooms · click the building for floors · Alt+click moves A and fills its viewshed"</div>
                {move || viewshed_on.get().then(|| view! {
                    <div class="flex items-center gap-2 rounded bg-emerald-500/10 px-2 py-1 text-xs text-emerald-300">
                        "viewshed from A"
                        <button
                            type="button"
                            class="rounded px-1 text-on-surface-variant hover:text-red-300"
                            on:click=move |_| viewshed_on.set(false)
                        >
                            "✕ clear"
                        </button>
                    </div>
                })}
                {move || load_err.get().map(|e| view! { <div class="rounded bg-red-500/15 p-2 text-xs text-red-300">{e}</div> })}
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
                <div><span class="mr-1 inline-block h-2 w-4 bg-[#ebcc40]"></span>"low cover · ray past cover"</div>
                <div><span class="mr-1 inline-block h-2 w-4 bg-[#e6574a]"></span>"full cover · ray blocked"</div>
                <div><span class="mr-1 inline-block h-2 w-4 bg-[#7059b3]"></span>"stairs (transparent treads)"</div>
            </div>
        </div>
    }
}

// ═════════════════════════════════════════════════════════════════════════════════════════════
// wasm host: engine mount, lane uploads, listeners.
// ═════════════════════════════════════════════════════════════════════════════════════════════
#[cfg(target_arch = "wasm32")]
mod live {
    use super::{geom, Cam, Drag, RayEnd, ViewFloor, DEFAULT_PREFAB_PATH};
    use leptos::prelude::*;
    use map_engine_core::building_blueprint::{BuildingBlueprint, LosResult};
    use map_engine_render::draw_order::role_id;
    use map_engine_render::RenderEngine;
    use std::cell::RefCell;
    use std::rc::Rc;
    use wasm_bindgen::prelude::*;
    use wasm_bindgen::JsCast;

    type EngineHandle = Rc<RefCell<Option<RenderEngine>>>;

    fn sync_cam(e: &RenderEngine, cam: RwSignal<Cam>) {
        cam.set(Cam {
            tx: e.target_x(),
            ty: e.target_y(),
            zoom: e.zoom(),
        });
    }

    fn upload_static(e: &mut RenderEngine, bp: &BuildingBlueprint, view: ViewFloor) {
        let lanes = geom::build_static_lanes(bp, view);
        e.upload_polygon_mesh(
            role_id::LANDCOVER,
            &lanes.floor_pos,
            &lanes.floor_col,
            &lanes.floor_idx,
            1,
            true,
        );
        e.upload_polygon_mesh(
            role_id::AIRFIELD_APRON,
            &lanes.furn_pos,
            &lanes.furn_col,
            &lanes.furn_idx,
            1,
            true,
        );
        e.upload_strip_tris(role_id::ROADS_CASING, &lanes.walls, lanes.wall_count, true);
        e.upload_strip_tris(role_id::ROADS, &lanes.apertures, lanes.aperture_count, true);
        e.upload_hairline_segments(
            role_id::CONTOURS,
            &lanes.hairlines,
            lanes.hairline_count,
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
        let (packed, n) = geom::build_ray_lane(
            [obs.x, obs.y, obs.z],
            [tgt.x, tgt.y, tgt.z],
            &los.hits,
            los.is_clear,
            band,
            band_last,
        );
        e.upload_strip_tris(role_id::MISSION_ZONES, &packed, n, true);
        e.mark_dirty();
    }

    fn upload_viewshed(e: &mut RenderEngine, bp: &BuildingBlueprint, obs: RayEnd, on: bool) {
        if !on {
            e.clear_vector_lane(role_id::FOREST_FILL);
            e.mark_dirty();
            return;
        }
        let bb = &bp.overall_footprint.bounding_box2_d;
        let range = (bb.width_m.hypot(bb.depth_m)) + 5.0;
        let boundary = geom::build_viewshed(bp, [obs.x, obs.y, obs.z], range, 720);
        let (packed, n) = geom::build_viewshed_lane([obs.x, obs.z], &boundary);
        e.upload_strip_tris(role_id::FOREST_FILL, &packed, n, true);
        e.mark_dirty();
    }

    /// Self-referential rAF closure slot (the editor's `start_raf` idiom).
    type RafSlot = Rc<RefCell<Option<Closure<dyn FnMut()>>>>;

    /// rAF loop — render + poll, drop on dispose (the editor's `start_raf` minus the HUD).
    fn start_raf(engine: EngineHandle, disposed: std::sync::Arc<std::sync::atomic::AtomicBool>) {
        use std::sync::atomic::Ordering;
        let f: RafSlot = Rc::new(RefCell::new(None));
        let g = f.clone();
        *g.borrow_mut() = Some(Closure::wrap(Box::new(move || {
            if disposed.load(Ordering::Relaxed) {
                f.borrow_mut().take();
                return;
            }
            if let Ok(mut guard) = engine.try_borrow_mut() {
                if let Some(e) = guard.as_mut() {
                    let _ = e.render();
                    e.poll();
                }
            }
            let cb = f.borrow();
            if let (Some(cb), Some(win)) = (cb.as_ref(), web_sys::window()) {
                let _ = win.request_animation_frame(cb.as_ref().unchecked_ref());
            }
        }) as Box<dyn FnMut()>));
        let cb = g.borrow();
        if let (Some(cb), Some(win)) = (cb.as_ref(), web_sys::window()) {
            let _ = win.request_animation_frame(cb.as_ref().unchecked_ref());
        }
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

    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    pub fn wire(
        canvas_ref: NodeRef<leptos::html::Canvas>,
        blueprint: RwSignal<Option<BuildingBlueprint>>,
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
    ) {
        let engine: EngineHandle = Rc::new(RefCell::new(None));
        let disposed = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let fitted = Rc::new(std::cell::Cell::new(false));
        // Bumped once the engine lands so the upload effects rerun — without it, a blueprint that
        // arrives before `RenderEngine::create` resolves would never get its first upload.
        let engine_ready = RwSignal::new(false);

        // Blueprint fetch (once).
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
                    match RenderEngine::create(canvas, false).await {
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
                            start_raf(engine.clone(), disposed.clone());
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
                            upload_static(e, bp, view);
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

        // Viewshed fill: follows the observer while enabled; clears when toggled off.
        Effect::new({
            let engine = engine.clone();
            move |_| {
                if !engine_ready.get() {
                    return;
                }
                let on = viewshed_on.get();
                let o = obs.get();
                blueprint.with(|bp| {
                    let Some(bp) = bp.as_ref() else { return };
                    if let Ok(mut guard) = engine.try_borrow_mut() {
                        if let Some(e) = guard.as_mut() {
                            upload_viewshed(e, bp, o, on);
                        }
                    }
                });
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
                            blueprint.with_untracked(|bp| {
                                if let Some(bp) = bp.as_ref() {
                                    if geom::point_in_polygon(l, &bp.overall_footprint.polygon2_d) {
                                        floors_open.set(true);
                                    }
                                }
                            });
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
