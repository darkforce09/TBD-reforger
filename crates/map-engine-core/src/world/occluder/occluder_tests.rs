//! Tests for the world occluder (T-090.12.3) — synthetic chunk worlds, no game content.

use std::collections::HashMap;
use std::sync::Arc;

use super::*;
use crate::building_blueprint::LosHitKind;
use crate::building_compound::{
    CoverTier, DoorRecord, InstanceKind, InstanceRecord, LocalTransform, PlacementSource,
};
use crate::bvh::{Bvh, BvhSidecar, SurfaceKind, segment_hits_tri};
use crate::geometry::rigid::Rigid;
use crate::world::chunk::WorldChunk;
use crate::world::chunk_math::TerrainSizeM;
use crate::world::prefab::PrefabRow;

// ───────────────────────────── helpers ─────────────────────────────

/// A deterministic LCG (no rand dependency).
struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 11) as f64) / ((1u64 << 53) as f64)
    }
    fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.next()
    }
}

fn cube_mesh(min: [f64; 3], max: [f64; 3]) -> (Vec<[f64; 3]>, Vec<[u32; 3]>) {
    let v = |x: usize, y: usize, z: usize| {
        [
            if x == 1 { max[0] } else { min[0] },
            if y == 1 { max[1] } else { min[1] },
            if z == 1 { max[2] } else { min[2] },
        ]
    };
    let verts = vec![
        v(0, 0, 0),
        v(1, 0, 0),
        v(1, 1, 0),
        v(0, 1, 0),
        v(0, 0, 1),
        v(1, 0, 1),
        v(1, 1, 1),
        v(0, 1, 1),
    ];
    let tris = vec![
        [0, 2, 1],
        [0, 3, 2],
        [4, 5, 6],
        [4, 6, 7],
        [0, 1, 5],
        [0, 5, 4],
        [3, 7, 6],
        [3, 6, 2],
        [1, 2, 6],
        [1, 6, 5],
        [0, 4, 7],
        [0, 7, 3],
    ];
    (verts, tris)
}

fn box_sidecar(min: [f64; 3], max: [f64; 3], kind: SurfaceKind) -> Arc<BvhSidecar> {
    let (verts, tris) = cube_mesh(min, max);
    let bvh = Bvh::build(&verts, &tris);
    Arc::new(BvhSidecar {
        kinds: vec![kind; tris.len()],
        verts,
        tris,
        bvh,
    })
}

fn record(
    id: &str,
    kind: InstanceKind,
    blas: &str,
    local: Rigid,
    parent: Option<&str>,
) -> InstanceRecord {
    InstanceRecord {
        id: id.into(),
        kind,
        prefab: "Prefabs/X.et".into(),
        blas: blas.into(),
        xob: None,
        local: LocalTransform::from_rigid(&local),
        door: None,
        cover: CoverTier::None,
        source: PlacementSource::PrefabCoords,
        parent: parent.map(ToString::to_string),
    }
}

fn descriptor(
    pid: u32,
    kind: &str,
    blocks: bool,
    instances: Vec<InstanceRecord>,
) -> PrefabDescriptor {
    let shell_bvh = instances
        .first()
        .map(|i| i.blas.clone())
        .unwrap_or_default();
    PrefabDescriptor {
        schema_version: DESCRIPTOR_SCHEMA_VERSION.into(),
        prefab_id: pid,
        slug: format!("P{pid}"),
        resource_name: format!("Prefabs/P{pid}.et"),
        kind: kind.into(),
        blocks,
        reason: (!blocks).then(|| "no-coll".to_string()),
        canopy: false,
        local_bounds: None,
        shell_bvh,
        instances,
        notes: vec![],
    }
}

/// `(pid, x, y_north, z_up, yaw, pitch, roll, scale)`.
type Row = (u16, f64, f64, f64, f64, f64, f64, f64);

/// Rows → a parsed chunk.
fn chunk(id: &str, rows: &[Row]) -> WorldChunk {
    let mut parts = id.split('_');
    let cx: f64 = parts.next().unwrap().parse().unwrap();
    let cy: f64 = parts.next().unwrap().parse().unwrap();
    let mut c = WorldChunk {
        id: id.to_string(),
        cx,
        cy,
        count: rows.len() as u32,
        ..Default::default()
    };
    for r in rows {
        c.positions.push(r.1 as f32);
        c.positions.push(r.2 as f32);
        c.prefab_idx.push(r.0);
        c.rotations.push(r.4 as f32);
        c.z.push(r.3 as f32);
        c.pitch.push(r.5 as f32);
        c.roll.push(r.6 as f32);
        c.scale.push(r.7 as f32);
        c.cls_codes.push(255);
    }
    c
}

