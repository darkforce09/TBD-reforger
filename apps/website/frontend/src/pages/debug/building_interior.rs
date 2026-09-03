//! T-090.11.6 — the building bench's OWN render lanes (`role_id::INTERIOR_*` / `SCENE_*`,
//! T-090.11.5) fed from the compound building (T-090.11.4): the shell's section cuts are the
//! walls, door leaves draw where they hang (orange closed, green open, with their swing arc),
//! glass panes are cyan strips, window frames jamb ticks, furniture and props footprints by
//! cover tier, scene trees a trunk disc + canopy, and the LOS probe rides its own lane above
//! the wash. Pure geometry — native-tested; `building_viewer.rs`'s wasm host uploads it.
//!
//! Without a compound the blueprint-only tessellation of [`geom::build_static_lanes`] is simply
//! re-routed lane by lane ([`InteriorLanes::from_static`]) — the six terrain lanes the bench used
//! to borrow (`LANDCOVER`, `AIRFIELD_APRON`, `ROADS_CASING`, `ROADS`, `CONTOURS`,
//! `FOREST_OUTLINE`) and `MISSION_ZONES` for the ray are never touched again.
#![allow(dead_code)] // native build: the wasm host wires the live path; tests pin the pure core.

use map_engine_core::building_blueprint::{clip_t_to_band, BuildingBlueprint, LosHit, LosHitKind};
use map_engine_core::building_compound::{CompoundBuilding, CoverTier, Instance, InstanceKind};
use map_engine_core::building_section::{section_at_owned, BuildingDrawing, Seg2, CUT_MAX_NY};
use map_engine_core::bvh::SurfaceKind;
use map_engine_core::geometry::polyline_strip::expand_polyline_strip;
use map_engine_core::geometry::rigid::Rigid;
#[cfg(target_arch = "wasm32")]
use map_engine_render::draw_order::role_id;

/// Native mirror of `map_engine_render::draw_order::role_id` — the render crate is a wasm32-only
/// dependency of the SPA, and this module's tests run natively. Not a hand-copy that can drift:
/// `lane_ids_match_the_render_crate` pins every value against the render crate's source.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod role_id {
    pub const LANDCOVER: u32 = 1;
    pub const CONTOURS: u32 = 2;
    pub const ROADS_CASING: u32 = 3;
    pub const ROADS: u32 = 4;
    pub const FOREST_OUTLINE: u32 = 6;
    pub const AIRFIELD_APRON: u32 = 8;
    pub const MISSION_ZONES: u32 = 10;
    pub const INTERIOR_SLABS: u32 = 11;
    pub const INTERIOR_FURNITURE: u32 = 12;
    pub const INTERIOR_FURNITURE_OUTLINE: u32 = 13;
    pub const INTERIOR_WALLS: u32 = 14;
    pub const INTERIOR_WALLS_OUTLINE: u32 = 15;
    pub const INTERIOR_PORTALS: u32 = 16;
    pub const INTERIOR_PORTALS_OUTLINE: u32 = 17;
    pub const INTERIOR_GLAZING: u32 = 18;
    pub const INTERIOR_GLAZING_OUTLINE: u32 = 19;
    pub const INTERIOR_STAIRS: u32 = 20;
    pub const SCENE_VEGETATION: u32 = 21;
    pub const SCENE_VEGETATION_OUTLINE: u32 = 22;
    pub const INTERIOR_PROBE: u32 = 23;
    pub const MAX: u32 = INTERIOR_PROBE;
}

use super::building_viewer::geom::{
    self, append_polygon, push_strip, quad, rect_corners, seg, to_world, StaticLanes,
    COL_DOOR_CLOSED, COL_DOOR_OPEN, COL_FURN_FULL, COL_FURN_LOW, COL_FURN_NONE, COL_HATCH,
    COL_NORMAL, COL_WALL_EXT, COL_WINDOW, RAY_BLOCKED, RAY_CLEAR, RAY_COVER, RAY_GLASS,
};
use super::building_viewer::ViewFloor;

