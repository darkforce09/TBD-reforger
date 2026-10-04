//! Interior plan geometry for the building bench, on the bench's own render lanes.
//!
//! **Role:** tessellates a compound building into the `INTERIOR_*` and `SCENE_*` lanes — the
//! shell's section cuts become the walls, door leaves draw where they hang (orange closed, green
//! open, with their swing arc), glass panes are cyan strips, window frames jamb ticks, furniture
//! and props footprints coloured by cover tier, scene trees a trunk disc plus canopy — and flies
//! the line-of-sight probe on a lane of its own above the viewshed wash. Without a compound, the
//! blueprint-only tessellation of [`crate::building_viewer::geom::build_static_lanes`] is routed
//! lane by lane through [`crate::building_interior::InteriorLanes::from_static`], so the bench
//! never borrows a terrain lane.
//! **Position:** the pure half of the building bench, below [`crate::building_viewer`], whose wasm
//! host uploads what this module packs. [`crate::world_los_scene`] shares the same lane set.
//! **Signals & state:** none. Every function is a pure transform from blueprint or compound
//! geometry to packed vertices, which is what lets the whole module be tested natively.
//! **Invariants:** lane ids come from the render crate's own table
//! ([`map_draw_lanes::lane_roles::role_id`]) on every target, and `lane_ids_match_the_render_crate`
//! pins each id the bench draws on against that table's source. Coordinates are the building's
//! local plan frame in metres.

use building_interiors::blueprint::sight_line::LosHit;
use building_interiors::blueprint::sight_line::LosHitKind;
use building_interiors::blueprint::sight_line::clip_t_to_band;
use building_interiors::blueprint::structure::BuildingBlueprint;
use building_interiors::compound::assembly::CompoundBuilding;
use building_interiors::compound::assembly::CoverTier;
use building_interiors::compound::instances::Instance;
use building_interiors::compound::instances::InstanceKind;
use building_interiors::section::cutter::BuildingDrawing;
use building_interiors::section::cutter::CUT_MAX_NY;
use building_interiors::section::cutter::Seg2;
use building_interiors::section::cutter::section_at_owned;
use geometry_primitives::rigid_transform::Rigid;
use map_draw_lanes::lane_roles::role_id;
use road_network::styling::expand_polyline_strip;
use spatial_indexes::bounding_volume_hierarchy::surface_kind::SurfaceKind;

use super::building_viewer::ViewFloor;
use super::building_viewer::geom::{
    self, COL_DOOR_CLOSED, COL_DOOR_OPEN, COL_FURN_FULL, COL_FURN_LOW, COL_FURN_NONE, COL_HATCH,
    COL_NORMAL, COL_WALL_EXT, COL_WINDOW, RAY_BLOCKED, RAY_CLEAR, RAY_COVER, RAY_GLASS,
    StaticLanes, append_polygon, push_strip, quad, rect_corners, seg, to_world,
};

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
    /// Index of the level in the blueprint's level list.
    pub level_index: usize,
    /// The level's main cut height in metres.
    pub y: f64,
    /// The owned section segments at that height, each with the id of the part that owns it.
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
    /// Per-vertex linear RGBA of the slab mesh.
    pub slabs_col: Vec<f32>,
    /// Triangle indices of the slab mesh.
    pub slabs_idx: Vec<u32>,
    /// `INTERIOR_FURNITURE` polygon mesh: furniture / prop footprints by cover tier.
    pub furniture_pos: Vec<f32>,
    /// Per-vertex linear RGBA of the furniture mesh.
    pub furniture_col: Vec<f32>,
    /// Triangle indices of the furniture mesh.
    pub furniture_idx: Vec<u32>,
    /// `INTERIOR_FURNITURE_OUTLINE` hairlines.
    pub furniture_outline: Vec<f32>,
    /// Number of hairline segments in `furniture_outline`.
    pub furniture_outline_count: u32,
    /// `INTERIOR_WALLS` strip tris: wall section cuts (or blueprint walls without a mesh).
    pub walls: Vec<f32>,
    /// Number of strips in `walls`.
    pub wall_count: u32,
    /// `INTERIOR_WALLS_OUTLINE` hairlines: cut twin, low cut, rings, ghosts, window normals.
    pub walls_outline: Vec<f32>,
    /// Number of hairline segments in `walls_outline`.
    pub walls_outline_count: u32,
    /// `INTERIOR_PORTALS` strip tris: door leaves where they hang + door frames.
    pub portals: Vec<f32>,
    /// Number of strips in `portals`.
    pub portal_count: u32,
    /// `INTERIOR_PORTALS_OUTLINE` hairlines: swing arcs.
    pub portals_outline: Vec<f32>,
    /// Number of hairline segments in `portals_outline`.
    pub portals_outline_count: u32,
    /// `INTERIOR_GLAZING` strip tris: glass pane cuts.
    pub glazing: Vec<f32>,
    /// Number of strips in `glazing`.
    pub glazing_count: u32,
    /// `INTERIOR_GLAZING_OUTLINE` hairlines: window-frame jamb ticks.
    pub glazing_outline: Vec<f32>,
    /// Number of hairline segments in `glazing_outline`.
    pub glazing_outline_count: u32,
    /// `INTERIOR_STAIRS` hairlines: tread hatch.
    pub stairs: Vec<f32>,
    /// Number of hairline segments in `stairs`.
    pub stairs_count: u32,
    /// `SCENE_VEGETATION` polygon mesh: trunk discs + canopies.
    pub vegetation_pos: Vec<f32>,
    /// Per-vertex linear RGBA of the vegetation mesh.
    pub vegetation_col: Vec<f32>,
    /// Triangle indices of the vegetation mesh.
    pub vegetation_idx: Vec<u32>,
    /// `SCENE_VEGETATION_OUTLINE` hairlines: canopy rims + stipple.
    pub vegetation_outline: Vec<f32>,
    /// Number of hairline segments in `vegetation_outline`.
    pub vegetation_outline_count: u32,
    /// Diagnostics: door leaves drawn on this view.
    pub leaf_count: u32,
    /// Diagnostics: glass panes drawn on this view.
    pub pane_count: u32,
    /// Diagnostics: furniture and prop footprints drawn on this view.
    pub furniture_count: u32,
    /// Diagnostics: scene trees drawn on this view.
    pub tree_count: u32,
}

impl InteriorLanes {
    /// The upload ids, in lane order — every one ≥ `INTERIOR_SLABS`; the probe is last. The
    /// lane-order tests pin the uploaders' role choice against it.
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

    /// The blueprint tessellation routed onto the bench's own lanes.
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

    if let ViewFloor::Level(i) = view
        && let Some(lc) = cuts.and_then(|cs| cs.iter().find(|l| l.level_index == i))
    {
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

mod ray_lanes;
pub use ray_lanes::{build_ray_lane, door_at};

#[cfg(test)]
#[path = "tests/building_interior.rs"]
mod tests;