fn prefab_row(pid: u16, kind: &str, hx: f64, hy: f64, hz: f64) -> PrefabRow {
    PrefabRow {
        prefab_id: f64::from(pid),
        kind: kind.into(),
        class: "x".into(),
        label: Some(format!("L{pid}")),
        resource_name: Some(format!("{{G}}Prefabs/P{pid}.et")),
        half_x: Some(hx),
        half_y: Some(hy),
        half_z: Some(hz),
        height_m: Some(2.0 * hz),
        icon_key: None,
        base_size_px: None,
        default_color: None,
        importance_zoom: None,
    }
}

/// 3×3 chunks of 512 m.
fn world() -> WorldOccluder {
    WorldOccluder::new(
        512.0,
        TerrainSizeM {
            width: 1536.0,
            height: 1536.0,
        },
    )
}

/// A unit-box prefab (±1 m, 0..2 m up) as a one-record descriptor + its BLAS.
fn install_box(w: &mut WorldOccluder, pid: u16, kind: InstanceKind) {
    let blas = format!("blas/box{pid}.bvh");
    w.insert_blas(
        &blas,
        box_sidecar([-1.0, 0.0, -1.0], [1.0, 2.0, 1.0], SurfaceKind::Opaque),
    );
    w.insert_descriptor(descriptor(
        u32::from(pid),
        "prop",
        true,
        vec![record(
            &format!("P{pid}"),
            kind,
            &blas,
            Rigid::identity(),
            None,
        )],
    ));
    w.refresh();
}

// ───────────────────────────── TLAS ─────────────────────────────

#[test]
fn tlas_matches_brute_force_including_the_observer_inside_case() {
    let mut rng = Lcg(7);
    let mut boxes = Vec::new();
    for _ in 0..2000 {
        let c = [
            rng.range(0.0, 512.0),
            rng.range(0.0, 60.0),
            rng.range(0.0, 512.0),
        ];
        let h = [
            rng.range(0.2, 6.0),
            rng.range(0.5, 15.0),
            rng.range(0.2, 6.0),
        ];
        boxes.push((
            [c[0] - h[0], c[1] - h[1], c[2] - h[2]],
            [c[0] + h[0], c[1] + h[1], c[2] + h[2]],
        ));
    }
    // Absent boxes stay at their index and never match.
    boxes.push(placed::NO_BOX);
    let tlas = AabbTlas::build(&boxes);
    assert_eq!(tlas.len(), 2001);
    let (mut fast, mut slow) = (Vec::new(), Vec::new());
    let mut inside_cases = 0;
    for i in 0..300 {
        let (a, b) = if i % 3 == 0 {
            // Start inside a random box (the canopy-over-the-observer case).
            let (lo, hi) = boxes[(rng.next() * 2000.0) as usize];
            let a = [
                rng.range(lo[0], hi[0]),
                rng.range(lo[1], hi[1]),
                rng.range(lo[2], hi[2]),
            ];
            inside_cases += 1;
            (
                a,
                [
                    rng.range(0.0, 512.0),
                    rng.range(0.0, 60.0),
                    rng.range(0.0, 512.0),
                ],
            )
        } else {
            (
                [
                    rng.range(-50.0, 560.0),
                    rng.range(-5.0, 70.0),
                    rng.range(-50.0, 560.0),
                ],
                [
                    rng.range(-50.0, 560.0),
                    rng.range(-5.0, 70.0),
                    rng.range(-50.0, 560.0),
                ],
            )
        };
        fast.clear();
        slow.clear();
        tlas.candidates(a, b, &mut fast);
        tlas.candidates_linear(a, b, &mut slow);
        assert_eq!(fast, slow, "segment {i}: {a:?} → {b:?}");
        if i % 3 == 0 {
            assert!(
                fast.iter().any(|c| c.t_entry == 0.0),
                "segment {i} starts inside a box"
            );
        }
    }
    assert_eq!(inside_cases, 100);
    assert!(AabbTlas::build(&[]).is_empty());
    assert!(AabbTlas::build(&[placed::NO_BOX]).is_empty());
}

// ───────────────────────────── DDA ─────────────────────────────

