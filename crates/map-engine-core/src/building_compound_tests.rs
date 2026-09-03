//! T-090.11.4 — compound-building tests: hinge state, glass and foliage semantics, transform
//! precision, flatten / owned section cuts, assembly errors and the wash over instances.
//! Split out per the `#[path]` precedent.

use std::collections::HashMap;
use std::sync::Arc;

use super::*;
use crate::building_compound::{
    CompoundError, CoverTier, DoorRecord, DoorState, FlatMesh, InstanceRecord, LocalTransform,
    PlacementSource,
};
use crate::building_section::section_at_owned;
use crate::building_viewshed::{WashParams, compound_wash};
use crate::bvh::{Bvh, BvhSidecar};
use crate::dem::sample::Visibility;

type Scene = (Vec<[f64; 3]>, Vec<[u32; 3]>);

fn cube(center: [f64; 3], half: [f64; 3]) -> Scene {
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

fn concat(scenes: &[Scene]) -> Scene {
    let mut verts = Vec::new();
    let mut tris = Vec::new();
    for (v, t) in scenes {
        let base = verts.len() as u32;
        verts.extend_from_slice(v);
        tris.extend(
            t.iter()
                .map(|tri| [tri[0] + base, tri[1] + base, tri[2] + base]),
        );
    }
    (verts, tris)
}

fn sidecar(scene: Scene, kind: SurfaceKind) -> Arc<BvhSidecar> {
    let (verts, tris) = scene;
    let bvh = Bvh::build(&verts, &tris);
    let kinds = vec![kind; tris.len()];
    Arc::new(BvhSidecar {
        verts,
        tris,
        bvh,
        kinds,
    })
}

fn box_blas(min: [f64; 3], max: [f64; 3], kind: SurfaceKind) -> Arc<BvhSidecar> {
    let c = [
        (min[0] + max[0]) * 0.5,
        (min[1] + max[1]) * 0.5,
        (min[2] + max[2]) * 0.5,
    ];
    let h = [
        (max[0] - min[0]) * 0.5,
        (max[1] - min[1]) * 0.5,
        (max[2] - min[2]) * 0.5,
    ];
    sidecar(cube(c, h), kind)
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
        cover: CoverTier::None,
        source: PlacementSource::PrefabCoords,
        parent: None,
    }
}

/// A 10 × 10 × 3 m room: floor slab, four 0.2 m walls, a 1 m doorway in the +z wall at
/// x ∈ [−0.5, 0.5] and a 1 × 1 m window hole in the −z wall at x ∈ [1, 2], y ∈ [1, 2].
fn room() -> Arc<BvhSidecar> {
    let parts = vec![
        cube([0.0, -0.1, 0.0], [5.2, 0.1, 5.2]),
        cube([-2.75, 1.5, 5.0], [2.25, 1.5, 0.1]),
        cube([2.75, 1.5, 5.0], [2.25, 1.5, 0.1]),
        cube([-2.0, 1.5, -5.0], [3.0, 1.5, 0.1]),
        cube([3.5, 1.5, -5.0], [1.5, 1.5, 0.1]),
        cube([1.5, 0.5, -5.0], [0.5, 0.5, 0.1]),
        cube([1.5, 2.5, -5.0], [0.5, 0.5, 0.1]),
        cube([5.0, 1.5, 0.0], [0.1, 1.5, 5.0]),
        cube([-5.0, 1.5, 0.0], [0.1, 1.5, 5.0]),
    ];
    sidecar(concat(&parts), SurfaceKind::Opaque)
}

fn leaf_blas() -> Arc<BvhSidecar> {
    box_blas([0.0, 0.0, -0.03], [0.9, 2.0, 0.03], SurfaceKind::Opaque)
}

/// Room + one leaf hung at the doorway's left jamb (hinge at x = −0.45, z = 5).
fn room_with_door(range: f64) -> CompoundBuilding {
    let mut blas = HashMap::new();
    blas.insert("blas/leaf.bvh".to_string(), leaf_blas());
    let rec = record(
        "door/leaf",
        InstanceKind::DoorLeaf,
        "blas/leaf.bvh",
        &Rigid::translation([-0.45, 0.0, 5.0]),
        Some(door(range)),
    );
    CompoundBuilding::assemble(room(), &[rec], &blas).unwrap()
}

