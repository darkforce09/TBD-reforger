//! Role: attribution 1.
//! Position: `world/architecture/blueprint` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::spatial::bvh::sidecar::BvhSidecar;
use crate::world::architecture::blueprint::attribution_2::collect_level_annotations;
use crate::world::architecture::blueprint::geometry::aabb_contains_2d;
use crate::world::architecture::blueprint::geometry::dist_2d;
use crate::world::architecture::blueprint::geometry::point_at;
use crate::world::architecture::blueprint::geometry::point_segment_dist_2d;
use crate::world::architecture::blueprint::structure::BuildingBlueprint;

/// What a LOS ray met at one point along its path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LosHitKind {
    /// Structural (mesh) hit attributed to a wall — terminates the ray.
    Wall,

    /// Window aperture: traversed (concealment 0) when the ray passes the opening, or the terminal structural hit when the ray stops on frame mass inside the aperture rect.
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

    /// A glass pane crossed — concealment 0.05, never terminal.
    Glass,

    /// A closed (or swung) door leaf stopped the ray.
    DoorLeaf,

    /// A door set's frame stopped the ray.
    DoorFrame,

    /// The ray passed where an OPEN leaf would hang closed — annotation, concealment 0.
    DoorAperture,

    /// A window frame (mullion, sill) stopped the ray.
    WindowFrame,

    /// Canopy crossed — concealment `1 − e^(−k·depth)`, never terminal.
    Foliage,

    /// An opaque prop (entry, tree trunk, decoration) stopped the ray.
    Prop,
}

/// One ordered event along the observer→target ray. `t` is the parametric position on the FULL 3D segment (0 = observer, 1 = target) so the viewer can color the ray piecewise; `pos` is the 3D point at `t`. `concealment` is this hit's own contribution (1.0 for terminal blocks).
#[derive(Clone, Debug, PartialEq)]
pub struct LosHit {
    /// T.
    pub t: f64,

    /// Pos.
    pub pos: [f64; 3],

    /// Kind.
    pub kind: LosHitKind,

    /// Id.
    pub id: String,

    /// Concealment.
    pub concealment: f64,
}

/// Result of a line-of-sight calculation between an observer and a target: the mesh's structural verdict plus the blueprint's attribution and annotations.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct LosResult {
    /// True if there is an unobstructed visual line of sight.
    pub is_clear: bool,

    /// Every event along the ray ordered by `t` (windows/doors passed, cover crossed, and — last, when blocked — the terminal structural or full-cover hit). Empty for a fully open ray.
    pub hits: Vec<LosHit>,

    /// Window apertures traversed by the ray.
    pub window_ids_traversed: Vec<String>,

    /// Door openings traversed by the ray.
    pub door_ids_traversed: Vec<String>,

    /// Wall ID that blocked the line of sight (if blocked on a wall). Roof, stairs, furniture and `Solid` terminals leave it `None` — `hits.last()` is the record for those.
    pub blocked_by_wall_id: Option<String>,

    /// Furniture ID providing cover/concealment (e.g. table, crate).
    pub cover_furniture_id: Option<String>,

    /// Concealment score [0.0 = completely open, 1.0 = completely blocked].
    pub concealment: f64,
}

/// Canonical aperture slack m value.
pub(crate) const APERTURE_SLACK_M: f64 = 0.05;

/// Canonical flat eps value.
pub(crate) const FLAT_EPS: f64 = 1e-9;

/// Canonical wall attr near m value.
pub(crate) const WALL_ATTR_NEAR_M: f64 = 0.35;

/// Canonical roof attr tol m value.
pub(crate) const ROOF_ATTR_TOL_M: f64 = 0.30;

impl BuildingBlueprint {
    /// Evaluates line-of-sight from observer position `[x, y, z]` to target position `[x, y, z]` in the building's local coordinate space (y up, XZ the plan view).
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

        events.dedup_by(|a, b| a.id == b.id && (a.t - b.t).abs() < 1e-9);

        if let Some(hit) = occl
            .bvh
            .first_hit(&occl.verts, &occl.tris, obs, tgt, 0.0, 1.0)
        {
            let pos = point_at(obs, tgt, hit.t);
            let (kind, id) = self.attribute_structural_hit(pos);

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

                LosHitKind::Window if !terminal => {
                    result.window_ids_traversed.push(ev.id.clone());
                }
                LosHitKind::DoorOpen if !terminal => result.door_ids_traversed.push(ev.id.clone()),
                LosHitKind::Furniture => result.cover_furniture_id = Some(ev.id.clone()),

                LosHitKind::Window
                | LosHitKind::DoorOpen
                | LosHitKind::Stairs
                | LosHitKind::Roof
                | LosHitKind::Solid
                | LosHitKind::Glass
                | LosHitKind::DoorLeaf
                | LosHitKind::DoorFrame
                | LosHitKind::DoorAperture
                | LosHitKind::WindowFrame
                | LosHitKind::Foliage
                | LosHitKind::Prop => {}
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

impl BuildingBlueprint {
    /// Name the blueprint feature a structural mesh hit at `p` belongs to. Attribution only — it never changes the verdict. Order: the containing level's nearest wall (a window frame when inside that wall's aperture rect) → roof surface → stairs footprint → furniture footprint → [`LosHitKind::Solid`].
    pub(crate) fn attribute_structural_hit(&self, p: [f64; 3]) -> (LosHitKind, String) {
        let xz = [p[0], p[2]];
        let last = self.levels.len().saturating_sub(1);

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

/// Clip the segment's y-span to a level band, returning the `t`-range of the full segment that lies inside the band (intersected with [0,1]), or `None` when the band is never entered. Bands are half-open `[min, max)` except the topmost (closed) so a horizontal ray exactly on a shared floor/ceiling boundary belongs to exactly one level.
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