#[test]
fn dda_matches_the_brute_force_rasteriser() {
    let mut rng = Lcg(11);
    for i in 0..400 {
        let a = [rng.range(-100.0, 1636.0), rng.range(-100.0, 1636.0)];
        let b = [rng.range(-100.0, 1636.0), rng.range(-100.0, 1636.0)];
        let fast = cells_on_segment(a, b, 512.0, 3, 3);
        let slow = dda::cells_on_segment_reference(a, b, 512.0, 3, 3);
        assert_eq!(fast, slow, "segment {i}: {a:?} → {b:?}");
    }
    // Axis-aligned, zero-length, off-grid, and an exact diagonal through the corner.
    assert_eq!(
        cells_on_segment([10.0, 700.0], [1500.0, 700.0], 512.0, 3, 3),
        vec![(0, 1), (1, 1), (2, 1)]
    );
    assert_eq!(
        cells_on_segment([700.0, 1500.0], [700.0, 10.0], 512.0, 3, 3),
        vec![(1, 2), (1, 1), (1, 0)]
    );
    assert_eq!(
        cells_on_segment([700.0, 700.0], [700.0, 700.0], 512.0, 3, 3),
        vec![(1, 1)]
    );
    assert_eq!(
        cells_on_segment([-600.0, 100.0], [-10.0, 100.0], 512.0, 3, 3),
        Vec::<(i64, i64)>::new()
    );
    let diag = cells_on_segment([100.0, 100.0], [1400.0, 1400.0], 512.0, 3, 3);
    assert_eq!(diag.first(), Some(&(0, 0)));
    assert_eq!(diag.last(), Some(&(2, 2)));
    assert!(diag.contains(&(1, 1)));
    assert!(diag.len() <= 5, "{diag:?}");
}

// ───────────────────────────── placement / frames ─────────────────────────────

#[test]
fn a_tree_at_map_700_900_is_hit_from_690_900_and_missed_beside_it() {
    let mut w = world();
    w.set_prefabs([prefab_row(1, "tree", 1.0, 1.0, 5.0)].iter());
    w.insert_chunk(
        "1_1",
        &chunk("1_1", &[(1, 700.0, 900.0, 50.0, 0.0, 0.0, 0.0, 1.0)]),
    );
    // Map (690, 900) at 1 m above the row's 50 m elevation → engine [690, 51, 900].
    let obs = map_to_engine(690.0, 900.0, 51.0);
    let tgt = map_to_engine(720.0, 900.0, 51.0);
    assert_eq!(obs, [690.0, 51.0, 900.0]);
    // Proxy box first: blocked, but only provisionally.
    assert!(w.blocked(obs, tgt, BlockPolicy::VISION));
    let r = w.evaluate_los(obs, tgt);
    assert_eq!(r.verdict, WorldVerdict::Provisional);
    assert_eq!(r.coverage.proxy_pids, vec![1]);
    assert_eq!(
        r.blocker.as_ref().map(|b| b.fidelity),
        Some(Fidelity::Proxy)
    );
    assert_eq!(w.proxy_rows("1_1"), Some(1));
    // The real BLAS lands: exact, blocked, named.
    install_box(&mut w, 1, InstanceKind::Tree);
    assert_eq!(w.proxy_rows("1_1"), Some(0));
    let r = w.evaluate_los(obs, tgt);
    assert_eq!(r.verdict, WorldVerdict::Blocked);
    let b = r.blocker.expect("blocker");
    assert_eq!(
        (b.pid, b.chunk.as_str(), b.row, b.fidelity),
        (1, "1_1", 0, Fidelity::Exact)
    );
    assert!(
        (b.pos[0] - 699.0).abs() < 1e-9,
        "enters the box at x = 699: {:?}",
        b.pos
    );
    assert_eq!(r.hits.len(), 1);
    assert_eq!(r.hits[0].id, "1:1_1:0/P1");
    assert_eq!(r.concealment, 1.0);
    // Beside it (5 m north) and along the north axis through it.
    let r = w.evaluate_los(
        map_to_engine(690.0, 905.0, 51.0),
        map_to_engine(720.0, 905.0, 51.0),
    );
    assert_eq!(r.verdict, WorldVerdict::Clear);
    assert!(r.blocker.is_none() && r.hits.is_empty());
    assert!(w.blocked(
        map_to_engine(700.0, 890.0, 51.0),
        map_to_engine(700.0, 910.0, 51.0),
        BlockPolicy::VISION
    ));
    assert!(
        !w.blocked(
            map_to_engine(700.0, 890.0, 53.0),
            map_to_engine(700.0, 910.0, 53.0),
            BlockPolicy::VISION
        ),
        "over the 2 m box"
    );
    assert_eq!(w.label_of(1), Some("L1"));
    assert_eq!(w.kind_of(1), Some("tree"));
}