/// Ray span / dot colour after a canopy crossing — yellow-green, between the cover yellow and the
/// clear green so a grazed hedge reads as "seen through leaves".
pub const RAY_FOLIAGE: [f32; 4] = [0.62, 0.85, 0.25, 1.0];
/// Tree trunk disc.
pub const COL_TRUNK: [f32; 4] = [0.55, 0.40, 0.25, 0.95];
/// Canopy fill (translucent so walls under an overhang stay legible).
pub const COL_CANOPY: [f32; 4] = [0.30, 0.55, 0.28, 0.45];
/// Canopy rim + stipple.
pub const COL_CANOPY_EDGE: [f32; 4] = [0.45, 0.75, 0.40, 0.80];
/// Prop footprints (entries, radiators, decorations — no cover tier).
pub const COL_PROP: [f32; 4] = [0.50, 0.55, 0.62, 0.55];
/// Strip width of a door leaf (m) — the collider is 6 cm; drawn a touch heavier.
pub const LEAF_STRIP_M: f64 = 0.08;
/// Strip width of a glass pane cut (m).
pub const PANE_STRIP_M: f64 = 0.05;
/// Trunk disc radius (m).
pub const TRUNK_R_M: f64 = 0.25;
/// Click slack around a leaf's collider when hit-testing (m).
pub const DOOR_HIT_SLACK_M: f64 = 0.18;

/// One level's owned section cuts of the flattened compound (`section_at_owned` at that level's
/// main cut height) — computed by the page whenever a door toggles.
#[derive(Clone, Debug, PartialEq)]
pub struct LevelCuts {
    pub level_index: usize,
    pub y: f64,
    pub cuts: Vec<(Seg2, u32)>,
}

impl LevelCuts {
    /// Every level's cuts from `drawing` (its `cut_main_y`) over the compound's current state.
    #[must_use]
    pub fn for_drawing(compound: &CompoundBuilding, drawing: &BuildingDrawing) -> Vec<LevelCuts> {
        let flat = compound.flatten();
        drawing
            .levels
            .iter()
            .map(|l| LevelCuts {
                level_index: l.level_index,
                y: l.cut_main_y,
                cuts: section_at_owned(&flat.mesh, &flat.owner, l.cut_main_y, CUT_MAX_NY),
            })
            .collect()
    }
}

/// One `Vec<f32>` (+ item count) per building-bench lane, in `ROLES` order.
#[derive(Default, Debug)]
pub struct InteriorLanes {
    /// `INTERIOR_SLABS` polygon mesh: floor plates / roof cells / stairs plates.
    pub slabs_pos: Vec<f32>,
    pub slabs_col: Vec<f32>,
    pub slabs_idx: Vec<u32>,
    /// `INTERIOR_FURNITURE` polygon mesh: furniture / prop footprints by cover tier.
    pub furniture_pos: Vec<f32>,
    pub furniture_col: Vec<f32>,
    pub furniture_idx: Vec<u32>,
    /// `INTERIOR_FURNITURE_OUTLINE` hairlines.
    pub furniture_outline: Vec<f32>,
    pub furniture_outline_count: u32,
    /// `INTERIOR_WALLS` strip tris: wall section cuts (or blueprint walls without a mesh).
    pub walls: Vec<f32>,
    pub wall_count: u32,
    /// `INTERIOR_WALLS_OUTLINE` hairlines: cut twin, low cut, rings, ghosts, window normals.
    pub walls_outline: Vec<f32>,
    pub walls_outline_count: u32,
    /// `INTERIOR_PORTALS` strip tris: door leaves where they hang + door frames.
    pub portals: Vec<f32>,
    pub portal_count: u32,
    /// `INTERIOR_PORTALS_OUTLINE` hairlines: swing arcs.
    pub portals_outline: Vec<f32>,
    pub portals_outline_count: u32,
    /// `INTERIOR_GLAZING` strip tris: glass pane cuts.
    pub glazing: Vec<f32>,
    pub glazing_count: u32,
    /// `INTERIOR_GLAZING_OUTLINE` hairlines: window-frame jamb ticks.
    pub glazing_outline: Vec<f32>,
    pub glazing_outline_count: u32,
    /// `INTERIOR_STAIRS` hairlines: tread hatch.
    pub stairs: Vec<f32>,
    pub stairs_count: u32,
    /// `SCENE_VEGETATION` polygon mesh: trunk discs + canopies.
    pub vegetation_pos: Vec<f32>,
    pub vegetation_col: Vec<f32>,
    pub vegetation_idx: Vec<u32>,
    /// `SCENE_VEGETATION_OUTLINE` hairlines: canopy rims + stipple.
    pub vegetation_outline: Vec<f32>,
    pub vegetation_outline_count: u32,
    /// Diagnostics: instances drawn on this view.
    pub leaf_count: u32,
    pub pane_count: u32,
    pub furniture_count: u32,
    pub tree_count: u32,
}

