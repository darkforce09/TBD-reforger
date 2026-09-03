//! T-090.11.6 — `building_interior` tests (lane ids pinned against the render crate, lane
//! routing, door toggle + arc, glazing, trees, `door_at`, ray colours), split out per the
//! `#[path]` precedent.

use std::collections::HashMap;
use std::sync::Arc;

use super::*;
use map_engine_core::building_compound::{
    DoorRecord, DoorState, InstanceRecord, LocalTransform, PlacementSource,
};
use map_engine_core::bvh::{Bvh, BvhSidecar};

fn farmhouse() -> BuildingBlueprint {
    serde_json::from_str(include_str!(
        "../../../../../../packages/map-assets/everon/prefabs/buildings/FarmHouse_E_1L01.json"
    ))
    .expect("farmhouse blueprint parses")
}

fn cube(center: [f64; 3], half: [f64; 3]) -> (Vec<[f64; 3]>, Vec<[u32; 3]>) {
    let mut verts = Vec::new();
    for corner in 0..8u32 {
        verts.push([
            center[0] + if corner & 1 != 0 { half[0] } else { -half[0] },
            center[1] + if corner & 2 != 0 { half[1] } else { -half[1] },
            center[2] + if corner & 4 != 0 { half[2] } else { -half[2] },
        ]);
    }
    const QUADS: [[u32; 4]; 6] = [
        [0, 4, 6, 2],
        [1, 3, 7, 5],
        [0, 1, 5, 4],
        [2, 6, 7, 3],
        [0, 2, 3, 1],
        [4, 5, 7, 6],
    ];
    let mut tris = Vec::new();
    for q in QUADS {
        tris.push([q[0], q[1], q[2]]);
        tris.push([q[0], q[2], q[3]]);
    }
    (verts, tris)
}

fn blas(center: [f64; 3], half: [f64; 3], kind: SurfaceKind) -> Arc<BvhSidecar> {
    let (verts, tris) = cube(center, half);
    let bvh = Bvh::build(&verts, &tris);
    let kinds = vec![kind; tris.len()];
    Arc::new(BvhSidecar {
        verts,
        tris,
        bvh,
        kinds,
    })
}

fn record(
    id: &str,
    kind: InstanceKind,
    blas: &str,
    local: &Rigid,
    door: Option<DoorRecord>,
) -> InstanceRecord {
    InstanceRecord {
        id: id.into(),
        kind,
        prefab: format!("Prefabs/{id}.et"),
        blas: blas.into(),
        xob: None,
        local: LocalTransform::from_rigid(local),
        door,
        cover: CoverTier::Low,
        source: PlacementSource::PrefabCoords,
        parent: None,
    }
}

fn door(range: f64) -> DoorRecord {
    DoorRecord {
        angle_range_deg: range,
        closed_angle_deg: 0.0,
        initial_angle_deg: 0.0,
        angle_range_explicit: true,
        opened_distance: None,
    }
}

/// A far-away shell (never cut) + leaf + pane + table + tree, all placed on the farmhouse's
/// ground floor near the origin.
fn compound() -> CompoundBuilding {
    let mut map = HashMap::new();
    map.insert(
        "shell".to_string(),
        blas([500.0, 0.0, 500.0], [0.5; 3], SurfaceKind::Opaque),
    );
    map.insert(
        "blas/leaf.bvh".to_string(),
        blas([0.45, 1.0, 0.0], [0.45, 1.0, 0.03], SurfaceKind::Opaque),
    );
    map.insert(
        "blas/pane.bvh".to_string(),
        blas([0.0, 0.0, 0.0], [0.5, 0.5, 0.002], SurfaceKind::Glass),
    );
    map.insert(
        "blas/table.bvh".to_string(),
        blas([0.0, 0.4, 0.0], [0.8, 0.4, 0.5], SurfaceKind::Opaque),
    );
    map.insert(
        "blas/tree.bvh".to_string(),
        blas([0.0, 6.0, 0.0], [3.0, 3.0, 3.0], SurfaceKind::Foliage),
    );
    let shell = Arc::clone(&map["shell"]);
    let records = vec![
        record(
            "door/leaf",
            InstanceKind::DoorLeaf,
            "blas/leaf.bvh",
            &Rigid::translation([-1.0, 0.0, 2.0]),
            Some(door(90.0)),
        ),
        record(
            "win/pane",
            InstanceKind::Glass,
            "blas/pane.bvh",
            &Rigid::translation([2.0, 1.5, -3.0]),
            None,
        ),
        record(
            "table",
            InstanceKind::Furniture,
            "blas/table.bvh",
            &Rigid::translation([1.0, 0.0, 0.0]),
            None,
        ),
        record(
            "tree_nw",
            InstanceKind::Tree,
            "blas/tree.bvh",
            &Rigid::translation([-15.0, 0.0, -3.0]),
            None,
        ),
    ];
    CompoundBuilding::assemble(shell, &records, &map).unwrap()
}