#[test]
fn scale_lengthens_the_crossing() {
    let mut w = world();
    w.set_prefabs([prefab_row(1, "prop", 1.0, 1.0, 1.0)].iter());
    install_box(&mut w, 1, InstanceKind::Prop);
    let depth = |w: &mut WorldOccluder, scale: f64| {
        w.insert_chunk(
            "1_1",
            &chunk("1_1", &[(1, 700.0, 900.0, 50.0, 0.0, 0.0, 0.0, scale)]),
        );
        w.refresh();
        let (ev, _) = w.trace(
            map_to_engine(680.0, 900.0, 50.5),
            map_to_engine(720.0, 900.0, 50.5),
        );
        let ts: Vec<f64> = ev.iter().map(|e| e.t).collect();
        (ts.last().unwrap() - ts.first().unwrap()) * 40.0
    };
    let d1 = depth(&mut w, 1.0);
    let d15 = depth(&mut w, 1.5);
    assert!((d1 - 2.0).abs() < 1e-6, "{d1}");
    assert!((d15 - 3.0).abs() < 1e-6, "{d15}");
}

#[test]
fn yaw_pitch_roll_placement_matches_a_brute_force_transform() {
    let mut w = world();
    w.set_prefabs([prefab_row(1, "prop", 1.0, 1.0, 1.0)].iter());
    install_box(&mut w, 1, InstanceKind::Prop);
    let row = (1u16, 700.0, 900.0, 50.0, 33.0, 10.0, -20.0, 1.25);
    w.insert_chunk("1_1", &chunk("1_1", &[row]));
    w.refresh();
    let rigid = Rigid::from_enfusion([700.0, 50.0, 900.0], [10.0, 33.0, -20.0], 1.25);
    let (verts, tris) = cube_mesh([-1.0, 0.0, -1.0], [1.0, 2.0, 1.0]);
    let world_verts: Vec<[f64; 3]> = verts.iter().map(|v| rigid.point(*v)).collect();
    let mut rng = Lcg(3);
    let mut compared = 0;
    for _ in 0..200 {
        let a = [
            rng.range(694.0, 706.0),
            rng.range(48.0, 55.0),
            rng.range(894.0, 906.0),
        ];
        let b = [
            rng.range(694.0, 706.0),
            rng.range(48.0, 55.0),
            rng.range(894.0, 906.0),
        ];
        let (ev, _) = w.trace(a, b);
        let mut mine: Vec<f64> = ev.iter().map(|e| e.t).collect();
        let mut brute: Vec<f64> = tris
            .iter()
            .filter_map(|t| {
                segment_hits_tri(
                    a,
                    b,
                    world_verts[t[0] as usize],
                    world_verts[t[1] as usize],
                    world_verts[t[2] as usize],
                )
            })
            .collect();
        brute.retain(|t| (0.0..=1.0).contains(t));
        brute.sort_by(f64::total_cmp);
        brute.dedup_by(|x, y| (*x - *y).abs() < 1e-9);
        mine.sort_by(f64::total_cmp);
        assert_eq!(
            mine.len(),
            brute.len(),
            "{a:?} → {b:?}: {mine:?} vs {brute:?}"
        );
        for (m, br) in mine.iter().zip(&brute) {
            assert!((m - br).abs() < 1e-7, "{m} vs {br}");
        }
        compared += brute.len();
    }
    assert!(compared > 40, "{compared} crossings compared");
}

// ───────────────────────────── coverage / honesty ─────────────────────────────

#[test]
fn a_missing_chunk_on_the_segment_is_provisional_and_named() {
    let mut w = world();
    w.set_prefabs([prefab_row(1, "prop", 1.0, 1.0, 1.0)].iter());
    w.insert_chunk("0_0", &chunk("0_0", &[]));
    w.insert_chunk("2_0", &chunk("2_0", &[]));
    let r = w.evaluate_los(
        map_to_engine(100.0, 300.0, 1.0),
        map_to_engine(1400.0, 300.0, 1.0),
    );
    assert_eq!(r.verdict, WorldVerdict::Provisional);
    assert!(r.blocker.is_none());
    assert_eq!(r.coverage.chunks_missing, vec!["1_0".to_string()]);
    assert_eq!(r.coverage.chunks_crossed, 3);
    // Blocked judges what is loaded.
    assert!(!w.blocked(
        map_to_engine(100.0, 300.0, 1.0),
        map_to_engine(1400.0, 300.0, 1.0),
        BlockPolicy::VISION
    ));
    w.insert_chunk("1_0", &chunk("1_0", &[]));
    let r = w.evaluate_los(
        map_to_engine(100.0, 300.0, 1.0),
        map_to_engine(1400.0, 300.0, 1.0),
    );
    assert_eq!(r.verdict, WorldVerdict::Clear);
}