const RAY_A: ([f64; 3], [f64; 3]) = ([0.0, 1.2, 8.0], [0.0, 1.2, 0.0]);
const RAY_B: ([f64; 3], [f64; 3]) = ([-0.45, 1.2, 8.0], [-0.45, 1.2, 0.0]);
const RAY_C: ([f64; 3], [f64; 3]) = ([1.5, 1.5, -8.0], [1.5, 1.5, 0.0]);

fn kinds(r: &LosResult) -> Vec<LosHitKind> {
    r.hits.iter().map(|h| h.kind.clone()).collect()
}

#[test]
fn door_state_fraction_toggle_and_initial_angle() {
    assert_eq!(DoorState::Closed.fraction(), 0.0);
    assert_eq!(DoorState::Open { fraction: 2.0 }.fraction(), 1.0);
    assert_eq!(DoorState::Open { fraction: -1.0 }.fraction(), 0.0);
    assert_eq!(DoorState::Open { fraction: f64::NAN }.fraction(), 0.0);
    assert_eq!(DoorState::Closed.toggled(), DoorState::OPEN);
    assert_eq!(DoorState::OPEN.toggled(), DoorState::Closed);
    assert!(!DoorState::Open { fraction: 0.0 }.is_open());
    // A leaf whose prefab opens it 45° of 90° starts half open.
    let mut blas = HashMap::new();
    blas.insert("blas/leaf.bvh".to_string(), leaf_blas());
    let mut rec = record(
        "d",
        InstanceKind::DoorLeaf,
        "blas/leaf.bvh",
        &Rigid::identity(),
        Some(door(90.0)),
    );
    rec.door.as_mut().unwrap().initial_angle_deg = 45.0;
    let c = CompoundBuilding::assemble(room(), &[rec], &blas).unwrap();
    assert_eq!(c.door_state("d"), Some(DoorState::Open { fraction: 0.5 }));
    assert_eq!(c.door_state("nope"), None);
}

#[test]
fn closed_leaf_blocks_open_leaf_passes_with_an_aperture_and_blocks_where_it_swung() {
    let mut c = room_with_door(90.0);
    let (obs, tgt) = RAY_A;
    let r = c.evaluate_los(None, obs, tgt);
    assert!(!r.is_clear);
    assert_eq!(kinds(&r), [LosHitKind::DoorLeaf]);
    assert_eq!(r.hits[0].id, "door/leaf");
    assert_eq!(r.concealment, 1.0);
    assert!(c.blocked(obs, tgt));

    assert!(c.set_door("door/leaf", DoorState::OPEN));
    assert!(!c.set_door("nope", DoorState::OPEN));
    let r = c.evaluate_los(None, obs, tgt);
    assert!(r.is_clear, "{r:?}");
    assert_eq!(kinds(&r), [LosHitKind::DoorAperture]);
    // Entry into the closed leaf's box (z = 5.03, the BVH pads its bounds by ≈ 1 mm).
    assert!((r.hits[0].t - 2.97 / 8.0).abs() < 2e-4, "{}", r.hits[0].t);
    assert_eq!(r.door_ids_traversed, ["door/leaf"]);
    assert_eq!(r.concealment, 0.0);
    assert!(!c.blocked(obs, tgt));
    // A +90° swing turns the leaf's free edge from +x toward −z: it now hangs along the
    // jamb into the room, across RAY_B.
    let (obs_b, tgt_b) = RAY_B;
    let r = c.evaluate_los(None, obs_b, tgt_b);
    assert!(!r.is_clear);
    assert_eq!(r.hits.last().unwrap().kind, LosHitKind::DoorLeaf);
    let leaf = &c.instances[0];
    let (lo, hi) = leaf.world_aabb();
    assert!(
        (lo[2] - 4.1).abs() < 2e-3 && (hi[2] - 5.0).abs() < 2e-3,
        "{lo:?}..{hi:?}"
    );
    assert!(
        (lo[0] + 0.48).abs() < 2e-3 && (hi[0] + 0.42).abs() < 2e-3,
        "{lo:?}..{hi:?}"
    );
    // Over-open clamps to the full sweep.
    c.set_door("door/leaf", DoorState::Open { fraction: 2.0 });
    assert_eq!(c.placement(0), leaf_placement_at(&c, 1.0));

    // A −90° range swings the other way (outward, z 5..5.9): RAY_A clear, RAY_B blocked out front.
    let mut c2 = room_with_door(-90.0);
    c2.set_door("door/leaf", DoorState::OPEN);
    assert!(c2.evaluate_los(None, obs, tgt).is_clear);
    let r = c2.evaluate_los(None, obs_b, tgt_b);
    assert!(!r.is_clear);
    assert!((r.hits.last().unwrap().pos[2] - 5.9).abs() < 1e-6, "{r:?}");
}

