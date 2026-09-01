//! High-fidelity building architectural blueprints + line-of-sight (LOS) evaluation over the
//! building's collision trimesh.
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
//! ── The LOS model (T-090.6 step 3) ────────────────────────────────────────────────────────────
//! **Structure is the mesh.** [`BuildingBlueprint::evaluate_los`] raycasts the observer→target
//! segment through the building's `.bvh` occlusion sidecar ([`BvhSidecar`] — the COLL
//! fire-collision trimesh the engine itself traces) and takes the closest hit as the ONE terminal
//! block. Blueprint geometry never blocks a ray: it exists to **attribute** that structural hit
//! (which wall / window frame / roof / stairs / furniture the ray stopped on, or plain
//! [`LosHitKind::Solid`] when nothing in the blueprint claims the point) and to **annotate** the
//! path with the non-structural events the mesh cannot express — window and open-door apertures
//! traversed, furniture cover crossed (world siblings, NOT in the COLL mesh — `full_cover` is the
//! one annotation that terminates), transparent stairwells crossed.
//!
//! Each [`BuildingLevel`] is a horizontal slice (`elevation_range` y-band) holding 2D geometry;
//! the segment is clipped to every band ([`clip_t_to_band`]) so annotations come from the floor
//! the ray is actually on. The retired 2.5D interpretation model (band-uniform wall planes, roof
//! heightfield march) lives on only as rendering data — [`RoofGrid`] paints the roof view.

use serde::{Deserialize, Serialize};

use crate::bvh::BvhSidecar;

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
    /// Largest outer ring of the traced floor plate (empty for slab-less levels like the
    /// attic). The full multi-piece truth lives in `floor_polygons` / `plate`.
    pub footprint_polygon: Vec<[f64; 2]>,
    /// Verbatim per-cell floor occupancy + heights; absent on pre-plate blueprints.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plate: Option<PlateGrid>,
    /// Traced plate boundary rings (outer + holes per connected piece); empty when untraced.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub floor_polygons: Vec<FloorPolygon>,
    pub walls: Vec<BuildingWall>,
    pub doors: Vec<BuildingDoor>,
    pub windows: Vec<BuildingWindow>,
    pub stairs: Vec<BuildingStairs>,
    pub furniture: Vec<BuildingFurniture>,
}

/// Downsampled top-surface heightfield in the building's local frame (y up). Optional — the
/// roof view paints it, and a structural hit near its surface is attributed to the roof.
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
    /// Shape sanity — an inconsistent grid is skipped by consumers (attribution falls through
    /// to [`LosHitKind::Solid`]; the roof view paints nothing).
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

/// Downsampled walkable-floor heightfield for ONE level, in the building's local frame (y up).
/// Same shape as [`RoofGrid`] but different semantics: a covered cell means "there is floor
/// slab here at this height" — the verbatim per-cell product of the plate scan, so partial
/// mezzanines and double-height voids render exactly as measured. Optional — absent on
/// pre-plate blueprints and on levels with no real slab (attic).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlateGrid {
    /// Local `[x, z]` of the low corner of cell (0, 0).
    pub origin: [f64; 2],
    pub cell_size_m: f64,
    pub nx: usize,
    pub nz: usize,
    /// Row-major `ix * nz + iz`; `None` = no floor here; `Some(y)` = local slab-surface height.
    pub heights_m: Vec<Option<f64>>,
}

impl PlateGrid {
    /// Shape sanity — an inconsistent grid is ignored by consumers.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.cell_size_m > 0.0
            && self.nx > 0
            && self.nz > 0
            && self.heights_m.len() == self.nx * self.nz
    }

    /// Nearest-cell floor height at local `(x, z)`; `None` outside the grid or over a void.
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

/// One connected piece of a level's floor plate as traced polygon rings: an outer boundary
/// (CCW) plus any interior voids (CW holes). A level with disconnected floor (split mezzanine)
/// carries several of these.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FloorPolygon {
    pub outer: Vec<[f64; 2]>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub holes: Vec<Vec<[f64; 2]>>,
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
    /// Structural (mesh) hit attributed to a wall — terminates the ray.
    Wall,
    /// Window aperture: traversed (concealment 0) when the ray passes the opening, or the
    /// terminal structural hit when the ray stops on frame mass inside the aperture rect.
    Window,
    /// Open-door aperture traversed (concealment 0).
    DoorOpen,
    /// Crossed furniture cover (`full_cover` terminates, `low_cover` conceals).
    Furniture,
    /// Crossed a transparent-tread stairwell (concealment only), or stopped on stair mass.
    Stairs,
    /// Structural hit attributed to the roof surface — terminates the ray.
    Roof,
    /// Structural hit no blueprint feature claims — terminates the ray.
    Solid,
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

