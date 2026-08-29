//! High-fidelity building architectural blueprints and 2.5D Line-of-Sight (LOS) evaluation.
//!
//! Models multi-floor building prefabs, non-rectangular footprint polygons, walls with thickness,
//! selectable openings (doors, windows, glass), unselectable internal geometry (stairs, partitions),
//! furniture props with cover ratings, and vertical profiles for macro and micro line-of-sight.
//!
//! The blueprint JSON contract is `packages/tbd-schema/schema/building-blueprint.schema.json`
//! (camelCase — the map-assets prefab catalog, not the API), produced by the Workbench extractor
//! `apps/mod/tbd-export/.../MapExport/Objects/Buildings/TBD_BuildingArchitectExtractor.c` and
//! consumed by the `/debug/building-viewer` test bench.
//!
//! ── The 2.5D model ─────────────────────────────────────────────────────────────────────────────
//! Each [`BuildingLevel`] is a horizontal slice (`elevation_range` y-band) holding 2D geometry.
//! [`BuildingBlueprint::evaluate_los`] clips the 3D observer→target segment to every level's
//! y-band and evaluates each clipped sub-segment against that level's 2D geometry, so a ray that
//! climbs from the ground floor outside into an upstairs window is tested against the walls of
//! BOTH floors along the correct portions of its path (the old average-height single-level pick
//! chose one floor for the whole ray and blamed the wrong wall). Walls are centerline segments —
//! `thickness` is rendered by the viewer but not yet part of the intersection test.
//!
//! An optional [`RoofGrid`] heightfield models the roof: the ray is marched over the grid and a
//! sign crossing of the top surface (with a margin and a surface-continuity guard) is a terminal
//! block. Crossing semantics — not solid-below-surface — so rays under eave overhangs stay clear.

use serde::{Deserialize, Serialize};

/// 2D Bounding box representation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BBox2D {
    pub min: [f64; 2],
    pub max: [f64; 2],
    pub width_m: f64,
    pub depth_m: f64,
}

/// Vertical elevation and roof structure metadata for macro line-of-sight.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerticalProfile {
    pub pivot_elevation_offset_m: f64,
    pub foundation_skirt_depth_m: f64,
    pub total_height_m: f64,
    pub eave_height_m: f64,
    pub ridge_height_m: f64,
    pub chimney_height_m: Option<f64>,
    pub roof_type: String,
}

/// Overall 2D footprint geometry (supporting non-rectangular L-shapes, T-shapes, etc.).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OverallFootprint {
    pub polygon2_d: Vec<[f64; 2]>,
    pub bounding_box2_d: BBox2D,
    pub footprint_sq_m: f64,
}

/// Wall segment with physical thickness and penetration characteristics.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildingWall {
    pub id: String,
    pub start: [f64; 2],
    pub end: [f64; 2],
    pub thickness: f64,
    pub is_exterior: bool,
    pub material: String,
}

/// Door portal with hinge, swing arc, and glass status.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildingDoor {
    pub id: String,
    pub prefab_resource: String,
    pub wall_id: String,
    pub pos2_d: [f64; 2],
    pub width_m: f64,
    pub height_m: f64,
    pub hinge_side: String,
    pub swing_direction: String,
    pub is_exterior: bool,
    pub has_glass: bool,
    pub default_state: String,
}

/// Window opening with sill elevation, aperture dimensions, facing normal, and glass panes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildingWindow {
    pub id: String,
    pub prefab_resource: String,
    pub wall_id: String,
    pub pos2_d: [f64; 2],
    pub width_m: f64,
    pub sill_height_m: f64,
    pub window_height_m: f64,
    pub normal: [f64; 2],
    pub fov_deg: f64,
    pub has_glass: bool,
    pub glass_pane_count: u32,
}

/// Staircase with vertical connection, steps, and tread transparency.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildingStairs {
    pub id: String,
    pub bounds: [[f64; 2]; 2],
    pub connects_to_level: usize,
    pub direction_deg: f64,
    pub step_count: u32,
    pub transparent_steps: bool,
    pub los_concealment: f64,
}

/// Interior furniture prop with 2D bounds, height, and ballistic/visual cover classification.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildingFurniture {
    pub id: String,
    pub name: String,
    pub category: String,
    pub prefab_resource: String,
    pub pos2_d: [f64; 2],
    pub rotation_deg: f64,
    pub size2_d: [f64; 2],
    pub height_m: f64,
    pub blocks_movement: bool,
    pub los_cover: String,
}

/// A single architectural floor/level of the building.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildingLevel {
    pub level_index: usize,
    pub name: String,
    pub elevation_range: [f64; 2],
    pub slice_height_m: f64,
    pub footprint_polygon: Vec<[f64; 2]>,
    pub walls: Vec<BuildingWall>,
    pub doors: Vec<BuildingDoor>,
    pub windows: Vec<BuildingWindow>,
    pub stairs: Vec<BuildingStairs>,
    pub furniture: Vec<BuildingFurniture>,
}