fn leaf_placement_at(c: &CompoundBuilding, fraction: f64) -> Rigid {
    let mut i = c.instances[0].clone();
    i.state = DoorState::Open { fraction };
    i.placement()
}

#[test]
fn sliding_leaf_translates_along_its_local_x() {
    let mut blas = HashMap::new();
    blas.insert("blas/leaf.bvh".to_string(), leaf_blas());
    let mut rec = record(
        "slider",
        InstanceKind::DoorLeaf,
        "blas/leaf.bvh",
        &Rigid::translation([-0.45, 0.0, 5.0]),
        Some(door(0.0)),
    );
    rec.door.as_mut().unwrap().opened_distance = Some(0.9);
    let mut c = CompoundBuilding::assemble(room(), &[rec], &blas).unwrap();
    let (obs, tgt) = RAY_A;
    assert!(!c.evaluate_los(None, obs, tgt).is_clear);
    c.set_door("slider", DoorState::OPEN);
    let p = c.placement(0);
    assert_eq!(p.t, [0.45, 0.0, 5.0]);
    let r = c.evaluate_los(None, obs, tgt);
    assert!(
        r.is_clear && kinds(&r) == [LosHitKind::DoorAperture],
        "{r:?}"
    );
    c.set_door("slider", DoorState::Open { fraction: 0.5 });
    assert_eq!(c.placement(0).t, [0.0, 0.0, 5.0]);
}

fn pane_blas() -> Arc<BvhSidecar> {
    box_blas([-0.5, -0.5, -0.001], [0.5, 0.5, 0.001], SurfaceKind::Glass)
}

#[test]
fn glass_conceals_five_percent_per_pane_and_never_blocks() {
    let mut blas = HashMap::new();
    blas.insert("blas/pane.bvh".to_string(), pane_blas());
    let one = record(
        "win/pane",
        InstanceKind::Glass,
        "blas/pane.bvh",
        &Rigid::translation([1.5, 1.5, -5.0]),
        None,
    );
    let c = CompoundBuilding::assemble(room(), std::slice::from_ref(&one), &blas).unwrap();
    let (obs, tgt) = RAY_C;
    let r = c.evaluate_los(None, obs, tgt);
    assert!(r.is_clear, "{r:?}");
    assert_eq!(
        kinds(&r),
        [LosHitKind::Glass],
        "two faces merge into one pane event"
    );
    assert!((r.hits[0].concealment - GLASS_CONCEALMENT).abs() < 1e-12);
    assert!((r.concealment - 0.05).abs() < 1e-12);
    assert_eq!(r.window_ids_traversed, ["win/pane"]);
    assert!(!c.blocked(obs, tgt));
    // Two panes: 1 − 0.95².
    let mut two = one;
    two.id = "win/pane2".into();
    two.local = LocalTransform::from_rigid(&Rigid::translation([1.5, 1.5, -4.9]));
    let c = CompoundBuilding::assemble(
        room(),
        &[
            record(
                "win/pane",
                InstanceKind::Glass,
                "blas/pane.bvh",
                &Rigid::translation([1.5, 1.5, -5.0]),
                None,
            ),
            two,
        ],
        &blas,
    )
    .unwrap();
    let r = c.evaluate_los(None, obs, tgt);
    assert!(r.is_clear);
    assert_eq!(r.hits.len(), 2);
    assert!((r.concealment - 0.0975).abs() < 1e-9, "{}", r.concealment);
    assert_eq!(r.window_ids_traversed, ["win/pane", "win/pane2"]);
    // The room's own wall (no instance) still blocks a ray that misses the hole.
    let r = c.evaluate_los(None, [-2.0, 1.5, -8.0], [-2.0, 1.5, 0.0]);
    assert!(!r.is_clear && kinds(&r) == [LosHitKind::Solid], "{r:?}");
}