/// Result of a line-of-sight calculation between an observer and a target: the mesh's
/// structural verdict plus the blueprint's attribution and annotations.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct LosResult {
    /// True if there is an unobstructed visual line of sight.
    pub is_clear: bool,
    /// Every event along the ray ordered by `t` (windows/doors passed, cover crossed, and — last,
    /// when blocked — the terminal structural or full-cover hit). Empty for a fully open ray.
    pub hits: Vec<LosHit>,
    /// Window apertures traversed by the ray.
    pub window_ids_traversed: Vec<String>,
    /// Door openings traversed by the ray.
    pub door_ids_traversed: Vec<String>,
    /// Wall ID that blocked the line of sight (if blocked on a wall). Roof, stairs, furniture and
    /// `Solid` terminals leave it `None` — `hits.last()` is the record for those.
    pub blocked_by_wall_id: Option<String>,
    /// Furniture ID providing cover/concealment (e.g. table, crate).
    pub cover_furniture_id: Option<String>,
    /// Concealment score [0.0 = completely open, 1.0 = completely blocked].
    pub concealment: f64,
}

/// Aperture match slack: a wall crossing within (width/2 + this) of an opening's center counts as
/// passing through that opening. Absorbs centerline-vs-aperture measurement noise.
const APERTURE_SLACK_M: f64 = 0.05;
/// Below this |Δy| the ray is treated as horizontal for band clipping.
const FLAT_EPS: f64 = 1e-9;
/// A structural hit within this XZ distance of a wall centerline is attributed to that wall.
/// Blueprint walls are centerlines and COLL faces sit half a thickness away — 0.35 covers the
/// 0.6 m log walls with margin without reaching across a room.
const WALL_ATTR_NEAR_M: f64 = 0.35;
/// A structural hit within this |Δy| of the roof heightfield surface is attributed to the roof
/// (the grid is a 0.3 m-pitch nearest-cell sample of a sloped surface).
const ROOF_ATTR_TOL_M: f64 = 0.30;