/// Downsampled top-surface heightfield in the building's local frame (y up). Optional — a
/// blueprint without one evaluates exactly as before (no roof test).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoofGrid {
    /// Local `[x, z]` of the low corner of cell (0, 0).
    pub origin: [f64; 2],
    pub cell_size_m: f64,
    pub nx: usize,
    pub nz: usize,
    /// Row-major `ix * nz + iz`; `None` = no coverage (outside the roof silhouette).
    pub heights_m: Vec<Option<f64>>,
}

impl RoofGrid {
    /// Shape sanity — an inconsistent grid is skipped by the LOS test (lean clear).
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.cell_size_m > 0.0
            && self.nx > 0
            && self.nz > 0
            && self.heights_m.len() == self.nx * self.nz
    }

    /// Nearest-cell surface height at local `(x, z)` — deliberately NOT interpolated:
    /// interpolation across `None` cells is undefined and would smear chimneys and dormer pits
    /// into their neighbors.
    #[must_use]
    pub fn height_at(&self, x: f64, z: f64) -> Option<f64> {
        let fx = (x - self.origin[0]) / self.cell_size_m;
        let fz = (z - self.origin[1]) / self.cell_size_m;
        if fx < 0.0 || fz < 0.0 {
            return None;
        }
        let (ix, iz) = (fx as usize, fz as usize);
        if ix >= self.nx || iz >= self.nz {
            return None;
        }
        self.heights_m[ix * self.nz + iz]
    }
}

/// Complete building archetype blueprint definition.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildingBlueprint {
    pub schema_version: String,
    pub prefab_id: String,
    pub resource_name: String,
    pub model_mesh: Option<String>,
    pub label: Option<String>,
    pub kind: String,
    pub category: String,
    pub destructible: bool,
    pub vertical_profile: VerticalProfile,
    pub overall_footprint: OverallFootprint,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub roof: Option<RoofGrid>,
    pub levels: Vec<BuildingLevel>,
}

/// What a LOS ray met at one point along its path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LosHitKind {
    /// Solid wall — terminates the ray.
    Wall,
    /// Passed through a window aperture (ray height between sill and top).
    Window,
    /// Passed through an open door aperture.
    DoorOpen,
    /// Crossed furniture cover (`full_cover` terminates, `low_cover` conceals).
    Furniture,
    /// Crossed a transparent-tread stairwell (concealment only).
    Stairs,
    /// Crossed the roof heightfield surface — terminates the ray.
    Roof,
}

/// One ordered event along the observer→target ray. `t` is the parametric position on the FULL
/// 3D segment (0 = observer, 1 = target) so the viewer can color the ray piecewise; `pos` is the
/// 3D point at `t`. `concealment` is this hit's own contribution (1.0 for terminal blocks).
#[derive(Clone, Debug, PartialEq)]
pub struct LosHit {
    pub t: f64,
    pub pos: [f64; 3],
    pub kind: LosHitKind,
    pub id: String,
    pub concealment: f64,
}

/// Result of a 2.5D Line-of-Sight calculation between an observer and a target.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct LosResult {
    /// True if there is an unobstructed visual line of sight.
    pub is_clear: bool,
    /// Every event along the ray ordered by `t` (windows/doors passed, cover crossed, and — last,
    /// when blocked — the terminal wall or full-cover hit). Empty for a fully open ray.
    pub hits: Vec<LosHit>,
    /// Window apertures traversed by the ray.
    pub window_ids_traversed: Vec<String>,
    /// Door openings traversed by the ray.
    pub door_ids_traversed: Vec<String>,
    /// Wall ID that blocked the line of sight (if blocked).
    pub blocked_by_wall_id: Option<String>,
    /// Furniture ID providing cover/concealment (e.g. table, crate).
    pub cover_furniture_id: Option<String>,
    /// Concealment score [0.0 = completely open, 1.0 = completely blocked].
    pub concealment: f64,
}

/// Aperture match slack: a wall hit within (width/2 + this) of an opening's center counts as
/// passing through that opening. Absorbs centerline-vs-aperture measurement noise.
const APERTURE_SLACK_M: f64 = 0.05;
/// Below this |Δy| the ray is treated as horizontal for band clipping.
const FLAT_EPS: f64 = 1e-9;
/// A roof crossing needs `d = ray_y − surface` clearly past this on BOTH sides of the sign flip —
/// rays skimming the surface inside the margin lean clear (phantom blocks are worse than misses).
const ROOF_MARGIN_M: f64 = 0.15;
/// Max height step between consecutive roof samples still treated as one continuous surface.
/// Silhouette edges, chimney flanks, dormer cheeks and gable-end verticals exceed it, so a sign
/// flip across such a step is never a crossing. Admits roof slopes up to ~4.5 at the emitter's
/// default sampling pitch, matching the wall extractor's `roof_slope_hi` band.
const ROOF_MAX_STEP_M: f64 = 0.9;
/// Cap on roof samples per ray (degenerate very long rays).
const ROOF_MAX_SAMPLES: usize = 4096;