impl InteriorLanes {
    /// The upload ids, in lane order — every one ≥ `INTERIOR_SLABS`; the probe is last.
    pub const ROLES: [u32; 13] = [
        role_id::INTERIOR_SLABS,
        role_id::INTERIOR_FURNITURE,
        role_id::INTERIOR_FURNITURE_OUTLINE,
        role_id::INTERIOR_WALLS,
        role_id::INTERIOR_WALLS_OUTLINE,
        role_id::INTERIOR_PORTALS,
        role_id::INTERIOR_PORTALS_OUTLINE,
        role_id::INTERIOR_GLAZING,
        role_id::INTERIOR_GLAZING_OUTLINE,
        role_id::INTERIOR_STAIRS,
        role_id::SCENE_VEGETATION,
        role_id::SCENE_VEGETATION_OUTLINE,
        role_id::INTERIOR_PROBE,
    ];

    /// The blueprint tessellation re-routed onto the bench's own lanes.
    #[must_use]
    pub fn from_static(s: StaticLanes) -> Self {
        let mut walls_outline = s.cuts;
        let mut walls_outline_count = s.cut_count;
        walls_outline.extend_from_slice(&s.hairlines);
        walls_outline_count += s.hairline_count;
        Self {
            slabs_pos: s.floor_pos,
            slabs_col: s.floor_col,
            slabs_idx: s.floor_idx,
            furniture_pos: s.furn_pos,
            furniture_col: s.furn_col,
            furniture_idx: s.furn_idx,
            walls: s.walls,
            wall_count: s.wall_count,
            walls_outline,
            walls_outline_count,
            portals: s.apertures,
            portal_count: s.aperture_count,
            portals_outline: s.arcs,
            portals_outline_count: s.arc_count,
            stairs: s.stairs,
            stairs_count: s.stairs_count,
            ..Self::default()
        }
    }
}

fn instance_in_band(inst: &Instance, band: [f64; 2]) -> bool {
    let (lo, hi) = inst.world_aabb();
    hi[1] >= band[0] && lo[1] <= band[1]
}

fn footprint_ring(inst: &Instance) -> [[f64; 2]; 4] {
    let (lo, hi) = inst.world_aabb();
    rect_corners(
        [(lo[0] + hi[0]) * 0.5, (lo[2] + hi[2]) * 0.5],
        [(hi[0] - lo[0]).max(0.05), (hi[2] - lo[2]).max(0.05)],
        0.0,
    )
}

fn ring_outline(out: &mut Vec<f32>, count: &mut u32, ring: &[[f64; 2]], col: [f32; 4]) {
    let n = ring.len();
    for i in 0..n {
        seg(out, to_world(ring[i]), to_world(ring[(i + 1) % n]), col);
        *count += 1;
    }
}

fn circle(center: [f64; 2], r: f64, n: usize) -> Vec<[f64; 2]> {
    (0..n)
        .map(|i| {
            let a = std::f64::consts::TAU * i as f64 / n as f64;
            [center[0] + r * a.cos(), center[1] + r * a.sin()]
        })
        .collect()
}

/// The leaf's swing arc: the free edge's path from closed to fully open, generated through the
/// hinge rotation itself (`local ∘ rot_y(θ)`), so the drawn side is always the modelled side.
fn leaf_arc(out: &mut Vec<f32>, count: &mut u32, inst: &Instance) {
    let Some(door) = inst.record.door else { return };
    if door.opened_distance.is_some() {
        return;
    }
    let len = (inst.bounds.1[0] - inst.bounds.0[0]).max(0.05);
    let hinge = [inst.local.t[0], inst.local.t[2]];
    let steps = 16;
    let mut prev: Option<[f64; 2]> = None;
    for i in 0..=steps {
        let f = f64::from(i) / f64::from(steps);
        let d = inst
            .local
            .compose(&Rigid::rot_y(
                door.closed_angle_deg + f * door.angle_range_deg,
            ))
            .dir([1.0, 0.0, 0.0]);
        let p = [hinge[0] + d[0] * len, hinge[1] + d[2] * len];
        if let Some(q) = prev {
            seg(out, to_world(q), to_world(p), geom::COL_ARC);
            *count += 1;
        }
        prev = Some(p);
    }
    if let Some(p) = prev {
        seg(out, to_world(hinge), to_world(p), geom::COL_ARC);
        *count += 1;
    }
}