#[test]
fn window_frame_mass_blocks() {
    let mut blas = HashMap::new();
    blas.insert(
        "blas/frame.bvh".to_string(),
        box_blas([-0.5, -0.5, -0.05], [0.5, 0.5, 0.05], SurfaceKind::Opaque),
    );
    let c = CompoundBuilding::assemble(
        room(),
        &[record(
            "win/frame",
            InstanceKind::WindowFrame,
            "blas/frame.bvh",
            &Rigid::translation([1.5, 1.5, -5.0]),
            None,
        )],
        &blas,
    )
    .unwrap();
    let (obs, tgt) = RAY_C;
    let r = c.evaluate_los(None, obs, tgt);
    assert!(!r.is_clear);
    assert_eq!(r.hits.last().unwrap().kind, LosHitKind::WindowFrame);
    assert_eq!(r.hits.last().unwrap().id, "win/frame");
}

fn far_shell() -> Arc<BvhSidecar> {
    sidecar(cube([500.0, 0.0, 500.0], [0.5; 3]), SurfaceKind::Opaque)
}

#[test]
fn foliage_conceals_by_depth_and_trunks_block() {
    let mut blas = HashMap::new();
    blas.insert(
        "blas/canopy.bvh".to_string(),
        box_blas([-3.0; 3], [3.0; 3], SurfaceKind::Foliage),
    );
    blas.insert(
        "blas/thin.bvh".to_string(),
        box_blas([-3.0, -1.0, -0.25], [3.0, 1.0, 0.25], SurfaceKind::Foliage),
    );
    blas.insert(
        "blas/trunk.bvh".to_string(),
        box_blas([-0.2, 0.0, -0.2], [0.2, 4.0, 0.2], SurfaceKind::Opaque),
    );
    let at = Rigid::translation([0.0, 2.0, -12.0]);
    let canopy = record(
        "tree/canopy",
        InstanceKind::TreeCanopy,
        "blas/canopy.bvh",
        &at,
        None,
    );
    let thin = record("bush", InstanceKind::TreeCanopy, "blas/thin.bvh", &at, None);
    let trunk = record(
        "tree",
        InstanceKind::Tree,
        "blas/trunk.bvh",
        &Rigid::translation([0.0, 0.0, -12.0]),
        None,
    );
    let obs = [0.0, 2.0, -20.0];
    let tgt = [0.0, 2.0, -8.0];

    let c = CompoundBuilding::assemble(far_shell(), std::slice::from_ref(&canopy), &blas).unwrap();
    let r = c.evaluate_los(None, obs, tgt);
    assert!(r.is_clear);
    assert_eq!(kinds(&r), [LosHitKind::Foliage]);
    let want = 1.0 - (-FOLIAGE_K * 6.0).exp();
    assert!(
        (r.hits[0].concealment - want).abs() < 1e-9 && want >= 0.95,
        "{r:?}"
    );
    assert!((r.hits[0].t - 5.0 / 12.0).abs() < 1e-9);
    assert!(!c.blocked(obs, tgt));

    let c = CompoundBuilding::assemble(far_shell(), &[thin], &blas).unwrap();
    let r = c.evaluate_los(None, obs, tgt);
    assert!(r.is_clear);
    assert!((r.concealment - 0.22).abs() < 0.01, "{}", r.concealment);

    // Observer inside the canopy: the first crossing is the exit, depth counts from t = 0.
    let c = CompoundBuilding::assemble(far_shell(), std::slice::from_ref(&canopy), &blas).unwrap();
    let r = c.evaluate_los(None, [0.0, 2.0, -12.0], tgt);
    assert!(r.is_clear);
    assert_eq!(kinds(&r), [LosHitKind::Foliage]);
    assert_eq!(r.hits[0].t, 0.0);
    assert!((r.hits[0].concealment - (1.0 - (-FOLIAGE_K * 3.0).exp())).abs() < 1e-9);

    // Target inside the canopy: the entry is unpaired and runs to the segment end.
    let r = c.evaluate_los(None, obs, [0.0, 2.0, -12.0]);
    assert!(r.is_clear);
    assert!((r.hits[0].concealment - (1.0 - (-FOLIAGE_K * 3.0).exp())).abs() < 1e-9);

    // Trunk + canopy: blocked by the trunk, attributed as a prop with the tree's id, and the
    // canopy entered before it still reports its depth up to the block.
    let c = CompoundBuilding::assemble(far_shell(), &[canopy, trunk], &blas).unwrap();
    let r = c.evaluate_los(None, obs, tgt);
    assert!(!r.is_clear);
    assert_eq!(kinds(&r), [LosHitKind::Foliage, LosHitKind::Prop], "{r:?}");
    assert_eq!(r.hits[1].id, "tree");
    let depth = 12.0 * (r.hits[1].t - r.hits[0].t);
    assert!((depth - 2.8).abs() < 1e-6, "{depth}");
    assert!(c.blocked(obs, tgt));
}