impl BuildingBlueprint {
    /// Evaluates 2.5D Line-of-Sight from observer position `[x, y, z]` to target position
    /// `[x, y, z]` in the building's local coordinate space (y up, XZ the plan view).
    ///
    /// The segment is clipped to every level's `elevation_range` y-band and each clipped
    /// sub-segment is tested against that level's 2D geometry; all events are merged into
    /// [`LosResult::hits`] ordered by `t` and the walk stops at the first terminal block
    /// (solid wall, or `full_cover` furniture below its height).
    #[must_use]
    pub fn evaluate_los(&self, obs: [f64; 3], tgt: [f64; 3]) -> LosResult {
        let mut events: Vec<LosHit> = Vec::new();
        let last_level = self.levels.len().saturating_sub(1);

        for (i, lvl) in self.levels.iter().enumerate() {
            let Some((t0, t1)) =
                clip_t_to_band(obs[1], tgt[1], lvl.elevation_range, i == last_level)
            else {
                continue;
            };
            collect_level_events(lvl, obs, tgt, t0, t1, &mut events);
        }

        if let Some(roof) = &self.roof
            && let Some(hit) = roof_crossing(roof, obs, tgt)
        {
            events.push(hit);
        }

        events.sort_by(|a, b| a.t.partial_cmp(&b.t).unwrap_or(std::cmp::Ordering::Equal));
        // A wall crossed exactly on a shared band boundary can be seen by both adjacent clips —
        // keep the first.
        events.dedup_by(|a, b| a.id == b.id && (a.t - b.t).abs() < 1e-9);

        let mut result = LosResult {
            is_clear: true,
            ..LosResult::default()
        };

        for ev in events {
            let terminal = ev.concealment >= 1.0;
            match ev.kind {
                LosHitKind::Wall => result.blocked_by_wall_id = Some(ev.id.clone()),
                LosHitKind::Window => result.window_ids_traversed.push(ev.id.clone()),
                LosHitKind::DoorOpen => result.door_ids_traversed.push(ev.id.clone()),
                LosHitKind::Furniture => result.cover_furniture_id = Some(ev.id.clone()),
                // Roof blocks terminate via concealment 1.0; there is no wall to blame.
                LosHitKind::Stairs | LosHitKind::Roof => {}
            }
            result.concealment = result.concealment.max(ev.concealment);
            result.hits.push(ev);
            if terminal {
                result.is_clear = false;
                break;
            }
        }

        result
    }
}

/// Clip the segment's y-span to a level band, returning the `t`-range of the full segment that
/// lies inside the band (intersected with [0,1]), or `None` when the band is never entered.
/// Bands are half-open `[min, max)` except the topmost (closed) so a horizontal ray exactly on a
/// shared floor/ceiling boundary belongs to exactly one level.
///
/// Public: the `/debug/building-viewer` bench clips its drawn ray to the ACTIVE floor's band with
/// the raycaster's own band math, so display and evaluation can never disagree.
#[must_use]
pub fn clip_t_to_band(y0: f64, y1: f64, band: [f64; 2], last: bool) -> Option<(f64, f64)> {
    let dy = y1 - y0;
    if dy.abs() < FLAT_EPS {
        let inside = y0 >= band[0] && (y0 < band[1] || (last && y0 <= band[1]));
        return inside.then_some((0.0, 1.0));
    }
    let (mut lo, mut hi) = ((band[0] - y0) / dy, (band[1] - y0) / dy);
    if lo > hi {
        std::mem::swap(&mut lo, &mut hi);
    }
    let (t0, t1) = (lo.max(0.0), hi.min(1.0));
    (t0 < t1).then_some((t0, t1))
}