/// Tessellate one (blueprint, drawing, compound, cuts, view) state. Without a compound the
/// blueprint draws everything ([`InteriorLanes::from_static`]); with one, its aperture / furniture
/// / arc annotations give way to the instances — the real geometry — routed by owner kind.
#[must_use]
pub fn build_interior_lanes(
    bp: &BuildingBlueprint,
    drawing: Option<&BuildingDrawing>,
    compound: Option<&CompoundBuilding>,
    cuts: Option<&[LevelCuts]>,
    view: ViewFloor,
) -> InteriorLanes {
    let mut out = InteriorLanes::from_static(geom::build_static_lanes(bp, drawing, view));
    let Some(c) = compound else {
        return out;
    };
    // The instances replace the blueprint's annotations of the same things.
    out.portals.clear();
    out.portal_count = 0;
    out.portals_outline.clear();
    out.portals_outline_count = 0;
    out.furniture_pos.clear();
    out.furniture_col.clear();
    out.furniture_idx.clear();
    let (band, _) = view.band(bp);

    if let ViewFloor::Level(i) = view {
        if let Some(lc) = cuts.and_then(|cs| cs.iter().find(|l| l.level_index == i)) {
            for (s, owner) in &lc.cuts {
                let Some(idx) = owner.checked_sub(1) else {
                    continue; // shell cuts are already the walls
                };
                let Some(inst) = c.instances.get(idx as usize) else {
                    continue;
                };
                let pts = [to_world(s[0]), to_world(s[1])];
                match inst.record.kind {
                    InstanceKind::DoorLeaf => {
                        let col = if inst.state.is_open() {
                            COL_DOOR_OPEN
                        } else {
                            COL_DOOR_CLOSED
                        };
                        push_strip(
                            &mut out.portals,
                            &expand_polyline_strip(&pts, LEAF_STRIP_M, col),
                        );
                        out.portal_count += 1;
                    }
                    InstanceKind::DoorFrame => {
                        push_strip(
                            &mut out.portals,
                            &expand_polyline_strip(&pts, geom::CUT_STRIP_M, COL_WALL_EXT),
                        );
                        out.portal_count += 1;
                    }
                    InstanceKind::WindowFrame => {
                        seg(&mut out.glazing_outline, pts[0], pts[1], COL_NORMAL);
                        out.glazing_outline_count += 1;
                    }
                    InstanceKind::Glass => {
                        push_strip(
                            &mut out.glazing,
                            &expand_polyline_strip(&pts, PANE_STRIP_M, COL_WINDOW),
                        );
                        out.glazing_count += 1;
                    }
                    // Furniture / props / trees are drawn as footprints below, not as cuts.
                    _ => {}
                }
            }
        }
    }

    for inst in &c.instances {
        match inst.record.kind {
            InstanceKind::DoorLeaf if inst.is_door() && instance_in_band(inst, band) => {
                out.leaf_count += 1;
                if inst.state.is_open() {
                    leaf_arc(
                        &mut out.portals_outline,
                        &mut out.portals_outline_count,
                        inst,
                    );
                }
            }
            InstanceKind::Glass if instance_in_band(inst, band) => out.pane_count += 1,
            InstanceKind::Furniture | InstanceKind::Prop if instance_in_band(inst, band) => {
                let col = match (inst.record.kind, inst.record.cover) {
                    (InstanceKind::Prop, _) => COL_PROP,
                    (_, CoverTier::Full) => COL_FURN_FULL,
                    (_, CoverTier::Low) => COL_FURN_LOW,
                    (_, CoverTier::None) => COL_FURN_NONE,
                };
                let ring = footprint_ring(inst);
                append_polygon(
                    &mut out.furniture_pos,
                    &mut out.furniture_col,
                    &mut out.furniture_idx,
                    &ring,
                    col,
                );
                ring_outline(
                    &mut out.furniture_outline,
                    &mut out.furniture_outline_count,
                    &ring,
                    [col[0], col[1], col[2], 1.0],
                );
                out.furniture_count += 1;
            }
            InstanceKind::Tree | InstanceKind::TreeCanopy => {
                // Every view: trees stand outside, above every floor.
                let place = inst.placement();
                let centre = [place.t[0], place.t[2]];
                let has_foliage = inst.blas.kinds.contains(&SurfaceKind::Foliage);
                let r = if has_foliage {
                    ((inst.bounds.1[0] - inst.bounds.0[0]).max(inst.bounds.1[2] - inst.bounds.0[2])
                        * 0.5
                        * place.scale)
                        .max(0.5)
                } else {
                    TRUNK_R_M * 2.0
                };
                let canopy = circle(centre, r, 24);
                append_polygon(
                    &mut out.vegetation_pos,
                    &mut out.vegetation_col,
                    &mut out.vegetation_idx,
                    &canopy,
                    COL_CANOPY,
                );
                ring_outline(
                    &mut out.vegetation_outline,
                    &mut out.vegetation_outline_count,
                    &canopy,
                    COL_CANOPY_EDGE,
                );
                for k in 0..8 {
                    let a = std::f64::consts::TAU * f64::from(k) / 8.0;
                    let inner = [
                        centre[0] + 0.55 * r * a.cos(),
                        centre[1] + 0.55 * r * a.sin(),
                    ];
                    let outer = [centre[0] + 0.9 * r * a.cos(), centre[1] + 0.9 * r * a.sin()];
                    seg(
                        &mut out.vegetation_outline,
                        to_world(inner),
                        to_world(outer),
                        COL_CANOPY_EDGE,
                    );
                    out.vegetation_outline_count += 1;
                }
                append_polygon(
                    &mut out.vegetation_pos,
                    &mut out.vegetation_col,
                    &mut out.vegetation_idx,
                    &circle(centre, TRUNK_R_M, 12),
                    COL_TRUNK,
                );
                out.tree_count += 1;
            }
            _ => {}
        }
    }
    out
}