fn cuts_at(c: &CompoundBuilding, y: f64) -> Vec<LevelCuts> {
    let flat = c.flatten();
    vec![LevelCuts {
        level_index: 0,
        y,
        cuts: section_at_owned(&flat.mesh, &flat.owner, y, CUT_MAX_NY),
    }]
}

fn colours(packed: &[f32]) -> Vec<[f32; 4]> {
    packed
        .chunks_exact(6)
        .map(|c| [c[2], c[3], c[4], c[5]])
        .collect()
}

fn strip_centroid(packed: &[f32]) -> [f32; 2] {
    let n = (packed.len() / 6).max(1) as f32;
    let (mut x, mut y) = (0.0f32, 0.0f32);
    for v in packed.chunks_exact(6) {
        x += v[0];
        y += v[1];
    }
    [x / n, y / n]
}

/// The native `role_id` mirror above must equal the render crate's table, value for value.
#[test]
fn lane_ids_match_the_render_crate() {
    const SRC: &str = include_str!("../../../../../../crates/map-engine-render/src/draw_order.rs");
    for (name, value) in [
        ("LANDCOVER", role_id::LANDCOVER),
        ("CONTOURS", role_id::CONTOURS),
        ("ROADS_CASING", role_id::ROADS_CASING),
        ("ROADS", role_id::ROADS),
        ("FOREST_OUTLINE", role_id::FOREST_OUTLINE),
        ("AIRFIELD_APRON", role_id::AIRFIELD_APRON),
        ("MISSION_ZONES", role_id::MISSION_ZONES),
        ("INTERIOR_SLABS", role_id::INTERIOR_SLABS),
        ("INTERIOR_FURNITURE", role_id::INTERIOR_FURNITURE),
        (
            "INTERIOR_FURNITURE_OUTLINE",
            role_id::INTERIOR_FURNITURE_OUTLINE,
        ),
        ("INTERIOR_WALLS", role_id::INTERIOR_WALLS),
        ("INTERIOR_WALLS_OUTLINE", role_id::INTERIOR_WALLS_OUTLINE),
        ("INTERIOR_PORTALS", role_id::INTERIOR_PORTALS),
        (
            "INTERIOR_PORTALS_OUTLINE",
            role_id::INTERIOR_PORTALS_OUTLINE,
        ),
        ("INTERIOR_GLAZING", role_id::INTERIOR_GLAZING),
        (
            "INTERIOR_GLAZING_OUTLINE",
            role_id::INTERIOR_GLAZING_OUTLINE,
        ),
        ("INTERIOR_STAIRS", role_id::INTERIOR_STAIRS),
        ("SCENE_VEGETATION", role_id::SCENE_VEGETATION),
        (
            "SCENE_VEGETATION_OUTLINE",
            role_id::SCENE_VEGETATION_OUTLINE,
        ),
        ("INTERIOR_PROBE", role_id::INTERIOR_PROBE),
    ] {
        let needle = format!("pub const {name}: u32 = {value};");
        assert!(
            SRC.contains(&needle),
            "render crate lost `{needle}` — renumber both sides"
        );
    }
    assert!(SRC.contains("pub const MAX: u32 = INTERIOR_PROBE;"));
}