/// March the ray over the roof heightfield and return the first top-surface crossing as a
/// terminal hit. A crossing needs `d = ray_y − surface` to flip from clearly above to clearly
/// below (or the reverse — piercing from inside the attic) along ONE continuous surface run:
/// `None` cells and steps over [`ROOF_MAX_STEP_M`] reset the run, so silhouette entries (through
/// walls or windows, where the surface jumps from nothing to roof height), chimney flanks and
/// dormer cheeks never count. Samples inside the ±[`ROOF_MARGIN_M`] band keep the last clear
/// side (hysteresis) instead of pairing adjacent samples — a sample landing inside the band
/// must not hide a genuine crossing, and wobble inside the band must not manufacture one.
fn roof_crossing(roof: &RoofGrid, obs: [f64; 3], tgt: [f64; 3]) -> Option<LosHit> {
    if !roof.is_valid() {
        return None;
    }
    // Clip the XZ projection to the grid AABB; `t` stays parametric on the FULL 3D segment.
    let max = [
        roof.origin[0] + roof.cell_size_m * roof.nx as f64,
        roof.origin[1] + roof.cell_size_m * roof.nz as f64,
    ];
    let (mut t0, mut t1) = (0.0_f64, 1.0_f64);
    for (o, d, lo, hi) in [
        (obs[0], tgt[0] - obs[0], roof.origin[0], max[0]),
        (obs[2], tgt[2] - obs[2], roof.origin[1], max[1]),
    ] {
        if d.abs() < FLAT_EPS {
            if o < lo || o > hi {
                return None;
            }
            continue;
        }
        let (mut a, mut b) = ((lo - o) / d, (hi - o) / d);
        if a > b {
            std::mem::swap(&mut a, &mut b);
        }
        t0 = t0.max(a);
        t1 = t1.min(b);
    }
    if t0 >= t1 {
        return None;
    }

    let dx = (tgt[0] - obs[0]) * (t1 - t0);
    let dz = (tgt[2] - obs[2]) * (t1 - t0);
    let plan_len = (dx * dx + dz * dz).sqrt();
    // Half-cell pitch so no cell along the path is skipped; ≥2 intervals covers vertical rays.
    let n = ((plan_len / (0.5 * roof.cell_size_m)).ceil() as usize).clamp(2, ROOF_MAX_SAMPLES);

    // (t, d, side) of the last sample clearly off the surface on the current continuous run.
    let mut anchor: Option<(f64, f64, i8)> = None;
    let mut prev_h: Option<f64> = None;
    for i in 0..=n {
        let t = t0 + (t1 - t0) * (i as f64 / n as f64);
        let p = point_at(obs, tgt, t);
        let Some(h) = roof.height_at(p[0], p[2]) else {
            prev_h = None;
            anchor = None;
            continue;
        };
        if let Some(ph) = prev_h
            && (ph - h).abs() > ROOF_MAX_STEP_M
        {
            anchor = None; // discontinuity — a new surface run starts here
        }
        prev_h = Some(h);
        let d = p[1] - h;
        let side: i8 = if d > ROOF_MARGIN_M {
            1
        } else if d < -ROOF_MARGIN_M {
            -1
        } else {
            0
        };
        if side != 0 {
            if let Some((ta, da, sa)) = anchor
                && sa != side
            {
                let tc = ta + (t - ta) * (da / (da - d));
                return Some(LosHit {
                    t: tc,
                    pos: point_at(obs, tgt, tc),
                    kind: LosHitKind::Roof,
                    id: "roof".to_string(),
                    concealment: 1.0,
                });
            }
            anchor = Some((t, d, side));
        }
    }
    None
}

/// Gather this level's wall / furniture / stairs events for the sub-segment `t ∈ [t0, t1]` of the
/// full 3D ray. `t` values pushed are relative to the FULL segment so cross-level ordering works.
fn collect_level_events(
    lvl: &BuildingLevel,
    obs: [f64; 3],
    tgt: [f64; 3],
    t0: f64,
    t1: f64,
    events: &mut Vec<LosHit>,
) {
    let p0 = point_at(obs, tgt, t0);
    let p1 = point_at(obs, tgt, t1);
    let sub0 = [p0[0], p0[2]];
    let sub1 = [p1[0], p1[2]];
    let sub_span = t1 - t0;
    if sub_span <= 0.0 {
        return;
    }

    // Walls — classified into window pass / open-door pass / solid block at the crossing point.
    for wall in &lvl.walls {
        let Some((t_sub, hit_pt)) = segment_intersection_t_2d(sub0, sub1, wall.start, wall.end)
        else {
            continue;
        };
        let t = t0 + t_sub * sub_span;
        let ray_y = obs[1] + t * (tgt[1] - obs[1]);
        let pos = point_at(obs, tgt, t);

        let hit_window = lvl.windows.iter().find(|w| {
            w.wall_id == wall.id && dist_2d(hit_pt, w.pos2_d) <= w.width_m * 0.5 + APERTURE_SLACK_M
        });
        if let Some(win) = hit_window {
            let win_bottom = lvl.elevation_range[0] + win.sill_height_m;
            let win_top = win_bottom + win.window_height_m;
            if ray_y >= win_bottom && ray_y <= win_top {
                events.push(LosHit {
                    t,
                    pos,
                    kind: LosHitKind::Window,
                    id: win.id.clone(),
                    concealment: 0.0,
                });
                continue;
            }
        }

        let hit_door = lvl.doors.iter().find(|d| {
            d.default_state == "open"
                && dist_2d(hit_pt, d.pos2_d) <= d.width_m * 0.5 + APERTURE_SLACK_M
        });
        if let Some(door) = hit_door
            && ray_y <= lvl.elevation_range[0] + door.height_m
        {
            events.push(LosHit {
                t,
                pos,
                kind: LosHitKind::DoorOpen,
                id: door.id.clone(),
                concealment: 0.0,
            });
            continue;
        }

        events.push(LosHit {
            t,
            pos,
            kind: LosHitKind::Wall,
            id: wall.id.clone(),
            concealment: 1.0,
        });
    }

    // Furniture cover — the ray must enter the prop's 2D AABB below the prop's top.
    for furn in &lvl.furniture {
        if furn.los_cover == "none" {
            continue;
        }
        let half_w = furn.size2_d[0] * 0.5;
        let half_d = furn.size2_d[1] * 0.5;
        let min_2d = [furn.pos2_d[0] - half_w, furn.pos2_d[1] - half_d];
        let max_2d = [furn.pos2_d[0] + half_w, furn.pos2_d[1] + half_d];

        let Some(t_sub) = segment_aabb_entry_t_2d(sub0, sub1, min_2d, max_2d) else {
            continue;
        };
        let t = t0 + t_sub * sub_span;
        let ray_y = obs[1] + t * (tgt[1] - obs[1]);
        let furn_top = lvl.elevation_range[0] + furn.height_m;
        if ray_y > furn_top {
            continue;
        }
        let concealment = if furn.los_cover == "full_cover" {
            1.0
        } else {
            0.60
        };
        events.push(LosHit {
            t,
            pos: point_at(obs, tgt, t),
            kind: LosHitKind::Furniture,
            id: furn.id.clone(),
            concealment,
        });
    }

    // Stairs — transparent treads conceal but never block.
    for stair in &lvl.stairs {
        if !stair.transparent_steps {
            continue;
        }
        let Some(t_sub) = segment_aabb_entry_t_2d(sub0, sub1, stair.bounds[0], stair.bounds[1])
        else {
            continue;
        };
        let t = t0 + t_sub * sub_span;
        events.push(LosHit {
            t,
            pos: point_at(obs, tgt, t),
            kind: LosHitKind::Stairs,
            id: stair.id.clone(),
            concealment: stair.los_concealment,
        });
    }
}