#[test]
fn a_blocks_false_descriptor_and_an_unknown_pid_never_block() {
    let mut w = world();
    w.set_prefabs([prefab_row(1, "prop", 1.0, 1.0, 1.0)].iter());
    w.insert_chunk(
        "1_1",
        &chunk(
            "1_1",
            &[
                (1, 700.0, 900.0, 50.0, 0.0, 0.0, 0.0, 1.0),
                (9, 710.0, 900.0, 50.0, 0.0, 0.0, 0.0, 1.0),
            ],
        ),
    );
    let obs = map_to_engine(690.0, 900.0, 51.0);
    let tgt = map_to_engine(720.0, 900.0, 51.0);
    assert!(
        w.blocked(obs, tgt, BlockPolicy::VISION),
        "the proxy blocks until the descriptor says otherwise"
    );
    w.insert_descriptor(descriptor(1, "prop", false, vec![]));
    w.refresh();
    assert!(!w.blocked(obs, tgt, BlockPolicy::VISION));
    let r = w.evaluate_los(obs, tgt);
    assert_eq!(r.verdict, WorldVerdict::Clear);
    assert!(r.coverage.proxy_pids.is_empty(), "{:?}", r.coverage);
}

#[test]
fn glass_and_foliage_follow_the_compound_semantics_and_the_policy() {
    let mut w = world();
    w.set_prefabs(
        [
            prefab_row(1, "tree", 2.0, 2.0, 4.0),
            prefab_row(2, "prop", 1.0, 1.0, 1.0),
        ]
        .iter(),
    );
    // pid 1: a 4 m canopy box (foliage) over a thin trunk (opaque). pid 2: a glass pane.
    w.insert_blas(
        "blas/canopy.bvh",
        box_sidecar([-2.0, 3.0, -2.0], [2.0, 8.0, 2.0], SurfaceKind::Foliage),
    );
    w.insert_blas(
        "blas/trunk.bvh",
        box_sidecar([-0.2, 0.0, -0.2], [0.2, 3.0, 0.2], SurfaceKind::Opaque),
    );
    w.insert_blas(
        "blas/pane.bvh",
        box_sidecar([-0.002, 0.0, -1.0], [0.002, 8.0, 1.0], SurfaceKind::Glass),
    );
    w.insert_descriptor(descriptor(
        1,
        "tree",
        true,
        vec![
            record(
                "P1",
                InstanceKind::Tree,
                "blas/trunk.bvh",
                Rigid::identity(),
                None,
            ),
            record(
                "P1/canopy",
                InstanceKind::TreeCanopy,
                "blas/canopy.bvh",
                Rigid::identity(),
                Some("P1"),
            ),
        ],
    ));
    w.insert_descriptor(descriptor(
        2,
        "prop",
        true,
        vec![record(
            "P2",
            InstanceKind::Glass,
            "blas/pane.bvh",
            Rigid::identity(),
            None,
        )],
    ));
    w.insert_chunk(
        "1_1",
        &chunk(
            "1_1",
            &[
                (1, 700.0, 900.0, 50.0, 0.0, 0.0, 0.0, 1.0),
                (2, 720.0, 900.0, 50.0, 0.0, 0.0, 0.0, 1.0),
            ],
        ),
    );
    w.refresh();
    // Through the canopy (5 m above ground, 4 m of foliage) and the pane: clear, concealed.
    let obs = map_to_engine(690.0, 900.0, 55.0);
    let tgt = map_to_engine(730.0, 900.0, 55.0);
    let r = w.evaluate_los(obs, tgt);
    assert_eq!(r.verdict, WorldVerdict::Clear, "{r:?}");
    let kinds: Vec<LosHitKind> = r.hits.iter().map(|h| h.kind.clone()).collect();
    assert_eq!(kinds, vec![LosHitKind::Foliage, LosHitKind::Glass]);
    let expected = 1.0 - (1.0 - (1.0 - (-0.5f64 * 4.0).exp())) * (1.0 - 0.05);
    assert!(
        (r.concealment - expected).abs() < 1e-9,
        "{} vs {expected}",
        r.concealment
    );
    assert!(!w.blocked(obs, tgt, BlockPolicy::VISION));
    assert!(w.blocked(
        obs,
        tgt,
        BlockPolicy {
            glass_blocks: true,
            ..BlockPolicy::VISION
        }
    ));
    assert!(w.blocked(
        obs,
        tgt,
        BlockPolicy {
            foliage_blocks: true,
            ..BlockPolicy::VISION
        }
    ));
    // Through the trunk (1 m up): blocked by the tree.
    let r = w.evaluate_los(
        map_to_engine(690.0, 900.0, 51.0),
        map_to_engine(730.0, 900.0, 51.0),
    );
    assert_eq!(r.verdict, WorldVerdict::Blocked);
    assert_eq!(r.hits[0].kind, LosHitKind::Prop);
    assert_eq!(r.hits[0].id, "1:1_1:0/P1");
    // Observer standing under the canopy: its first crossing is an exit, depth counts from 0.
    let r = w.evaluate_los(
        map_to_engine(700.0, 900.0, 55.0),
        map_to_engine(730.0, 900.0, 55.0),
    );
    assert_eq!(r.verdict, WorldVerdict::Clear);
    let fol = r
        .hits
        .iter()
        .find(|h| h.kind == LosHitKind::Foliage)
        .expect("foliage hit");
    assert!(
        (fol.t - 0.0).abs() < 1e-12
            && (fol.concealment - (1.0 - (-0.5f64 * 2.0).exp())).abs() < 1e-9,
        "{fol:?}"
    );
}

