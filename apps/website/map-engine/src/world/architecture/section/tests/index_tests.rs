//! Role: index tests.
//! Position: `world/architecture/section/tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::spatial::bvh::sidecar::BvhSidecar;
use crate::spatial::bvh::traversal::Bvh;
use crate::spatial::bvh::tree::tests::Scene;
use crate::world::architecture::blueprint::model::tests::room_sidecar;
use crate::world::architecture::blueprint::model::tests::slab;
use crate::world::architecture::section::cutter::CUT_MAX_NY;
use crate::world::architecture::section::cutter::HeightField;
use crate::world::architecture::section::cutter::MAX_PLAN_DIM;
use crate::world::architecture::section::cutter::PLAN_CELL_M;
use crate::world::architecture::section::cutter::Seg2;
use crate::world::architecture::section::cutter::VOID_PAD_M;
use crate::world::architecture::section::cutter::mesh_bounds;
use crate::world::architecture::section::cutter::section_at_owned;
use crate::world::architecture::section::index::*;

fn farmhouse() -> BvhSidecar {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../assets_v2/terrains/everon/prefabs/buildings/FarmHouse_E_1L01_Wood.bvh"
    );
    BvhSidecar::parse(&std::fs::read(path).expect("FarmHouse Wood sidecar"))
        .expect("parse FarmHouse Wood")
}

fn tower() -> BvhSidecar {
    let scenes: [Scene; 2] = [
        slab([0.0, 4.0], [0.0, 3.0], [0.0, 4.0]),
        slab([0.0, 4.0], [6.0, 9.0], [0.0, 4.0]),
    ];
    let (verts, tris) = crate::spatial::bvh::tree::tests::concat(&scenes);
    let bvh = Bvh::build(&verts, &tris);
    BvhSidecar::opaque(verts, tris, bvh)
}

fn pillar() -> BvhSidecar {
    let sc = slab([1.0, 1.4], [0.0, 2.5], [1.0, 1.4]);
    let (verts, tris) = crate::spatial::bvh::tree::tests::concat(&[sc]);
    let bvh = Bvh::build(&verts, &tris);
    BvhSidecar::opaque(verts, tris, bvh)
}

fn golden_buildings() -> Vec<(&'static str, BvhSidecar)> {
    vec![
        ("FarmHouse_E_1L01_Wood", farmhouse()),
        ("room", room_sidecar(&[])),
        (
            "room_stairwell",
            room_sidecar(&[
                slab([0.0, 6.0], [2.95, 3.05], [0.0, 1.0]),
                slab([0.0, 6.0], [2.95, 3.05], [2.0, 6.0]),
                slab([0.0, 1.0], [2.95, 3.05], [1.0, 2.0]),
                slab([2.0, 6.0], [2.95, 3.05], [1.0, 2.0]),
            ]),
        ),
        (
            "room_treads",
            room_sidecar(
                &(0..4)
                    .map(|k| {
                        let x0 = 4.0 + 0.25 * f64::from(k);
                        slab([x0, x0 + 0.25], [0.0, 0.2 + 0.2 * f64::from(k)], [1.0, 2.0])
                    })
                    .collect::<Vec<_>>(),
            ),
        ),
        (
            "room_slope",
            room_sidecar(&[(
                vec![[0.0, 0.0, 0.0], [6.0, 0.0, 0.0], [0.0, 6.0, 6.0]],
                vec![[0, 1, 2]],
            )]),
        ),
        ("tower", tower()),
    ]
}

fn brute_section_at_owned(
    occl: &BvhSidecar,
    owner: &[u32],
    y: f64,
    max_abs_ny: f64,
) -> Vec<(Seg2, u32)> {
    use crate::spatial::bvh::node::cross;
    use crate::spatial::bvh::node::sub;
    let mut out = Vec::new();
    for (ti, &[ia, ib, ic]) in occl.tris.iter().enumerate() {
        let v = [
            occl.verts[ia as usize],
            occl.verts[ib as usize],
            occl.verts[ic as usize],
        ];
        let d = [v[0][1] - y, v[1][1] - y, v[2][1] - y];
        if d.iter().all(|&e| e > 0.0) || d.iter().all(|&e| e < 0.0) || d.iter().all(|&e| e == 0.0) {
            continue;
        }
        let n = cross(sub(v[1], v[0]), sub(v[2], v[0]));
        let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
        if len < 1e-12 || (n[1] / len).abs() > max_abs_ny {
            continue;
        }
        let mut pts: Vec<[f64; 2]> = Vec::with_capacity(3);
        let mut push = |p: [f64; 2]| {
            if pts.iter().all(|q| (q[0] - p[0]).hypot(q[1] - p[1]) > 1e-9) {
                pts.push(p);
            }
        };
        for i in 0..3 {
            let j = (i + 1) % 3;
            if d[i] == 0.0 {
                push([v[i][0], v[i][2]]);
            }
            if d[i] * d[j] < 0.0 {
                let t = d[i] / (d[i] - d[j]);
                push([
                    v[i][0] + t * (v[j][0] - v[i][0]),
                    v[i][2] + t * (v[j][2] - v[i][2]),
                ]);
            }
        }
        if pts.len() >= 2 {
            out.push(([pts[0], pts[1]], owner.get(ti).copied().unwrap_or(0)));
        }
    }
    out
}