/// Möller–Trumbore, both sides, `t ∈ [0, 1]` on the segment.
fn seg_tri(p: [f64; 3], q: [f64; 3], a: [f64; 3], b: [f64; 3], c: [f64; 3]) -> Option<f64> {
    let sub = |u: [f64; 3], v: [f64; 3]| [u[0] - v[0], u[1] - v[1], u[2] - v[2]];
    let cross = |u: [f64; 3], v: [f64; 3]| {
        [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ]
    };
    let dot = |u: [f64; 3], v: [f64; 3]| u[0] * v[0] + u[1] * v[1] + u[2] * v[2];
    let d = sub(q, p);
    let e1 = sub(b, a);
    let e2 = sub(c, a);
    let h = cross(d, e2);
    let det = dot(e1, h);
    if det.abs() < 1e-14 {
        return None;
    }
    let s = sub(p, a);
    let u = dot(s, h) / det;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let qv = cross(s, e1);
    let v = dot(d, qv) / det;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let t = dot(e2, qv) / det;
    (0.0..=1.0).contains(&t).then_some(t)
}

#[test]
fn transformed_instance_hits_match_the_hand_placed_mesh_to_a_micrometre() {
    let place = Rigid::from_enfusion([-8.87, 3.58, -5.29], [12.0, 137.5, -7.0], 1.152);
    let mut blas = HashMap::new();
    let unit = box_blas([-0.5; 3], [0.5; 3], SurfaceKind::Opaque);
    blas.insert("blas/crate.bvh".to_string(), Arc::clone(&unit));
    let c = CompoundBuilding::assemble(
        far_shell(),
        &[record(
            "crate",
            InstanceKind::Furniture,
            "blas/crate.bvh",
            &place,
            None,
        )],
        &blas,
    )
    .unwrap();
    // Straight at the centre from 3 m out along local −z: the first face is local z = −0.5.
    let obs = place.point([0.0, 0.0, -3.0]);
    let tgt = place.point([0.0, 0.0, 0.0]);
    let ev = c.trace(obs, tgt);
    assert!(!ev.is_empty());
    let want = place.point([0.0, 0.0, -0.5]);
    let got = ev[0].pos;
    let err = (0..3)
        .map(|k| (got[k] - want[k]).powi(2))
        .sum::<f64>()
        .sqrt();
    assert!(err < 1e-6, "{err}");
    assert_eq!(ev[0].owner, Owner::Instance(0));
    let r = c.evaluate_los(None, obs, tgt);
    assert_eq!(kinds(&r), [LosHitKind::Furniture]);
    assert_eq!(r.cover_furniture_id.as_deref(), Some("crate"));

    // Brute force over the hand-transformed triangles on a ray grid: the AABB cull never
    // drops a true hit and the inverse mapping never invents one.
    let world: Vec<[[f64; 3]; 3]> = unit
        .tris
        .iter()
        .map(|t| {
            [
                place.point(unit.verts[t[0] as usize]),
                place.point(unit.verts[t[1] as usize]),
                place.point(unit.verts[t[2] as usize]),
            ]
        })
        .collect();
    let centre = place.t;
    let mut checked = 0;
    let mut hits = 0;
    for ix in -6..=6 {
        for iy in -4..=4 {
            let a = [
                centre[0] + ix as f64 * 0.31 - 4.0,
                centre[1] + iy as f64 * 0.29,
                centre[2] - 4.0,
            ];
            let b = [
                centre[0] + ix as f64 * 0.17 + 3.0,
                centre[1] - iy as f64 * 0.11,
                centre[2] + 4.0,
            ];
            let brute = world
                .iter()
                .any(|t| seg_tri(a, b, t[0], t[1], t[2]).is_some());
            assert_eq!(c.blocked(a, b), brute, "ray {a:?}→{b:?}");
            let traced = c.trace(a, b).iter().any(|e| e.kind == SurfaceKind::Opaque);
            assert_eq!(traced, brute);
            checked += 1;
            hits += usize::from(brute);
        }
    }
    assert_eq!(checked, 117);
    assert!(hits > 10 && hits < checked, "{hits} of {checked}");
}