#[test]
fn a_closed_leaf_blocks_and_an_initially_open_leaf_leaves_the_aperture_clear() {
    // A wall with a 1 m gap (two shell boxes), a 1 m leaf hinged at the gap's left edge.
    let mut w = world();
    w.set_prefabs(
        [
            prefab_row(1, "building", 4.0, 1.0, 1.5),
            prefab_row(2, "building", 4.0, 1.0, 1.5),
        ]
        .iter(),
    );
    w.insert_blas(
        "blas/wall_l.bvh",
        box_sidecar([-4.0, 0.0, -0.1], [-0.5, 3.0, 0.1], SurfaceKind::Opaque),
    );
    w.insert_blas(
        "blas/wall_r.bvh",
        box_sidecar([0.5, 0.0, -0.1], [4.0, 3.0, 0.1], SurfaceKind::Opaque),
    );
    w.insert_blas(
        "blas/leaf.bvh",
        box_sidecar([0.0, 0.0, -0.05], [1.0, 2.0, 0.05], SurfaceKind::Opaque),
    );
    let door = |initial: f64| DoorRecord {
        angle_range_deg: 90.0,
        closed_angle_deg: 0.0,
        initial_angle_deg: initial,
        angle_range_explicit: true,
        opened_distance: None,
    };
    let mut leaf = record(
        "B/leaf",
        InstanceKind::DoorLeaf,
        "blas/leaf.bvh",
        Rigid::translation([-0.5, 0.0, 0.0]),
        Some("B"),
    );
    let building = |pid: u32, initial: f64, leaf: &InstanceRecord| {
        let mut l = leaf.clone();
        l.door = Some(door(initial));
        descriptor(
            pid,
            "building",
            true,
            vec![
                record(
                    "B",
                    InstanceKind::Shell,
                    "blas/wall_l.bvh",
                    Rigid::identity(),
                    None,
                ),
                record(
                    "B/right",
                    InstanceKind::Shell,
                    "blas/wall_r.bvh",
                    Rigid::identity(),
                    Some("B"),
                ),
                l,
            ],
        )
    };
    leaf.door = Some(door(0.0));
    w.insert_descriptor(building(1, 0.0, &leaf));
    w.insert_descriptor(building(2, 90.0, &leaf));
    w.insert_chunk(
        "1_1",
        &chunk(
            "1_1",
            &[
                (1, 700.0, 900.0, 50.0, 0.0, 0.0, 0.0, 1.0),
                (2, 720.0, 900.0, 50.0, 0.0, 0.0, 0.0, 1.0),
            ],
        ),
    );
    w.refresh();
    // Through the gap of the closed-door building (x = 700, along north): blocked by the leaf.
    let r = w.evaluate_los(
        map_to_engine(700.0, 895.0, 51.0),
        map_to_engine(700.0, 905.0, 51.0),
    );
    assert_eq!(r.verdict, WorldVerdict::Blocked, "{r:?}");
    assert_eq!(r.hits[0].kind, LosHitKind::DoorLeaf);
    assert_eq!(r.hits[0].id, "1:1_1:0/B/leaf");
    // Through the gap of the open-door building: clear (the leaf swung out of the gap).
    let r = w.evaluate_los(
        map_to_engine(720.0, 895.0, 51.0),
        map_to_engine(720.0, 905.0, 51.0),
    );
    assert_eq!(r.verdict, WorldVerdict::Clear, "{r:?}");
    // The wall itself blocks either way.
    assert!(w.blocked(
        map_to_engine(722.0, 895.0, 51.0),
        map_to_engine(722.0, 905.0, 51.0),
        BlockPolicy::VISION
    ));
}