fn segs_equal(a: &[(Seg2, u32)], b: &[(Seg2, u32)]) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut aa = a.to_vec();
    let mut bb = b.to_vec();
    let key = |s: &(Seg2, u32)| {
        let (p, q) = (s.0[0], s.0[1]);
        let (lo, hi) = if (p[0], p[1]) <= (q[0], q[1]) {
            (p, q)
        } else {
            (q, p)
        };
        (
            s.1,
            lo[0].to_bits(),
            lo[1].to_bits(),
            hi[0].to_bits(),
            hi[1].to_bits(),
        )
    };
    aa.sort_by_key(key);
    bb.sort_by_key(key);
    aa == bb
}

#[test]
fn t938_4_measure_visits_and_bytes() {
    let occl = farmhouse();
    let (lo, hi) = mesh_bounds(&occl).expect("bounds");
    let min = [lo[0] - VOID_PAD_M, lo[1] - VOID_PAD_M];
    let max = [hi[0] + VOID_PAD_M, hi[1] + VOID_PAD_M];
    let empty = HeightField::empty(min, max, PLAN_CELL_M);
    let (_cands, visits) = triangles_overlapping_y_counted(&occl, 1.2, 1.2);
    let built = HeightField::build(&occl, min, max, PLAN_CELL_M, 1.2, 0.2);
    eprintln!(
        "T-938.4 FarmHouse_E_1L01_Wood: tris={} verts={} visits={} empty_bytes={} built_bytes={} cols={} rows={} cap={}",
        occl.tris.len(),
        occl.verts.len(),
        visits,
        empty.allocated_bytes(),
        built.allocated_bytes(),
        empty.cols,
        empty.rows,
        MAX_PLAN_DIM
    );
    assert!(
        visits < occl.tris.len(),
        "index must visit fewer than all {} triangles, got {visits}",
        occl.tris.len()
    );
    assert_eq!(
        empty.allocated_bytes(),
        0,
        "empty HeightField allocates no plan cells"
    );
    let dense = empty.cols * empty.rows * std::mem::size_of::<Option<f64>>();
    assert!(
        built.allocated_bytes() < dense,
        "sparse {} >= dense {}",
        built.allocated_bytes(),
        dense
    );
}

#[test]
fn bvh_root_encloses_section_geometry() {
    for (name, occl) in golden_buildings() {
        assert!(
            bvh_encloses_mesh(&occl),
            "{name}: occlusion BVH root bounds miss mesh verts"
        );
    }
    assert!(bvh_encloses_mesh(&pillar()));
}

#[test]
fn golden_section_cut_equals_brute_force() {
    for (name, occl) in golden_buildings() {
        assert!(bvh_encloses_mesh(&occl), "{name} enclosure");
        for y in [0.45_f64, 1.2, 3.5, 7.0] {
            let got = section_at_owned(&occl, &[], y, CUT_MAX_NY);
            let brute = brute_section_at_owned(&occl, &[], y, CUT_MAX_NY);
            assert!(
                segs_equal(&got, &brute),
                "{name} y={y}: indexed {} segs vs brute {}",
                got.len(),
                brute.len()
            );
        }
    }
}

#[test]
fn zero_height_inverted_interval_is_empty() {
    let occl = room_sidecar(&[]);
    let (cands, visits) = triangles_overlapping_y_counted(&occl, 1.2, 1.2 - 1e-12);
    assert!(cands.is_empty() && visits == 0);
}

#[test]
fn sparse_heightfield_one_percent_memory() {
    let span = MAX_PLAN_DIM as f64 * PLAN_CELL_M;
    let mut hf = HeightField::empty([0.0, 0.0], [span, span], PLAN_CELL_M);
    assert_eq!(hf.cols, MAX_PLAN_DIM);
    assert_eq!(hf.rows, MAX_PLAN_DIM);
    assert_eq!(hf.allocated_bytes(), 0);
    let dense = hf.cols * hf.rows * std::mem::size_of::<Option<f64>>();
    let n = (MAX_PLAN_DIM * MAX_PLAN_DIM) / 100;
    let side = (n as f64).sqrt().ceil() as usize;
    for row in 0..side {
        for col in 0..side {
            hf.set(col, row, Some(1.0));
        }
    }
    let sparse = hf.allocated_bytes();
    assert!(
        sparse > 0 && sparse * 50 < dense,
        "1% clustered write should be ~1% of dense: sparse={sparse} dense={dense}"
    );
}