fn point_at(a: [f64; 3], b: [f64; 3], t: f64) -> [f64; 3] {
    [
        a[0] + t * (b[0] - a[0]),
        a[1] + t * (b[1] - a[1]),
        a[2] + t * (b[2] - a[2]),
    ]
}

/// 2D Euclidean distance helper.
#[must_use]
pub fn dist_2d(p1: [f64; 2], p2: [f64; 2]) -> f64 {
    ((p1[0] - p2[0]).powi(2) + (p1[1] - p2[1]).powi(2)).sqrt()
}

/// Like [`line_segment_intersection_2d`] but also returns `t` along `(p1, p2)`.
fn segment_intersection_t_2d(
    p1: [f64; 2],
    p2: [f64; 2],
    q1: [f64; 2],
    q2: [f64; 2],
) -> Option<(f64, [f64; 2])> {
    let dx1 = p2[0] - p1[0];
    let dy1 = p2[1] - p1[1];
    let dx2 = q2[0] - q1[0];
    let dy2 = q2[1] - q1[1];

    let det = dx1 * dy2 - dy1 * dx2;
    if det.abs() < 1e-9 {
        return None;
    }

    let t = ((q1[0] - p1[0]) * dy2 - (q1[1] - p1[1]) * dx2) / det;
    let u = ((q1[0] - p1[0]) * dy1 - (q1[1] - p1[1]) * dx1) / det;

    if (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u) {
        Some((t, [p1[0] + t * dx1, p1[1] + t * dy1]))
    } else {
        None
    }
}

/// Calculates intersection point of two 2D line segments `(p1, p2)` and `(q1, q2)`.
#[must_use]
pub fn line_segment_intersection_2d(
    p1: [f64; 2],
    p2: [f64; 2],
    q1: [f64; 2],
    q2: [f64; 2],
) -> Option<[f64; 2]> {
    segment_intersection_t_2d(p1, p2, q1, q2).map(|(_, pt)| pt)
}

/// Entry `t` of segment `(p1, p2)` into an axis-aligned 2D box: 0.0 when starting inside, else
/// the smallest edge-crossing `t`. `None` when the segment never touches the box.
fn segment_aabb_entry_t_2d(
    p1: [f64; 2],
    p2: [f64; 2],
    min: [f64; 2],
    max: [f64; 2],
) -> Option<f64> {
    if p1[0] >= min[0] && p1[0] <= max[0] && p1[1] >= min[1] && p1[1] <= max[1] {
        return Some(0.0);
    }
    let corners = [
        [min[0], min[1]],
        [max[0], min[1]],
        [max[0], max[1]],
        [min[0], max[1]],
    ];
    let mut best: Option<f64> = None;
    for i in 0..4 {
        if let Some((t, _)) = segment_intersection_t_2d(p1, p2, corners[i], corners[(i + 1) % 4]) {
            best = Some(best.map_or(t, |b: f64| b.min(t)));
        }
    }
    best
}