#[test]
fn blocked_agrees_with_the_verdict_on_random_segments() {
    let mut w = world();
    w.set_prefabs(
        [
            prefab_row(1, "prop", 1.0, 1.0, 1.0),
            prefab_row(2, "tree", 2.0, 2.0, 4.0),
        ]
        .iter(),
    );
    install_box(&mut w, 1, InstanceKind::Prop);
    w.insert_blas(
        "blas/canopy.bvh",
        box_sidecar([-2.0, 3.0, -2.0], [2.0, 8.0, 2.0], SurfaceKind::Foliage),
    );
    w.insert_descriptor(descriptor(
        2,
        "tree",
        true,
        vec![record(
            "P2",
            InstanceKind::TreeCanopy,
            "blas/canopy.bvh",
            Rigid::identity(),
            None,
        )],
    ));
    let mut rng = Lcg(5);
    for id in ["0_0", "0_1", "1_0", "1_1"] {
        let mut rows = Vec::new();
        let (cx, cy) = (
            id.chars().next().unwrap().to_digit(10).unwrap() as f64,
            id.chars().last().unwrap().to_digit(10).unwrap() as f64,
        );
        for _ in 0..80 {
            let pid = if rng.next() < 0.7 { 1 } else { 2 };
            rows.push((
                pid,
                cx * 512.0 + rng.range(0.0, 512.0),
                cy * 512.0 + rng.range(0.0, 512.0),
                50.0,
                rng.range(0.0, 360.0),
                rng.range(-5.0, 5.0),
                rng.range(-5.0, 5.0),
                rng.range(0.8, 1.3),
            ));
        }
        w.insert_chunk(id, &chunk(id, &rows));
    }
    w.refresh();
    let (mut blocked_n, mut clear_n) = (0, 0);
    for _ in 0..300 {
        let a = map_to_engine(
            rng.range(0.0, 1024.0),
            rng.range(0.0, 1024.0),
            rng.range(50.2, 56.0),
        );
        let b = map_to_engine(
            rng.range(0.0, 1024.0),
            rng.range(0.0, 1024.0),
            rng.range(50.2, 56.0),
        );
        let r = w.evaluate_los(a, b);
        let blocked = w.blocked(a, b, BlockPolicy::VISION);
        assert_eq!(blocked, r.blocker.is_some(), "{a:?} → {b:?}: {r:?}");
        assert!(r.coverage.chunks_missing.is_empty());
        assert_eq!(r.verdict != WorldVerdict::Clear, blocked);
        if blocked {
            blocked_n += 1;
        } else {
            clear_n += 1;
        }
    }
    assert!(
        blocked_n >= 10 && clear_n >= 10,
        "{blocked_n} blocked / {clear_n} clear"
    );
}

#[test]
fn the_blocked_closure_drives_a_wash_over_two_chunks() {
    use crate::building_viewshed::{WashParams, wash_band};
    use crate::dem::sample::Visibility;
    let mut w = world();
    w.set_prefabs([prefab_row(1, "prop", 1.0, 1.0, 1.0)].iter());
    w.insert_blas(
        "blas/wall.bvh",
        box_sidecar([-0.1, 0.0, -30.0], [0.1, 4.0, 30.0], SurfaceKind::Opaque),
    );
    w.insert_descriptor(descriptor(
        1,
        "prop",
        true,
        vec![record(
            "W",
            InstanceKind::Prop,
            "blas/wall.bvh",
            Rigid::identity(),
            None,
        )],
    ));
    // A 60 m wall along north at x = 512 (the chunk boundary), placed in chunk 1_0.
    w.insert_chunk("0_0", &chunk("0_0", &[]));
    w.insert_chunk(
        "1_0",
        &chunk("1_0", &[(1, 512.0, 256.0, 50.0, 0.0, 0.0, 0.0, 1.0)]),
    );
    w.refresh();
    let p = WashParams {
        radius_m: 40.0,
        cell_m: 4.0,
        ..WashParams::default()
    };
    let wash = wash_band(
        0,
        51.0,
        map_to_engine(490.0, 256.0, 51.0),
        &p,
        w.blocked_fn(BlockPolicy::VISION),
    );
    // West of the wall (x < 512) visible, east hidden.
    let vis = |x: f64, z: f64| {
        let (col, row) = wash.cell_at(x, z).expect("inside the wash");
        wash.cells[row * wash.cols + col]
    };
    let west = vis(500.0, 256.0);
    let east = vis(520.0, 256.0);
    assert_eq!((west, east), (Visibility::Visible, Visibility::Hidden));
    let (v, h, _) = wash.class_counts();
    assert!(v > 0 && h > 0);
}