#[test]
fn flatten_bakes_instances_with_owners_and_owned_cuts_tag_the_leaf() {
    let mut c = room_with_door(90.0);
    let shell_tris = c.shell.tris.len();
    let leaf_tris = c.instances[0].blas.tris.len();
    let flat = c.flatten();
    assert_eq!(flat.mesh.tris.len(), shell_tris + leaf_tris);
    assert_eq!(flat.owner.len(), flat.mesh.tris.len());
    assert_eq!(flat.mesh.kinds.len(), flat.mesh.tris.len());
    assert_eq!(flat.owner.iter().filter(|&&o| o == 0).count(), shell_tris);
    assert_eq!(flat.owner.iter().filter(|&&o| o == 1).count(), leaf_tris);
    assert_eq!(flat.owner_of(0), None);
    assert_eq!(flat.owner_of(shell_tris as u32), Some(0));
    assert_eq!(flat.owner_of(u32::MAX), None);
    // Closed: the leaf's baked verts sit at z ≈ 5; open: they reach z = 4.1.
    let shell_verts = c.shell.verts.len();
    let leaf_min_z = |f: &FlatMesh| {
        f.mesh
            .verts
            .iter()
            .skip(shell_verts)
            .map(|v| v[2])
            .fold(f64::INFINITY, f64::min)
    };
    assert!((leaf_min_z(&flat) - 4.97).abs() < 1e-9);
    c.set_door("door/leaf", DoorState::OPEN);
    let flat = c.flatten();
    assert!((leaf_min_z(&flat) - 4.1).abs() < 1e-9);
    // Owned section cut at eye height: shell walls own 0, the leaf owns 1.
    let cuts = section_at_owned(&flat.mesh, &flat.owner, 1.2, 0.35);
    assert!(cuts.iter().any(|(_, o)| *o == 0));
    let leaf_cuts: Vec<_> = cuts.iter().filter(|(_, o)| *o == 1).collect();
    assert!(!leaf_cuts.is_empty());
    for (seg, _) in &leaf_cuts {
        for p in seg {
            assert!(p[1] >= 4.1 - 1e-9 && p[1] <= 5.0 + 1e-9, "{seg:?}");
        }
    }
    // The flat mesh raycasts like the compound.
    let (obs, tgt) = RAY_B;
    assert!(
        flat.mesh
            .bvh
            .any_hit(&flat.mesh.verts, &flat.mesh.tris, obs, tgt, 0.0, 1.0)
            .is_some()
    );
    // A plain sidecar's cuts all belong to owner 0 (the wrapper).
    let plain = crate::building_section::section_at(&flat.mesh, 1.2, 0.35);
    assert_eq!(plain.len(), cuts.len());
}

#[test]
fn assemble_is_atomic_and_append_adds_scene_trees() {
    let mut blas = HashMap::new();
    blas.insert("blas/leaf.bvh".to_string(), leaf_blas());
    let missing = record(
        "x",
        InstanceKind::Prop,
        "blas/missing.bvh",
        &Rigid::identity(),
        None,
    );
    let ok = record(
        "d",
        InstanceKind::DoorLeaf,
        "blas/leaf.bvh",
        &Rigid::identity(),
        Some(door(90.0)),
    );
    let err = CompoundBuilding::assemble(room(), &[ok.clone(), missing.clone(), missing], &blas)
        .unwrap_err();
    assert_eq!(
        err,
        CompoundError::MissingBlas(vec!["blas/missing.bvh".into()])
    );
    assert_eq!(err.to_string(), "missing BLAS: blas/missing.bvh");
    let mut c = CompoundBuilding::assemble(room(), &[ok], &blas).unwrap();
    assert_eq!(c.instances.len(), 1);
    assert_eq!(c.doors().count(), 1);
    blas.insert(
        "blas/tree.bvh".to_string(),
        box_blas([-2.0, 0.0, -2.0], [2.0, 8.0, 2.0], SurfaceKind::Foliage),
    );
    c.append(
        &[record(
            "tree_nw",
            InstanceKind::Tree,
            "blas/tree.bvh",
            &Rigid::translation([-15.0, 0.0, -3.0]),
            None,
        )],
        &blas,
    )
    .unwrap();
    assert_eq!(c.instances.len(), 2);
    assert_eq!(c.instance_index("tree_nw"), Some(1));
    assert_eq!(c.doors().count(), 1);
}