/// The door leaf under local plan point `p` on a view of `band`: the leaf where it hangs now, or
/// its closed footprint (the aperture) — either counts, so an open door closes on click too.
#[must_use]
pub fn door_at(c: &CompoundBuilding, p: [f64; 2], band: [f64; 2]) -> Option<String> {
    let inside = |inst: &Instance, frame: &Rigid| {
        let q = frame.inverse().point([p[0], 0.0, p[1]]);
        let (lo, hi) = inst.bounds;
        q[0] >= lo[0] - DOOR_HIT_SLACK_M
            && q[0] <= hi[0] + DOOR_HIT_SLACK_M
            && q[2] >= lo[2] - DOOR_HIT_SLACK_M
            && q[2] <= hi[2] + DOOR_HIT_SLACK_M
    };
    c.doors()
        .filter(|inst| instance_in_band(inst, band))
        .find(|inst| inside(inst, &inst.placement()) || inside(inst, &inst.local))
        .map(|inst| inst.record.id.clone())
}

/// Ray strip + event dots for `INTERIOR_PROBE`, clipped to the ACTIVE view's elevation band
/// (`band`/`band_last` from [`ViewFloor::band`], intersected via the raycaster's own
/// [`clip_t_to_band`] so display and evaluation cannot disagree). Spans between consecutive
/// hits are coloured by a state machine over the trace: clear → green, after glass → cyan,
/// after canopy → yellow-green, after furniture cover → yellow; from a terminal block (wall,
/// roof, solid, leaf, frame, prop, full cover) to the target → red. Dots draw only where the
/// hit's own elevation lies inside the band. Returns `(packed, item_count)` — empty when the ray
/// never enters the band (that floor's plan honestly shows no ray).
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
            LosHitKind::Wall | LosHitKind::Roof | LosHitKind::Solid => RAY_BLOCKED,
            // A terminal window / stairs hit is frame or tread MASS, not a pass.
            LosHitKind::Window | LosHitKind::Stairs if h.concealment >= 1.0 => RAY_BLOCKED,
            LosHitKind::Furniture if h.concealment >= 1.0 => RAY_BLOCKED,
            LosHitKind::Window | LosHitKind::Glass => RAY_GLASS,
            LosHitKind::Foliage => RAY_FOLIAGE,
            LosHitKind::Furniture => RAY_COVER,
            LosHitKind::DoorOpen | LosHitKind::DoorAperture | LosHitKind::Stairs => color,
            LosHitKind::DoorLeaf
            | LosHitKind::DoorFrame
            | LosHitKind::WindowFrame
            | LosHitKind::Prop => RAY_BLOCKED,
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
            LosHitKind::Wall | LosHitKind::Roof | LosHitKind::Solid => RAY_BLOCKED,
            LosHitKind::Window | LosHitKind::Stairs if h.concealment >= 1.0 => RAY_BLOCKED,
            LosHitKind::Window | LosHitKind::Glass => RAY_GLASS,
            LosHitKind::DoorOpen | LosHitKind::DoorAperture => RAY_CLEAR,
            LosHitKind::Foliage => RAY_FOLIAGE,
            LosHitKind::Furniture => RAY_COVER,
            LosHitKind::Stairs => COL_HATCH,
            LosHitKind::DoorLeaf
            | LosHitKind::DoorFrame
            | LosHitKind::WindowFrame
            | LosHitKind::Prop => RAY_BLOCKED,
        };
        let c = [h.pos[0], h.pos[2]];
        quad(&mut packed, rect_corners(c, [0.34, 0.34], 45.0), col);
        count += 1;
    }
    (packed, count)
}

#[cfg(test)]
#[path = "building_interior_tests.rs"]
mod tests;