#[test]
fn walls_never_use_borrowed_lanes() {
    let roles = InteriorLanes::ROLES;
    assert_eq!(roles.len(), 13);
    assert!(roles
        .iter()
        .all(|&r| (role_id::INTERIOR_SLABS..=role_id::MAX).contains(&r)));
    let mut sorted = roles.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), 13, "distinct");
    assert_eq!(*roles.last().unwrap(), role_id::INTERIOR_PROBE);
    for borrowed in [
        role_id::LANDCOVER,
        role_id::AIRFIELD_APRON,
        role_id::ROADS_CASING,
        role_id::ROADS,
        role_id::CONTOURS,
        role_id::FOREST_OUTLINE,
        role_id::MISSION_ZONES,
    ] {
        assert!(
            !roles.contains(&borrowed),
            "borrowed lane {borrowed} must not be used"
        );
    }
    // The blueprint-only path lands its payloads on the bench lanes with nothing lost.
    let bp = farmhouse();
    let s = geom::build_static_lanes(&bp, None, ViewFloor::Level(0));
    let (walls, hair, cutn, arcs, stairs) = (
        s.wall_count,
        s.hairline_count,
        s.cut_count,
        s.arc_count,
        s.stairs_count,
    );
    let l = InteriorLanes::from_static(s);
    assert_eq!(l.wall_count, walls);
    assert_eq!(l.walls_outline_count, hair + cutn);
    assert_eq!(l.portals_outline_count, arcs);
    assert_eq!(l.stairs_count, stairs);
    assert!(!l.slabs_idx.is_empty());
}

#[test]
fn door_toggle_moves_leaf_and_arc() {
    let bp = farmhouse();
    let mut c = compound();
    let closed = build_interior_lanes(
        &bp,
        None,
        Some(&c),
        Some(&cuts_at(&c, 1.2)),
        ViewFloor::Level(0),
    );
    assert_eq!(closed.leaf_count, 1);
    assert!(closed.portal_count >= 1, "{closed:?}");
    assert!(colours(&closed.portals).contains(&COL_DOOR_CLOSED));
    assert!(!colours(&closed.portals).contains(&COL_DOOR_OPEN));
    assert_eq!(closed.portals_outline_count, 0, "no arc while closed");
    let closed_centroid = strip_centroid(&closed.portals);

    assert!(c.set_door("door/leaf", DoorState::OPEN));
    let open = build_interior_lanes(
        &bp,
        None,
        Some(&c),
        Some(&cuts_at(&c, 1.2)),
        ViewFloor::Level(0),
    );
    assert!(colours(&open.portals).contains(&COL_DOOR_OPEN));
    assert!(!colours(&open.portals).contains(&COL_DOOR_CLOSED));
    assert!(
        open.portals_outline_count >= 17,
        "{}",
        open.portals_outline_count
    );
    let open_centroid = strip_centroid(&open.portals);
    let moved = ((open_centroid[0] - closed_centroid[0]).powi(2)
        + (open_centroid[1] - closed_centroid[1]).powi(2))
    .sqrt();
    assert!(moved > 0.3, "leaf centroid moved {moved} m");
    // The blueprint's own aperture overlays and arcs are gone: the instances replaced them.
    let plain =
        InteriorLanes::from_static(geom::build_static_lanes(&bp, None, ViewFloor::Level(0)));
    assert!(
        plain.portal_count > 0,
        "the blueprint path still draws its overlays"
    );
    // … every portal vertex now sits at the one leaf.
    let hinge = to_world([-1.0, 2.0]);
    for v in open.portals.chunks_exact(6) {
        let d =
            ((f64::from(v[0]) - hinge[0]).powi(2) + (f64::from(v[1]) - hinge[1]).powi(2)).sqrt();
        assert!(d < 1.6, "portal vertex {d} m from the hinge");
    }
}

#[test]
fn glass_cut_lands_on_glazing_lane() {
    let bp = farmhouse();
    let c = compound();
    let l = build_interior_lanes(
        &bp,
        None,
        Some(&c),
        Some(&cuts_at(&c, 1.5)),
        ViewFloor::Level(0),
    );
    assert_eq!(l.pane_count, 1);
    assert!(l.glazing_count >= 1, "{l:?}");
    assert!(colours(&l.glazing).iter().all(|c| *c == COL_WINDOW));
    assert!(!colours(&l.walls).contains(&COL_WINDOW));
    // The table draws as a low-cover footprint with an outline, on the furniture lanes.
    assert_eq!(l.furniture_count, 1);
    assert!(!l.furniture_idx.is_empty());
    assert_eq!(l.furniture_outline_count, 4);
    assert!(l.furniture_col.chunks_exact(4).all(|c| c == COL_FURN_LOW));
}