#[test]
fn wash_sees_through_an_open_door_and_through_glass_but_not_a_frame() {
    let p = WashParams {
        cell_m: 0.5,
        eye_m: 1.2,
        radius_m: 12.0,
    };
    let mut c = room_with_door(90.0);
    let obs = [0.0, 1.2, 8.0];
    let behind_door = |w: &crate::building_viewshed::LevelWash| w.visibility_at(0.0, 3.0);
    let w = compound_wash(&c, obs, 1.2, 0, &p);
    assert_eq!(w.level_index, 0);
    assert_eq!(behind_door(&w), Visibility::Hidden);
    assert_eq!(w.visibility_at(0.0, 7.0), Visibility::Visible);
    c.set_door("door/leaf", DoorState::OPEN);
    let w = compound_wash(&c, obs, 1.2, 0, &p);
    assert_eq!(behind_door(&w), Visibility::Visible);
    // Beside the doorway the wall still hides the room.
    assert_eq!(w.visibility_at(-3.0, 3.0), Visibility::Hidden);

    let mut blas = HashMap::new();
    blas.insert("blas/pane.bvh".to_string(), pane_blas());
    blas.insert(
        "blas/frame.bvh".to_string(),
        box_blas([-0.5, -0.5, -0.05], [0.5, 0.5, 0.05], SurfaceKind::Opaque),
    );
    let at = Rigid::translation([1.5, 1.5, -5.0]);
    let obs = [1.5, 1.5, -8.0];
    let glazed = CompoundBuilding::assemble(
        room(),
        &[record(
            "win/pane",
            InstanceKind::Glass,
            "blas/pane.bvh",
            &at,
            None,
        )],
        &blas,
    )
    .unwrap();
    let w = compound_wash(&glazed, obs, 1.5, 0, &p);
    assert_eq!(w.visibility_at(1.5, -3.0), Visibility::Visible);
    let framed = CompoundBuilding::assemble(
        room(),
        &[record(
            "win/frame",
            InstanceKind::WindowFrame,
            "blas/frame.bvh",
            &at,
            None,
        )],
        &blas,
    )
    .unwrap();
    let w = compound_wash(&framed, obs, 1.5, 0, &p);
    assert_eq!(w.visibility_at(1.5, -3.0), Visibility::Hidden);
}

#[test]
fn segment_aabb_window_and_trace_ordering() {
    assert_eq!(
        segment_aabb_window(
            [0.0; 3],
            [10.0, 0.0, 0.0],
            [2.0, -1.0, -1.0],
            [4.0, 1.0, 1.0]
        ),
        Some((0.2, 0.4))
    );
    assert_eq!(
        segment_aabb_window(
            [0.0; 3],
            [10.0, 0.0, 0.0],
            [2.0, 1.0, -1.0],
            [4.0, 3.0, 1.0]
        ),
        None
    );
    assert_eq!(
        segment_aabb_window([3.0, 0.0, 0.0], [3.0, 0.0, 0.0], [2.0; 3], [4.0; 3]),
        None,
        "a point outside the box in y misses"
    );
    // Events come back sorted by t, shell before instances on ties.
    let c = room_with_door(90.0);
    let ev = c.trace([0.0, 1.2, 8.0], [0.0, 1.2, -8.0]);
    assert!(ev.windows(2).all(|w| w[0].t <= w[1].t));
    assert!(ev.iter().any(|e| e.owner == Owner::Instance(0)));
    assert!(ev.iter().any(|e| e.owner == Owner::Shell));
    assert!(c.trace([1.0; 3], [1.0; 3]).is_empty());
    assert!(!c.blocked([1.0; 3], [1.0; 3]));
}
