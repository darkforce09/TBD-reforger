//! Role: tree.
//! Position: `spatial/bvh/tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::spatial::bvh::node::BvhNode;

use crate::spatial::bvh::tree::*;

/// Scene.
pub(crate) type Scene = (Vec<[f64; 3]>, Vec<[u32; 3]>);

/// Axis-aligned cuboid as 12 outward-wound triangles (quad table from the COLL box emitter in xtask's `xob.rs`).
pub(crate) fn cube(center: [f64; 3], half: [f64; 3]) -> Scene {
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

/// Concat.
pub(crate) fn concat(scenes: &[Scene]) -> Scene {
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

fn brute_force_any(
    verts: &[[f64; 3]],
    tris: &[[u32; 3]],
    p: [f64; 3],
    q: [f64; 3],
    t_lo: f64,
    t_hi: f64,
) -> bool {
    tris.iter().any(|&[a, b, c]| {
        segment_hits_tri(
            p,
            q,
            verts[a as usize],
            verts[b as usize],
            verts[c as usize],
        )
        .is_some_and(|t| (t_lo..=t_hi).contains(&t))
    })
}

struct Rng(u64);
impl Rng {
    fn next_f64(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 11) as f64 / (1u64 << 53) as f64
    }
    fn coord(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.next_f64()
    }
}

#[test]
fn segment_tri_hits_both_sides() {
    let a = [0.0, 0.0, 0.0];
    let b = [2.0, 0.0, 0.0];
    let c = [0.0, 0.0, 2.0];
    let p = [0.5, -1.0, 0.5];
    let q = [0.5, 1.0, 0.5];
    let t1 = segment_hits_tri(p, q, a, b, c).expect("CCW winding hits");
    let t2 = segment_hits_tri(p, q, a, c, b).expect("CW winding hits");
    assert!((t1 - 0.5).abs() < 1e-12 && (t2 - 0.5).abs() < 1e-12);

    let t3 = segment_hits_tri(q, p, a, b, c).expect("reverse direction hits");
    assert!((t3 - 0.5).abs() < 1e-12);

    assert!(segment_hits_tri([3.0, -1.0, 3.0], [3.0, 1.0, 3.0], a, b, c).is_none());

    let t4 = segment_hits_tri([0.5, -1.0, 0.5], [0.5, -0.5, 0.5], a, b, c).expect("raw t");
    assert!((t4 - 2.0).abs() < 1e-12);
}

#[test]
fn segment_tri_parallel_and_degenerate_none() {
    let a = [0.0, 0.0, 0.0];
    let b = [2.0, 0.0, 0.0];
    let c = [0.0, 0.0, 2.0];

    assert!(segment_hits_tri([0.2, 1.0, 0.2], [1.0, 1.0, 1.0], a, b, c).is_none());

    let d = [1.0, 0.0, 0.0];
    let e = [2.0, 0.0, 0.0];
    let f = [3.0, 0.0, 0.0];
    assert!(segment_hits_tri([1.5, -1.0, 0.0], [1.5, 1.0, 0.0], d, e, f).is_none());
}

#[test]
fn cube_any_hit_center_misses_and_trange() {
    let (verts, tris) = cube([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
    let bvh = Bvh::build(&verts, &tris);

    assert!(
        bvh.any_hit(&verts, &tris, [-3.0, 0.1, 0.2], [3.0, 0.1, 0.2], 0.0, 1.0)
            .is_some()
    );

    assert!(
        bvh.any_hit(&verts, &tris, [-3.0, 5.0, 0.0], [3.0, 5.0, 0.0], 0.0, 1.0)
            .is_none()
    );

    assert!(
        bvh.any_hit(&verts, &tris, [-3.0, 0.0, 0.0], [1.0, 0.0, 0.0], 0.0, 0.4)
            .is_none()
    );

    assert!(
        bvh.any_hit(&verts, &tris, [0.0, 0.0, 0.0], [5.0, 0.0, 0.0], 0.0, 1.0)
            .is_some()
    );
}

#[test]
fn bvh_matches_brute_force_on_box_grid() {
    let (verts, tris) = box_grid();
    assert_eq!(tris.len(), 324);
    let bvh = Bvh::build(&verts, &tris);
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    for i in 0..200 {
        let p = [
            rng.coord(-3.0, 9.0),
            rng.coord(-3.0, 9.0),
            rng.coord(-3.0, 9.0),
        ];
        let q = [
            rng.coord(-3.0, 9.0),
            rng.coord(-3.0, 9.0),
            rng.coord(-3.0, 9.0),
        ];
        let fast = bvh.any_hit(&verts, &tris, p, q, 0.0, 1.0).is_some();
        let slow = brute_force_any(&verts, &tris, p, q, 0.0, 1.0);
        assert_eq!(fast, slow, "segment {i}: p {p:?} q {q:?}");
    }
}

#[test]
fn coincident_centroids_terminate_as_leaf() {
    let mut verts = Vec::new();
    let mut tris = Vec::new();
    for _ in 0..30 {
        let base = verts.len() as u32;
        verts.push([0.0, 0.0, 0.0]);
        verts.push([1.0, 0.0, 0.0]);
        verts.push([0.0, 0.0, 1.0]);
        tris.push([base, base + 1, base + 2]);
    }
    let bvh = Bvh::build(&verts, &tris);
    assert!(
        bvh.any_hit(&verts, &tris, [0.2, -1.0, 0.2], [0.2, 1.0, 0.2], 0.0, 1.0)
            .is_some()
    );
    assert!(
        bvh.any_hit(&verts, &tris, [5.0, -1.0, 5.0], [5.0, 1.0, 5.0], 0.0, 1.0)
            .is_none()
    );
}

#[test]
fn union_spans_all_records() {
    let (verts, tris) = concat(&[
        cube([0.0, 0.0, 0.0], [0.5, 0.5, 0.5]),
        cube([10.0, 0.0, 0.0], [0.5, 0.5, 0.5]),
    ]);
    let bvh = Bvh::build(&verts, &tris);
    let hit = bvh
        .any_hit(&verts, &tris, [10.0, -3.0, 0.0], [10.0, 3.0, 0.0], 0.0, 1.0)
        .expect("second cube occludes");
    assert!(
        hit.tri >= 12,
        "hit triangle {} should be in the second cube",
        hit.tri
    );
}

#[test]
fn endpoint_epsilon_excludes_surface_start() {
    let (verts, tris) = cube([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
    let bvh = Bvh::build(&verts, &tris);

    let p = [1.0, 0.0, 0.0];
    let q = [4.0, 0.0, 0.0];
    assert!(bvh.any_hit(&verts, &tris, p, q, 0.0, 1.0).is_some());

    let t_lo = 0.01 / 3.0;
    assert!(bvh.any_hit(&verts, &tris, p, q, t_lo, 1.0 - t_lo).is_none());

    let t_lo2 = 0.01 / 6.0;
    assert!(
        bvh.any_hit(
            &verts,
            &tris,
            [-3.0, 0.0, 0.0],
            [3.0, 0.0, 0.0],
            t_lo2,
            1.0 - t_lo2
        )
        .is_some()
    );
}

fn brute_force_first(
    verts: &[[f64; 3]],
    tris: &[[u32; 3]],
    p: [f64; 3],
    q: [f64; 3],
    t_lo: f64,
    t_hi: f64,
) -> Option<f64> {
    tris.iter()
        .filter_map(|&[a, b, c]| {
            segment_hits_tri(
                p,
                q,
                verts[a as usize],
                verts[b as usize],
                verts[c as usize],
            )
        })
        .filter(|t| (t_lo..=t_hi).contains(t))
        .min_by(f64::total_cmp)
}

#[test]
fn first_hit_returns_nearest_of_stacked_cubes() {
    let (verts, tris) = concat(&[
        cube([0.0, 0.0, 0.0], [0.5, 0.5, 0.5]),
        cube([10.0, 0.0, 0.0], [0.5, 0.5, 0.5]),
    ]);
    let bvh = Bvh::build(&verts, &tris);

    let p = [-3.0, 0.1, 0.2];
    let q = [13.0, 0.1, 0.2];
    let h = bvh
        .first_hit(&verts, &tris, p, q, 0.0, 1.0)
        .expect("near cube hit");
    assert!((h.t - 2.5 / 16.0).abs() < 1e-12, "t = {}", h.t);
    assert!(h.tri < 12, "tri {} should be in the near cube", h.tri);

    let h = bvh
        .first_hit(&verts, &tris, q, p, 0.0, 1.0)
        .expect("far cube hit");
    assert!((h.t - 2.5 / 16.0).abs() < 1e-12, "t = {}", h.t);
    assert!(h.tri >= 12, "tri {} should be in the far cube", h.tri);

    let any = bvh.any_hit(&verts, &tris, p, q, 0.0, 1.0).expect("any hit");
    let first = bvh
        .first_hit(&verts, &tris, p, q, 0.0, 1.0)
        .expect("first hit");
    assert!(first.t <= any.t);
}

#[test]
fn first_hit_matches_min_t_brute_force_on_box_grid() {
    let (verts, tris) = box_grid();
    let bvh = Bvh::build(&verts, &tris);
    let mut rng = Rng(0x5851_F42D_4C95_7F2D);
    for i in 0..200 {
        let p = [
            rng.coord(-3.0, 9.0),
            rng.coord(-3.0, 9.0),
            rng.coord(-3.0, 9.0),
        ];
        let q = [
            rng.coord(-3.0, 9.0),
            rng.coord(-3.0, 9.0),
            rng.coord(-3.0, 9.0),
        ];
        let fast = bvh.first_hit(&verts, &tris, p, q, 0.0, 1.0);
        let slow = brute_force_first(&verts, &tris, p, q, 0.0, 1.0);

        assert_eq!(
            fast.map(|h| h.t.to_bits()),
            slow.map(f64::to_bits),
            "segment {i}: p {p:?} q {q:?}"
        );

        if let Some(h) = fast {
            let [a, b, c] = tris[h.tri as usize];
            let t = segment_hits_tri(
                p,
                q,
                verts[a as usize],
                verts[b as usize],
                verts[c as usize],
            )
            .expect("returned tri is hit");
            assert_eq!(t.to_bits(), h.t.to_bits(), "segment {i}");
        }
    }
}

#[test]
fn first_hit_none_iff_any_hit_none() {
    let (verts, tris) = box_grid();
    let bvh = Bvh::build(&verts, &tris);
    let mut rng = Rng(0x0123_4567_89AB_CDEF);
    for i in 0..300 {
        let p = [
            rng.coord(-3.0, 9.0),
            rng.coord(-3.0, 9.0),
            rng.coord(-3.0, 9.0),
        ];
        let q = [
            rng.coord(-3.0, 9.0),
            rng.coord(-3.0, 9.0),
            rng.coord(-3.0, 9.0),
        ];
        let (a, b) = (rng.next_f64(), rng.next_f64());
        let (t_lo, t_hi) = (a.min(b), a.max(b));
        let first = bvh.first_hit(&verts, &tris, p, q, t_lo, t_hi);
        let any = bvh.any_hit(&verts, &tris, p, q, t_lo, t_hi);
        assert_eq!(
            first.is_none(),
            any.is_none(),
            "segment {i}: p {p:?} q {q:?} range [{t_lo}, {t_hi}]"
        );
        if let (Some(f), Some(a)) = (first, any) {
            assert!(f.t <= a.t, "segment {i}: first {} behind any {}", f.t, a.t);
        }
    }
}

fn box_grid() -> Scene {
    let mut scenes = Vec::new();
    for x in 0..3 {
        for y in 0..3 {
            for z in 0..3 {
                scenes.push(cube(
                    [x as f64 * 3.0, y as f64 * 3.0, z as f64 * 3.0],
                    [0.6, 0.6, 0.6],
                ));
            }
        }
    }
    concat(&scenes)
}

type Emitted = (Vec<u8>, Vec<[f64; 3]>, Vec<[u32; 3]>, Bvh);

fn emit_scene(scene: &Scene) -> Emitted {
    emit_scene_kinds(scene, &vec![SurfaceKind::Opaque; scene.1.len()])
}

fn emit_scene_kinds(scene: &Scene, kinds: &[SurfaceKind]) -> Emitted {
    let (verts, tris) = scene;
    let verts_f32 = quantize_verts(verts);
    let lifted = lift_verts(&verts_f32);
    let bvh = Bvh::build(&lifted, tris);
    let bytes = emit_bytes(&verts_f32, tris, kinds, &bvh);
    (bytes, lifted, tris.clone(), bvh)
}

fn kinds_len(ntris: usize) -> usize {
    ntris.div_ceil(4) * 4
}

fn hit_key(h: Option<Hit>) -> Option<(u64, u32)> {
    h.map(|h| (h.t.to_bits(), h.tri))
}

#[test]
fn sidecar_round_trip_box_grid() {
    let scene = box_grid();
    let (bytes, lifted, tris, fresh) = emit_scene(&scene);
    let sc = BvhSidecar::parse(&bytes).expect("emitted bytes parse");
    assert_eq!(sc.verts, lifted, "lifted verts round-trip bit-exact");
    assert_eq!(sc.tris, tris);
    assert_eq!(sc.bvh.node_count(), fresh.node_count());

    let mut rng = Rng(0xDEAD_BEEF_CAFE_F00D);
    for _ in 0..50 {
        let p = [
            rng.coord(-3.0, 9.0),
            rng.coord(-3.0, 9.0),
            rng.coord(-3.0, 9.0),
        ];
        let q = [
            rng.coord(-3.0, 9.0),
            rng.coord(-3.0, 9.0),
            rng.coord(-3.0, 9.0),
        ];
        assert_eq!(
            hit_key(sc.bvh.any_hit(&sc.verts, &sc.tris, p, q, 0.0, 1.0)),
            hit_key(fresh.any_hit(&lifted, &tris, p, q, 0.0, 1.0)),
        );
    }
    assert_eq!(sc.kinds, vec![SurfaceKind::Opaque; tris.len()]);

    let verts_f32 = quantize_verts(&sc.verts);
    assert_eq!(emit_bytes(&verts_f32, &sc.tris, &sc.kinds, &sc.bvh), bytes);
}

#[test]
fn v1_sidecar_parses_as_all_opaque_and_upgrades() {
    let scene = box_grid();
    let (v2, _, tris, _) = emit_scene(&scene);
    let mut v1 = v2[..v2.len() - kinds_len(tris.len())].to_vec();
    v1[4..8].copy_from_slice(&1u32.to_le_bytes());
    v1[20..24].copy_from_slice(&0u32.to_le_bytes());
    let sc = BvhSidecar::parse(&v1).expect("v1 bytes parse");
    assert_eq!(sc.kinds, vec![SurfaceKind::Opaque; tris.len()]);
    assert_eq!(sc.kind_counts(), (tris.len(), 0, 0));
    let verts_f32 = quantize_verts(&sc.verts);
    assert_eq!(emit_bytes(&verts_f32, &sc.tris, &sc.kinds, &sc.bvh), v2);

    let mut flagged = v1.clone();
    flagged[20] = 1;
    assert_eq!(
        BvhSidecar::parse(&flagged).unwrap_err(),
        BvhParseError::NonZeroReserved
    );
}

#[test]
fn kinds_round_trip_and_rejections() {
    use crate::spatial::bvh::tree::tests::BvhParseError as E;

    let (verts, tris) = cube([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
    let scene = (verts, tris[..9].to_vec());
    let kinds: Vec<SurfaceKind> = (0..9u8)
        .map(|i| SurfaceKind::from_u8(i % 3).unwrap())
        .collect();
    let (bytes, ..) = emit_scene_kinds(&scene, &kinds);
    let o = offs(&bytes);
    assert_eq!(bytes.len(), o.kinds + 12);
    assert!(bytes[o.kinds + 9..].iter().all(|&b| b == 0));
    let sc = BvhSidecar::parse(&bytes).expect("mixed kinds parse");
    assert_eq!(sc.kinds, kinds);
    assert_eq!(sc.kind_counts(), (3, 3, 3));
    assert_eq!(sc.kind(4), SurfaceKind::Glass);
    assert_eq!(
        BvhSidecar::parse(&patch(&bytes, o.kinds + 2, &[3])).unwrap_err(),
        E::UnknownKind { tri: 2, code: 3 }
    );
    assert_eq!(
        BvhSidecar::parse(&patch(&bytes, o.kinds + 10, &[1])).unwrap_err(),
        E::KindsPadding
    );
    assert_eq!(SurfaceKind::from_u8(SurfaceKind::MAX_CODE + 1), None);
    assert!(SurfaceKind::Opaque.is_terminal());
    assert!(!SurfaceKind::Glass.is_terminal() && !SurfaceKind::Foliage.is_terminal());
}

#[test]
fn filtered_traversals_skip_non_terminal_kinds() {
    let scene = concat(&[
        cube([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]),
        cube([0.0, 4.0, 0.0], [1.0, 1.0, 1.0]),
    ]);
    let (verts, tris) = &scene;
    let bvh = Bvh::build(verts, tris);
    let mut kinds = vec![SurfaceKind::Glass; 12];
    kinds.extend(std::iter::repeat_n(SurfaceKind::Opaque, 12));
    let p = [0.3, -3.0, -0.2];
    let q = [0.3, 8.0, -0.2];

    let t_at = |y: f64| (y + 3.0) / 11.0;
    let solid = SurfaceKind::is_terminal;
    let first_all = bvh
        .first_hit(verts, tris, p, q, 0.0, 1.0)
        .expect("glass face");
    assert!((first_all.t - t_at(-1.0)).abs() < 1e-12);
    let first_solid = bvh
        .first_hit_where(verts, tris, &kinds, p, q, 0.0, 1.0, solid)
        .expect("opaque face");
    assert!((first_solid.t - t_at(3.0)).abs() < 1e-12);
    assert!(first_solid.tri >= 12);
    let any_solid = bvh
        .any_hit_where(verts, tris, &kinds, p, q, 0.0, 1.0, solid)
        .expect("some opaque face");
    assert!(any_solid.tri >= 12 && any_solid.t >= t_at(3.0) - 1e-12);
    let any_glass = bvh
        .any_hit_where(verts, tris, &kinds, p, q, 0.0, 1.0, |k| {
            k == SurfaceKind::Glass
        })
        .expect("some glass face");
    assert!(any_glass.tri < 12 && any_glass.t <= t_at(1.0) + 1e-12);

    let q_short = [0.3, 2.0, -0.2];
    assert!(
        bvh.any_hit_where(verts, tris, &kinds, p, q_short, 0.0, 1.0, solid)
            .is_none()
    );
    assert!(bvh.any_hit(verts, tris, p, q_short, 0.0, 1.0).is_some());

    let opaque = vec![SurfaceKind::Opaque; tris.len()];
    let mut rng = Rng(0x5EED_5EED_5EED_5EED);
    for _ in 0..40 {
        let a = [
            rng.coord(-3.0, 3.0),
            rng.coord(-3.0, 7.0),
            rng.coord(-3.0, 3.0),
        ];
        let b = [
            rng.coord(-3.0, 3.0),
            rng.coord(-3.0, 7.0),
            rng.coord(-3.0, 3.0),
        ];
        assert_eq!(
            hit_key(bvh.first_hit_where(verts, tris, &opaque, a, b, 0.0, 1.0, solid)),
            hit_key(bvh.first_hit(verts, tris, a, b, 0.0, 1.0)),
        );
        assert_eq!(
            hit_key(bvh.any_hit_where(verts, tris, &opaque, a, b, 0.0, 1.0, solid)),
            hit_key(bvh.any_hit(verts, tris, a, b, 0.0, 1.0)),
        );
    }
}

#[test]
fn all_hits_lists_every_crossing_sorted() {
    let scene = concat(&[
        cube([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]),
        cube([0.0, 4.0, 0.0], [1.0, 1.0, 1.0]),
        cube([0.0, 8.0, 0.0], [1.0, 1.0, 1.0]),
    ]);
    let (verts, tris) = &scene;
    let bvh = Bvh::build(verts, tris);

    let p = [0.3, -3.0, -0.2];
    let q = [0.3, 12.0, -0.2];
    let mut out = vec![Hit { t: -1.0, tri: 99 }];
    bvh.all_hits(verts, tris, p, q, 0.0, 1.0, &mut out);
    assert_eq!(out[0], Hit { t: -1.0, tri: 99 }, "prefix untouched");
    let hits = &out[1..];
    assert_eq!(hits.len(), 6, "entry + exit per cube: {hits:?}");
    assert!(hits.windows(2).all(|w| w[0].t < w[1].t), "ascending t");
    let mut brute: Vec<Hit> = tris
        .iter()
        .enumerate()
        .filter_map(|(i, &[a, b, c])| {
            segment_hits_tri(
                p,
                q,
                verts[a as usize],
                verts[b as usize],
                verts[c as usize],
            )
            .filter(|t| (0.0..=1.0).contains(t))
            .map(|t| Hit { t, tri: i as u32 })
        })
        .collect();
    brute.sort_by(|a, b| a.t.total_cmp(&b.t).then(a.tri.cmp(&b.tri)));
    assert_eq!(hits, brute.as_slice());

    let mut short = Vec::new();
    bvh.all_hits(verts, tris, p, [0.3, 4.0, -0.2], 0.0, 1.0, &mut short);
    assert_eq!(short.len(), 3);
}

#[test]
fn double_emit_is_byte_identical() {
    let scene = box_grid();
    let (a, ..) = emit_scene(&scene);
    let (b, ..) = emit_scene(&scene);
    assert_eq!(a, b, "two independent builds must serialize identically");
}

#[test]
fn emitted_size_matches_formula() {
    let scene = box_grid();
    let (bytes, _, tris, bvh) = emit_scene(&scene);
    let expected = 32
        + scene.0.len() * 12
        + tris.len() * 12
        + bvh.node_count() * 32
        + tris.len() * 4
        + kinds_len(tris.len());
    assert_eq!(bytes.len(), expected);
    assert_eq!(u32::from_le_bytes(bytes[4..8].try_into().unwrap()), 2);
    assert_eq!(
        u32::from_le_bytes(bytes[20..24].try_into().unwrap()),
        FLAG_KINDS
    );
}

#[test]
fn root_leaf_single_node_round_trips() {
    let (verts, tris) = cube([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
    let scene = (verts, tris[..8].to_vec());
    let (bytes, lifted, tris8, fresh) = emit_scene(&scene);
    assert_eq!(fresh.node_count(), 1);
    let sc = BvhSidecar::parse(&bytes).expect("single-node sidecar parses");
    assert_eq!(sc.bvh.node_count(), 1);
    assert_eq!(
        hit_key(sc.bvh.any_hit(
            &sc.verts,
            &sc.tris,
            [0.0, 0.0, 0.0],
            [0.0, 5.0, 0.0],
            0.0,
            1.0
        )),
        hit_key(fresh.any_hit(&lifted, &tris8, [0.0, 0.0, 0.0], [0.0, 5.0, 0.0], 0.0, 1.0)),
    );
}

struct Offs {
    verts: usize,
    tris: usize,
    nodes: usize,
    order: usize,
    kinds: usize,
    nverts: u32,
    ntris: u32,
}

fn offs(bytes: &[u8]) -> Offs {
    let nverts = u32::from_le_bytes(bytes[8..12].try_into().unwrap());
    let ntris = u32::from_le_bytes(bytes[12..16].try_into().unwrap());
    let nnodes = u32::from_le_bytes(bytes[16..20].try_into().unwrap());
    let verts = 32;
    let tris = verts + nverts as usize * 12;
    let nodes = tris + ntris as usize * 12;
    let order = nodes + nnodes as usize * 32;
    let kinds = order + ntris as usize * 4;
    Offs {
        verts,
        tris,
        nodes,
        order,
        kinds,
        nverts,
        ntris,
    }
}

fn patch(bytes: &[u8], at: usize, with: &[u8]) -> Vec<u8> {
    let mut b = bytes.to_vec();
    b[at..at + with.len()].copy_from_slice(with);
    b
}

#[test]
fn parse_rejects_malformed_bytes() {
    use crate::spatial::bvh::tree::tests::BvhParseError as E;

    let (base, ..) = emit_scene(&cube([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]));
    let o = offs(&base);

    assert_eq!(
        BvhSidecar::parse(&base[..16]).unwrap_err(),
        E::TooShort { len: 16 }
    );
    assert_eq!(
        BvhSidecar::parse(&patch(&base, 0, b"XBVH")).unwrap_err(),
        E::BadMagic(*b"XBVH")
    );
    assert_eq!(
        BvhSidecar::parse(&patch(&base, 4, &3u32.to_le_bytes())).unwrap_err(),
        E::UnsupportedVersion(3)
    );
    assert_eq!(
        BvhSidecar::parse(&patch(&base, 4, &0u32.to_le_bytes())).unwrap_err(),
        E::UnsupportedVersion(0)
    );

    assert_eq!(
        BvhSidecar::parse(&patch(&base, 21, &[1])).unwrap_err(),
        E::NonZeroReserved
    );
    assert_eq!(
        BvhSidecar::parse(&patch(&base, 24, &[1])).unwrap_err(),
        E::NonZeroReserved
    );
    assert_eq!(
        BvhSidecar::parse(&patch(&base, 31, &[1])).unwrap_err(),
        E::NonZeroReserved
    );

    assert_eq!(
        BvhSidecar::parse(&patch(&base, 12, &0u32.to_le_bytes())).unwrap_err(),
        E::EmptyMesh
    );
    let mut longer = base.clone();
    longer.push(0);
    assert_eq!(
        BvhSidecar::parse(&longer).unwrap_err(),
        E::LengthMismatch {
            expected: base.len() as u64,
            actual: base.len() as u64 + 1
        }
    );
    assert_eq!(
        BvhSidecar::parse(&patch(&base, o.verts, &f32::NAN.to_le_bytes())).unwrap_err(),
        E::NonFiniteVert { vert: 0 }
    );
    assert_eq!(
        BvhSidecar::parse(&patch(&base, o.tris, &o.nverts.to_le_bytes())).unwrap_err(),
        E::TriIndexOutOfBounds { tri: 0 }
    );
    assert_eq!(
        BvhSidecar::parse(&patch(&base, o.nodes, &f32::NAN.to_le_bytes())).unwrap_err(),
        E::NonFiniteNodeBound { node: 0 }
    );

    assert_eq!(
        BvhSidecar::parse(&patch(
            &base,
            o.nodes + 32 + 28,
            &(o.ntris + 1).to_le_bytes()
        ))
        .unwrap_err(),
        E::LeafRangeOutOfBounds { node: 1 }
    );

    assert_eq!(
        BvhSidecar::parse(&patch(&base, o.nodes + 24, &0u32.to_le_bytes())).unwrap_err(),
        E::ChildOutOfBounds { node: 0 }
    );
    assert_eq!(
        BvhSidecar::parse(&patch(&base, o.order, &o.ntris.to_le_bytes())).unwrap_err(),
        E::TriOrderOutOfBounds { slot: 0 }
    );
    let slot1 = &base[o.order + 4..o.order + 8];
    assert_eq!(
        BvhSidecar::parse(&patch(&base, o.order, slot1)).unwrap_err(),
        E::TriOrderNotPermutation {
            tri: u32::from_le_bytes(slot1.try_into().unwrap())
        }
    );
}

fn craft(nodes: Vec<BvhNode>, ntris: u32) -> Vec<u8> {
    let verts_f32 = vec![[0.0f32; 3], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]];
    let tris = vec![[0u32, 1, 2]; ntris as usize];
    let tri_order: Vec<u32> = (0..ntris).collect();
    let kinds = vec![SurfaceKind::Opaque; ntris as usize];
    emit_bytes(&verts_f32, &tris, &kinds, &Bvh { nodes, tri_order })
}

fn leaf(first: u32, count: u32) -> BvhNode {
    BvhNode {
        min: [0.0; 3],
        max: [1.0; 3],
        left_first: first,
        count,
    }
}

fn internal(left_first: u32) -> BvhNode {
    BvhNode {
        min: [0.0; 3],
        max: [1.0; 3],
        left_first,
        count: 0,
    }
}

#[test]
fn parse_rejects_structural_attacks() {
    use crate::spatial::bvh::tree::tests::BvhParseError as E;

    let diamond = craft(vec![internal(1), internal(2), leaf(0, 1), leaf(1, 1)], 2);
    assert_eq!(
        BvhSidecar::parse(&diamond).unwrap_err(),
        E::NodeRevisited { node: 2 }
    );

    let orphan = craft(vec![leaf(0, 2), leaf(0, 2)], 2);
    assert_eq!(
        BvhSidecar::parse(&orphan).unwrap_err(),
        E::OrphanNodes {
            visited: 1,
            nnodes: 2
        }
    );

    let overlap = craft(vec![internal(1), leaf(0, 2), leaf(0, 1)], 3);
    assert!(matches!(
        BvhSidecar::parse(&overlap).unwrap_err(),
        E::LeafCoverageMismatch { .. }
    ));

    let levels = 61u32;
    let nnodes = 2 * levels + 3;
    let mut nodes = Vec::new();
    let mut slot = 0u32;
    nodes.push(internal(1));
    for k in 0..levels {
        let idx = 2 * k + 1;

        nodes.push(internal(idx + 2));
        nodes.push(leaf(slot, 1));
        slot += 1;
    }
    nodes.push(leaf(slot, 1));
    nodes.push(leaf(slot + 1, 1));
    let ntris = slot + 2;
    assert_eq!(nodes.len() as u32, nnodes);
    let deep = craft(nodes, ntris);
    assert_eq!(BvhSidecar::parse(&deep).unwrap_err(), E::TreeTooDeep);
}