/// Tests if 2D line segment intersects 2D axis-aligned bounding box.
#[must_use]
pub fn segment_intersects_aabb_2d(
    p1: [f64; 2],
    p2: [f64; 2],
    min: [f64; 2],
    max: [f64; 2],
) -> bool {
    segment_aabb_entry_t_2d(p1, p2, min, max).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn farmhouse() -> BuildingBlueprint {
        let json_str = include_str!(
            "../../../packages/map-assets/everon/prefabs/buildings/FarmHouse_E_1L01.json"
        );
        serde_json::from_str(json_str).expect("Valid blueprint JSON")
    }

    #[test]
    fn parses_farmhouse_blueprint_json() {
        let bp = farmhouse();
        assert_eq!(bp.prefab_id, "FarmHouse_E_1L01");
        assert_eq!(bp.levels.len(), 2);
        assert_eq!(bp.overall_footprint.polygon2_d.len(), 6); // L-shape 6 vertices
        assert_eq!(bp.levels[0].windows.len(), 3);
        assert_eq!(bp.levels[0].doors.len(), 2);
        assert_eq!(bp.levels[0].stairs.len(), 1);
        assert!(bp.levels[0].stairs[0].transparent_steps);
    }

    #[test]
    fn evaluates_los_through_window() {
        let bp = farmhouse();

        // Outside observer looking straight through the south window at [-3.8, -4.5]
        let obs = [-3.8, 1.4, -8.0];
        let tgt = [-3.8, 1.4, -1.0];

        let los = bp.evaluate_los(obs, tgt);
        assert!(los.is_clear);
        assert!(
            los.window_ids_traversed
                .contains(&"win_gf_front_left".to_string())
        );
        assert_eq!(los.blocked_by_wall_id, None);
        // The ordered hit trace carries the same event with its position on the wall plane.
        let win_hit = los
            .hits
            .iter()
            .find(|h| h.kind == LosHitKind::Window)
            .expect("window hit present");
        assert_eq!(win_hit.id, "win_gf_front_left");
        assert!((win_hit.pos[2] - -4.5).abs() < 1e-9);
        assert!(win_hit.t > 0.0 && win_hit.t < 1.0);
    }

    #[test]
    fn evaluates_los_blocked_by_log_wall() {
        let bp = farmhouse();

        // Outside observer looking at solid wall at [-5.5, -4.5] (away from windows and doors)
        let obs = [-5.5, 1.4, -8.0];
        let tgt = [-5.5, 1.4, -1.0];

        let los = bp.evaluate_los(obs, tgt);
        assert!(!los.is_clear);
        assert_eq!(los.blocked_by_wall_id, Some("w_ext_south".to_string()));
        assert_eq!(los.concealment, 1.0);
        // The terminal block is the LAST hit; nothing is recorded past it.
        let last = los.hits.last().expect("blocking hit recorded");
        assert_eq!(last.kind, LosHitKind::Wall);
        assert_eq!(last.id, "w_ext_south");
        assert!((last.concealment - 1.0).abs() < f64::EPSILON);
    }

    /// Low-cover furniture conceals without blocking. Hand-built host: the farmhouse fixture's
    /// furniture went empty when 62d6ae3ad made it the v7 scan output (dump meta furniture: 0)
    /// — the viewer's static-lanes test mirrors that; this one keeps the semantics covered.
    #[test]
    fn evaluates_los_furniture_low_cover() {
        let mut bp = gable_blueprint();
        bp.levels[0].furniture.push(BuildingFurniture {
            id: "furn_table_01".to_string(),
            name: "table".to_string(),
            category: "prop".to_string(),
            prefab_resource: "synthetic://table".to_string(),
            pos2_d: [4.0, 5.0],
            rotation_deg: 0.0,
            size2_d: [1.2, 0.8],
            height_m: 0.78,
            blocks_movement: true,
            los_cover: "low_cover".to_string(),
        });

        // Enters the table's AABB at y ≈ 0.64 (below the 0.78 top): concealed, still clear.
        let los = bp.evaluate_los([3.0, 0.7, 4.5], [5.0, 0.4, 5.5]);
        assert!(los.is_clear, "hits: {:?}", los.hits);
        assert_eq!(los.cover_furniture_id, Some("furn_table_01".to_string()));
        assert!(los.concealment >= 0.60);
        assert!(
            los.hits
                .iter()
                .any(|h| h.kind == LosHitKind::Furniture && h.id == "furn_table_01")
        );
    }

    #[test]
    fn evaluates_los_second_floor_dormer_window() {
        let bp = farmhouse();

        // Observer outside on elevated slope looking into upper floor dormer window at [0.0, -4.5] (Y=3.8m)
        let obs = [0.0, 4.0, -9.0];
        let tgt = [0.0, 3.8, -1.0];

        let los = bp.evaluate_los(obs, tgt);
        assert!(los.is_clear);
        assert!(
            los.window_ids_traversed
                .contains(&"win_f1_dormer_south".to_string())
        );
    }

    /// The multi-band regression the old average-height pick got wrong: a ray climbing from
    /// ground level outside to above the upstairs windows crosses the south wall PLANE while in
    /// the level-1 band, so the blocker must be `w_f1_south` (upstairs siding), not the ground
    /// floor's `w_ext_south` (which its sub-segment never reaches). The old code averaged
    /// (0.9 + 4.5)/2 = 2.7 → picked level 0 for the whole ray and blamed `w_ext_south`.
    #[test]
    fn evaluates_los_cross_band_blames_correct_floor_wall() {
        let bp = farmhouse();

        let obs = [-3.8, 0.9, -12.0];
        let tgt = [-3.8, 4.5, 1.0];

        let los = bp.evaluate_los(obs, tgt);
        assert!(!los.is_clear);
        assert_eq!(los.blocked_by_wall_id, Some("w_f1_south".to_string()));
        // And no phantom ground-floor events: the level-0 clip ends before z = -4.5.
        assert!(los.hits.iter().all(|h| h.id != "w_ext_south"));
    }

    /// Same geometry through the aperture: raise the observer so the ray passes the south wall
    /// plane inside the dormer window's sill..top band — clear, via the level-1 window, even
    /// though the ray STARTED in the level-0 band.
    #[test]
    fn evaluates_los_cross_band_through_upstairs_window() {
        let bp = farmhouse();

        let obs = [0.0, 2.0, -12.0];
        let tgt = [0.0, 4.6, -1.0];

        let los = bp.evaluate_los(obs, tgt);
        assert!(los.is_clear, "hits: {:?}", los.hits);
        assert!(
            los.window_ids_traversed
                .contains(&"win_f1_dormer_south".to_string())
        );
    }

    #[test]
    fn clip_t_to_band_horizontal_boundary_belongs_to_upper_level() {
        // A horizontal ray exactly on the shared 2.8 boundary: excluded from the half-open
        // ground band, included in the closed top band.
        assert_eq!(clip_t_to_band(2.8, 2.8, [0.0, 2.8], false), None);
        assert_eq!(clip_t_to_band(2.8, 2.8, [2.8, 5.6], true), Some((0.0, 1.0)));
    }

    /// 14×14 half-metre synthetic gable: ridge along x at z = 3.5, pitch `h = 5.0 − |z_c − 3.5|
    /// · 0.8` per cell center (eave 2.8, ridge row 4.8), a `None` silhouette ring, one chimney
    /// cell at (3, 3) poking to 6.1, and a two-cell dormer pit at (8, 5..=6) dropping to 2.9.
    fn gable_roof() -> RoofGrid {
        let (nx, nz) = (14usize, 14usize);
        let mut heights = vec![None; nx * nz];
        for ix in 1..nx - 1 {
            for iz in 1..nz - 1 {
                let z_c = (iz as f64 + 0.5) * 0.5;
                heights[ix * nz + iz] = Some(5.0 - (z_c - 3.5).abs() * 0.8);
            }
        }
        heights[3 * nz + 3] = Some(6.1); // chimney
        heights[8 * nz + 5] = Some(2.9); // dormer pit
        heights[8 * nz + 6] = Some(2.9);
        RoofGrid {
            origin: [0.0, 0.0],
            cell_size_m: 0.5,
            nx,
            nz,
            heights_m: heights,
        }
    }

    /// Minimal host blueprint for the synthetic gable: one 7×7 level with a single wall plane at
    /// x = 2.0 (band [0, 2.4]) under the [`gable_roof`] heightfield.
    fn gable_blueprint() -> BuildingBlueprint {
        let square = vec![[0.0, 0.0], [7.0, 0.0], [7.0, 7.0], [0.0, 7.0]];
        BuildingBlueprint {
            schema_version: "1.0.0".to_string(),
            prefab_id: "SyntheticGable".to_string(),
            resource_name: "synthetic://gable".to_string(),
            model_mesh: None,
            label: None,
            kind: "building".to_string(),
            category: "generic".to_string(),
            destructible: false,
            vertical_profile: VerticalProfile {
                pivot_elevation_offset_m: 0.0,
                foundation_skirt_depth_m: 0.0,
                total_height_m: 6.1,
                eave_height_m: 2.8,
                ridge_height_m: 5.0,
                chimney_height_m: Some(6.1),
                roof_type: "scanned".to_string(),
            },
            overall_footprint: OverallFootprint {
                polygon2_d: square.clone(),
                bounding_box2_d: BBox2D {
                    min: [0.0, 0.0],
                    max: [7.0, 7.0],
                    width_m: 7.0,
                    depth_m: 7.0,
                },
                footprint_sq_m: 49.0,
            },
            roof: Some(gable_roof()),
            levels: vec![BuildingLevel {
                level_index: 0,
                name: "1st".to_string(),
                elevation_range: [0.0, 2.4],
                slice_height_m: 1.2,
                footprint_polygon: square,
                walls: vec![BuildingWall {
                    id: "w_test".to_string(),
                    start: [2.0, 0.0],
                    end: [2.0, 7.0],
                    thickness: 0.2,
                    is_exterior: true,
                    material: "scanned".to_string(),
                }],
                doors: vec![],
                windows: vec![],
                stairs: vec![],
                furniture: vec![],
            }],
        }
    }

    #[test]
    fn roof_crossing_blocks_through_pitch() {
        let bp = gable_blueprint();
        // Descends over the near eave into the rising pitch: d flips + → − on one continuous run.
        let los = bp.evaluate_los([3.5, 6.0, 0.2], [3.5, 2.0, 3.4]);
        assert!(!los.is_clear);
        let last = los.hits.last().expect("terminal roof hit");
        assert_eq!(last.kind, LosHitKind::Roof);
        assert_eq!(last.id, "roof");
        assert!((last.concealment - 1.0).abs() < f64::EPSILON);
        assert!(last.t > 0.4 && last.t < 0.75, "t = {}", last.t);
        assert!(
            last.pos[1] > 3.2 && last.pos[1] < 4.2,
            "pierce height = {}",
            last.pos[1]
        );
        assert_eq!(los.blocked_by_wall_id, None);
        assert!((los.concealment - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn roof_over_ridge_stays_clear() {
        let bp = gable_blueprint();
        // Horizontal at 6.0 — above the 4.8 ridge row everywhere along the path.
        let los = bp.evaluate_los([3.5, 6.0, 0.2], [3.5, 6.0, 6.8]);
        assert!(los.is_clear, "hits: {:?}", los.hits);
        assert!(los.hits.is_empty());
    }

    /// The fatal-flaw regression: a low ray entering the silhouette (surface jumps from the
    /// `None` ring straight to roof height) must NOT read the jump as a roof crossing.
    #[test]
    fn roof_silhouette_entry_is_not_a_crossing() {
        let bp = gable_blueprint();
        let los = bp.evaluate_los([3.5, 1.0, -1.0], [3.5, 1.0, 8.0]);
        assert!(los.is_clear, "hits: {:?}", los.hits);
        assert!(los.hits.is_empty());
    }

    /// An attic-height ray passing UNDER the dormer pit sees d flip − (under the pitch) to +
    /// (above the pit floor) — the >0.9 m cheek step splits the runs, so no crossing. Without
    /// the continuity guard this is a phantom block.
    #[test]
    fn roof_dormer_pit_passes() {
        let bp = gable_blueprint();
        let los = bp.evaluate_los([4.25, 3.2, 1.0], [4.25, 3.2, 4.6]);
        assert!(los.is_clear, "hits: {:?}", los.hits);
    }

    /// Flying past the chimney cell at flank height: pitch → chimney → pitch are three separate
    /// runs (2+ m steps), so neither flank flip is a crossing. Lean clear by design.
    #[test]
    fn roof_chimney_flank_needs_continuity() {
        let bp = gable_blueprint();
        let los = bp.evaluate_los([1.75, 5.0, 0.8], [1.75, 5.0, 2.8]);
        assert!(los.is_clear, "hits: {:?}", los.hits);
    }

    /// Skimming the flat ridge row inside the ±0.15 margin band — above or below — never
    /// anchors a side, so no crossing can fire.
    #[test]
    fn roof_margin_rejects_skims() {
        let bp = gable_blueprint();
        let over = bp.evaluate_los([0.8, 4.9, 3.75], [6.2, 4.9, 3.75]);
        assert!(over.is_clear, "hits: {:?}", over.hits);
        let under = bp.evaluate_los([0.8, 4.7, 3.75], [6.2, 4.7, 3.75]);
        assert!(under.is_clear, "hits: {:?}", under.hits);
    }

    /// Vertical attic → sky ray: d flips − → + through the ridge-row surface = a genuine
    /// piercing from below. Also exercises the zero-plan-length sampling path.
    #[test]
    fn roof_exit_from_below_blocks() {
        let bp = gable_blueprint();
        let los = bp.evaluate_los([3.5, 3.0, 3.75], [3.5, 7.0, 3.75]);
        assert!(!los.is_clear);
        let last = los.hits.last().expect("terminal roof hit");
        assert_eq!(last.kind, LosHitKind::Roof);
        assert!(
            (last.pos[1] - 4.8).abs() < 1e-9,
            "exit height = {}",
            last.pos[1]
        );
    }

    #[test]
    fn wall_closer_than_roof_wins_attribution() {
        let bp = gable_blueprint();
        // Crosses the x = 2.0 wall plane at t = 0.25 (y = 2.25, inside the band) and would
        // pierce the ridge row at t ≈ 0.79 — the walk stops at the nearer wall.
        let los = bp.evaluate_los([0.5, 1.0, 3.6], [6.5, 6.0, 3.6]);
        assert!(!los.is_clear);
        assert_eq!(los.blocked_by_wall_id, Some("w_test".to_string()));
        let last = los.hits.last().expect("terminal hit");
        assert_eq!(last.kind, LosHitKind::Wall);
        assert!(los.hits.iter().all(|h| h.kind != LosHitKind::Roof));
    }

    /// Roofless blueprints (every pre-roof JSON on map-assets) evaluate exactly as before, the
    /// absent field round-trips away, and a shape-invalid grid is skipped, not trusted.
    #[test]
    fn blueprint_without_roof_is_unchanged() {
        let bp = farmhouse();
        assert!(bp.roof.is_none());
        let v = serde_json::to_value(&bp).expect("serialize");
        assert!(v.get("roof").is_none(), "absent roof must not serialize");
        let los = bp.evaluate_los([-5.5, 1.4, -8.0], [-5.5, 1.4, -1.0]);
        assert!(!los.is_clear);
        assert_eq!(los.blocked_by_wall_id, Some("w_ext_south".to_string()));

        let mut broken = gable_blueprint();
        broken.roof.as_mut().expect("has roof").heights_m.pop();
        assert!(!broken.roof.as_ref().expect("has roof").is_valid());
        // The pitch-piercing ray from `roof_crossing_blocks_through_pitch` — with the grid
        // invalid the roof test is skipped and nothing else is on the path.
        let los = broken.evaluate_los([3.5, 6.0, 0.2], [3.5, 2.0, 3.4]);
        assert!(los.is_clear, "invalid grid must be ignored: {:?}", los.hits);
    }
}