impl BuildingBlueprint {
    /// Evaluates line-of-sight from observer position `[x, y, z]` to target position `[x, y, z]`
    /// in the building's local coordinate space (y up, XZ the plan view).
    ///
    /// `occl` is the building's `.bvh` occlusion sidecar — the structural authority. The closest
    /// mesh hit on the segment is the ONE terminal block, attributed through the blueprint (wall
    /// id, window frame, roof, stairs, furniture, or [`LosHitKind::Solid`]). The blueprint's own
    /// geometry only annotates: the segment is clipped to every level's `elevation_range` band
    /// and each clipped sub-segment yields that floor's aperture traversals and cover crossings.
    /// All events merge into [`LosResult::hits`] ordered by `t` and the walk stops at the first
    /// terminal (the structural hit, or `full_cover` furniture below its height).
    #[must_use]
    pub fn evaluate_los(&self, occl: &BvhSidecar, obs: [f64; 3], tgt: [f64; 3]) -> LosResult {
        let mut events: Vec<LosHit> = Vec::new();
        let last_level = self.levels.len().saturating_sub(1);

        for (i, lvl) in self.levels.iter().enumerate() {
            let Some((t0, t1)) =
                clip_t_to_band(obs[1], tgt[1], lvl.elevation_range, i == last_level)
            else {
                continue;
            };
            collect_level_annotations(lvl, obs, tgt, t0, t1, &mut events);
        }

        events.sort_by(|a, b| a.t.partial_cmp(&b.t).unwrap_or(std::cmp::Ordering::Equal));
        // An aperture crossed exactly on a shared band boundary can be seen by both adjacent
        // clips — keep the first. Annotation-only: the structural hit is inserted afterwards
        // and can never be collapsed.
        events.dedup_by(|a, b| a.id == b.id && (a.t - b.t).abs() < 1e-9);

        if let Some(hit) = occl
            .bvh
            .first_hit(&occl.verts, &occl.tris, obs, tgt, 0.0, 1.0)
        {
            let pos = point_at(obs, tgt, hit.t);
            let (kind, id) = self.attribute_structural_hit(pos);
            // Before any equal-t annotation: a terminal ON the aperture plane is frame mass —
            // the ray stopped there, it did not pass through.
            let at = events.partition_point(|e| e.t < hit.t);
            events.insert(
                at,
                LosHit {
                    t: hit.t,
                    pos,
                    kind,
                    id,
                    concealment: 1.0,
                },
            );
        }

        let mut result = LosResult {
            is_clear: true,
            ..LosResult::default()
        };

        for ev in events {
            let terminal = ev.concealment >= 1.0;
            match ev.kind {
                LosHitKind::Wall => result.blocked_by_wall_id = Some(ev.id.clone()),
                // Traversed only when the ray actually passed the opening — a terminal aperture
                // hit is the frame.
                LosHitKind::Window if !terminal => {
                    result.window_ids_traversed.push(ev.id.clone());
                }
                LosHitKind::DoorOpen if !terminal => result.door_ids_traversed.push(ev.id.clone()),
                LosHitKind::Furniture => result.cover_furniture_id = Some(ev.id.clone()),
                // Terminals with no wall to blame: the hit itself (last in `hits`) is the record.
                LosHitKind::Window
                | LosHitKind::DoorOpen
                | LosHitKind::Stairs
                | LosHitKind::Roof
                | LosHitKind::Solid => {}
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

    /// Name the blueprint feature a structural mesh hit at `p` belongs to. Attribution only —
    /// it never changes the verdict. Order: the containing level's nearest wall (a window frame
    /// when inside that wall's aperture rect) → roof surface → stairs footprint → furniture
    /// footprint → [`LosHitKind::Solid`].
    fn attribute_structural_hit(&self, p: [f64; 3]) -> (LosHitKind, String) {
        let xz = [p[0], p[2]];
        let last = self.levels.len().saturating_sub(1);
        // Same half-open rule as `clip_t_to_band`: bands own [min, max), the topmost is closed.
        let level = self
            .levels
            .iter()
            .enumerate()
            .find(|(i, l)| {
                let [lo, hi] = l.elevation_range;
                p[1] >= lo && (p[1] < hi || (*i == last && p[1] <= hi))
            })
            .map(|(_, l)| l);

        if let Some(lvl) = level {
            let nearest = lvl
                .walls
                .iter()
                .map(|w| {
                    let (d, on_wall) = point_segment_dist_2d(xz, w.start, w.end);
                    (d, on_wall, w)
                })
                .filter(|(d, ..)| *d <= WALL_ATTR_NEAR_M)
                .min_by(|a, b| a.0.total_cmp(&b.0));
            if let Some((_, on_wall, wall)) = nearest {
                let frame = lvl.windows.iter().find(|w| {
                    let bottom = lvl.elevation_range[0] + w.sill_height_m;
                    w.wall_id == wall.id
                        && dist_2d(on_wall, w.pos2_d) <= w.width_m * 0.5 + APERTURE_SLACK_M
                        && p[1] >= bottom
                        && p[1] <= bottom + w.window_height_m
                });
                return match frame {
                    Some(win) => (LosHitKind::Window, win.id.clone()),
                    None => (LosHitKind::Wall, wall.id.clone()),
                };
            }
        }

        if let Some(roof) = &self.roof
            && roof.is_valid()
            && roof
                .height_at(p[0], p[2])
                .is_some_and(|h| (p[1] - h).abs() <= ROOF_ATTR_TOL_M)
        {
            return (LosHitKind::Roof, "roof".to_string());
        }

        if let Some(lvl) = level {
            if let Some(stair) = lvl
                .stairs
                .iter()
                .find(|s| aabb_contains_2d(xz, s.bounds[0], s.bounds[1]))
            {
                return (LosHitKind::Stairs, stair.id.clone());
            }
            if let Some(furn) = lvl.furniture.iter().find(|f| {
                let half = [f.size2_d[0] * 0.5, f.size2_d[1] * 0.5];
                p[1] <= lvl.elevation_range[0] + f.height_m
                    && aabb_contains_2d(
                        xz,
                        [f.pos2_d[0] - half[0], f.pos2_d[1] - half[1]],
                        [f.pos2_d[0] + half[0], f.pos2_d[1] + half[1]],
                    )
            }) {
                return (LosHitKind::Furniture, furn.id.clone());
            }
        }

        (LosHitKind::Solid, "solid".to_string())
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

/// Gather this level's annotations for the sub-segment `t ∈ [t0, t1]` of the full 3D ray:
/// window / open-door aperture traversals, furniture cover and transparent-stair crossings.
/// Never terminal except `full_cover` furniture — props are world siblings of the building, not
/// part of its COLL mesh, so the mesh cannot see them. A wall crossing outside any aperture is
/// NOT an event: whether the ray stops there is the mesh's call (`evaluate_los`'s structural hit).
/// `t` values pushed are relative to the FULL segment so cross-level ordering works.
fn collect_level_annotations(
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

    // Apertures — a wall-centerline crossing inside a window's sill..top band or an open door's
    // height is a traversal.
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
        }
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

/// Distance from `p` to segment `(a, b)` and the closest point on it (project, clamp).
fn point_segment_dist_2d(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> (f64, [f64; 2]) {
    let ab = [b[0] - a[0], b[1] - a[1]];
    let len2 = ab[0] * ab[0] + ab[1] * ab[1];
    let u = if len2 > 0.0 {
        (((p[0] - a[0]) * ab[0] + (p[1] - a[1]) * ab[1]) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let closest = [a[0] + u * ab[0], a[1] + u * ab[1]];
    (dist_2d(p, closest), closest)
}

fn aabb_contains_2d(p: [f64; 2], min: [f64; 2], max: [f64; 2]) -> bool {
    p[0] >= min[0] && p[0] <= max[0] && p[1] >= min[1] && p[1] <= max[1]
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
    if aabb_contains_2d(p1, min, max) {
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
#[path = "building_blueprint_tests.rs"]
pub(crate) mod tests;