#[test]
fn the_blas_cap_evicts_only_sidecars_nothing_resident_needs() {
    let mut w = world();
    w.set_blas_cap_bytes(600);
    w.set_prefabs(
        [
            prefab_row(1, "prop", 1.0, 1.0, 1.0),
            prefab_row(2, "prop", 1.0, 1.0, 1.0),
        ]
        .iter(),
    );
    w.insert_chunk(
        "1_1",
        &chunk("1_1", &[(1, 700.0, 900.0, 50.0, 0.0, 0.0, 0.0, 1.0)]),
    );
    // pid 1 is placed, pid 2 is not: each sidecar is ~450 B, so the second insert busts the cap.
    install_box(&mut w, 1, InstanceKind::Prop);
    assert_eq!(w.expanded_count(), 1);
    install_box(&mut w, 2, InstanceKind::Prop);
    assert_eq!(w.blas_count(), 1, "the unplaced pid's sidecar was evicted");
    assert!(w.expanded_of(1).is_some() && w.expanded_of(2).is_none());
    assert!(w.memory_bytes() > 0);
    // Wanted lists the un-expanded pid's BLAS once the chunk places it.
    w.insert_chunk(
        "0_0",
        &chunk("0_0", &[(2, 100.0, 100.0, 50.0, 0.0, 0.0, 0.0, 1.0)]),
    );
    let want = w.wanted(&["0_0".to_string(), "1_1".to_string()], 10);
    assert_eq!(want.blas, vec!["blas/box2.bvh".to_string()]);
    assert!(want.descriptors.is_empty());
    // A never-described pid asks for its descriptor.
    w.insert_chunk(
        "2_2",
        &chunk("2_2", &[(7, 1100.0, 1100.0, 50.0, 0.0, 0.0, 0.0, 1.0)]),
    );
    let want = w.wanted(&["2_2".to_string()], 10);
    assert_eq!(want.descriptors, vec![7]);
}

#[test]
fn rows_of_chunk_maps_the_columns_into_the_engine_frame() {
    let c = chunk(
        "1_1",
        &[(3, 700.5, 900.25, 50.0, 38.46, -3.04, -4.75, 1.15)],
    );
    let rows = rows_of_chunk(&c);
    assert_eq!(rows.len(), 1);
    let r = rows[0];
    assert_eq!(r.pid, 3);
    assert_eq!(r.pos, [700.5, 50.0, 900.25]);
    assert_eq!(r.angles_deg, [-3.04, 38.46, -4.75]);
    assert_eq!(r.scale, 1.15);
    let rigid = r.rigid();
    assert_eq!(rigid.t, [f64::from(700.5_f32), 50.0, f64::from(900.25_f32)]);
    assert!((rigid.scale - 1.15).abs() < 1e-6);
}

#[test]
fn removing_a_chunk_drops_its_rows_and_placements() {
    let mut w = world();
    w.set_prefabs([prefab_row(1, "prop", 1.0, 1.0, 1.0)].iter());
    install_box(&mut w, 1, InstanceKind::Prop);
    w.insert_chunk(
        "1_1",
        &chunk("1_1", &[(1, 700.0, 900.0, 50.0, 0.0, 0.0, 0.0, 1.0)]),
    );
    let obs = map_to_engine(690.0, 900.0, 51.0);
    let tgt = map_to_engine(720.0, 900.0, 51.0);
    assert!(w.blocked(obs, tgt, BlockPolicy::VISION));
    w.remove_chunk("1_1");
    assert_eq!(w.chunk_count(), 0);
    let r = w.evaluate_los(obs, tgt);
    assert_eq!(r.verdict, WorldVerdict::Provisional);
    assert_eq!(r.coverage.chunks_missing, vec!["1_1".to_string()]);
    let _ = HashMap::<u8, u8>::new();
}