#[test]
fn scene_trees_paint_canopy_and_trunk() {
    let bp = farmhouse();
    let c = compound();
    for view in [ViewFloor::Level(0), ViewFloor::Roof] {
        let l = build_interior_lanes(&bp, None, Some(&c), None, view);
        assert_eq!(l.tree_count, 1, "{view:?}");
        // Canopy 24-gon + trunk 12-gon.
        assert_eq!(l.vegetation_pos.len(), (24 + 12) * 2);
        assert!(l.vegetation_col.chunks_exact(4).any(|c| c == COL_CANOPY));
        assert!(l.vegetation_col.chunks_exact(4).any(|c| c == COL_TRUNK));
        assert_eq!(l.vegetation_outline_count, 24 + 8);
    }
}

#[test]
fn door_at_hits_leaf_and_closed_footprint() {
    let bp = farmhouse();
    let (band, _) = ViewFloor::Level(0).band(&bp);
    let mut c = compound();
    // Closed leaf spans x −1.0..−0.1 at z = 2.
    assert_eq!(door_at(&c, [-0.5, 2.0], band).as_deref(), Some("door/leaf"));
    assert_eq!(door_at(&c, [-0.5, 3.5], band), None);
    assert_eq!(door_at(&c, [2.0, -3.0], band), None, "a pane is not a door");
    c.set_door("door/leaf", DoorState::OPEN);
    // The aperture (closed footprint) still toggles it …
    assert_eq!(door_at(&c, [-0.5, 2.0], band).as_deref(), Some("door/leaf"));
    // … and so does the swung leaf (a +90° swing turns +x toward −z: x ≈ −1, z ∈ 1.1..2).
    assert_eq!(door_at(&c, [-1.0, 1.5], band).as_deref(), Some("door/leaf"));
    // Out of the viewed band, nothing.
    assert_eq!(door_at(&c, [-0.5, 2.0], [40.0, 50.0]), None);
}

const GROUND_BAND: [f64; 2] = [0.0, 2.8];

fn hit(t: f64, kind: LosHitKind, conceal: f64) -> LosHit {
    LosHit {
        t,
        pos: [0.0, 1.4, -8.0 + t * 7.0],
        kind,
        id: "x".into(),
        concealment: conceal,
    }
}

fn lane_colours(kind: LosHitKind, conceal: f64, clear: bool) -> Vec<[f32; 4]> {
    let (packed, _) = build_ray_lane(
        [0.0, 1.4, -8.0],
        [0.0, 1.4, -1.0],
        &[hit(0.5, kind, conceal)],
        clear,
        GROUND_BAND,
        false,
    );
    colours(&packed)
}

#[test]
fn ray_lane_colours_glass_and_foliage() {
    let c = lane_colours(LosHitKind::Glass, 0.05, true);
    assert!(c.contains(&RAY_CLEAR) && c.contains(&RAY_GLASS) && !c.contains(&RAY_BLOCKED));
    let c = lane_colours(LosHitKind::Foliage, 0.6, true);
    assert!(c.contains(&RAY_FOLIAGE) && !c.contains(&RAY_BLOCKED));
    let c = lane_colours(LosHitKind::DoorAperture, 0.0, true);
    assert!(c.iter().all(|k| *k == RAY_CLEAR), "{c:?}");
    for kind in [
        LosHitKind::DoorLeaf,
        LosHitKind::DoorFrame,
        LosHitKind::WindowFrame,
        LosHitKind::Prop,
    ] {
        let c = lane_colours(kind.clone(), 1.0, false);
        assert!(c.contains(&RAY_BLOCKED), "{kind:?}");
    }
}

#[test]
fn ray_lane_colors_follow_the_hit_state_machine() {
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
    let c = colours(&packed);
    assert!(c.contains(&RAY_CLEAR) && c.contains(&RAY_GLASS));
    // blocked wall / roof / solid → red span present.
    for kind in [LosHitKind::Wall, LosHitKind::Roof, LosHitKind::Solid] {
        assert!(
            lane_colours(kind.clone(), 1.0, false).contains(&RAY_BLOCKED),
            "{kind:?}"
        );
    }
    // a TERMINAL window hit is frame mass: red, never cyan.
    let c = lane_colours(LosHitKind::Window, 1.0, false);
    assert!(c.contains(&RAY_BLOCKED) && !c.contains(&RAY_GLASS));
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
