//! Tests for [`super`] (the BVH raycaster + `.bvh` sidecar codec), split out per the
//! `building_blueprint_tests.rs` `#[path]` precedent to stay under the SIZE gate.

use super::*;

type Scene = (Vec<[f64; 3]>, Vec<[u32; 3]>);

/// Axis-aligned cuboid as 12 outward-wound triangles (quad table from the COLL box
/// emitter in xtask's `xob.rs`).
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

/// Deterministic xorshift64* — no rand dep.
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
    // Reversed segment sees the same plane at the mirrored parameter.
    let t3 = segment_hits_tri(q, p, a, b, c).expect("reverse direction hits");
    assert!((t3 - 0.5).abs() < 1e-12);
    // Outside the barycentric bounds: crossing the plane misses the triangle.
    assert!(segment_hits_tri([3.0, -1.0, 3.0], [3.0, 1.0, 3.0], a, b, c).is_none());
    // Raw t beyond the segment is still returned — the caller owns the range.
    let t4 = segment_hits_tri([0.5, -1.0, 0.5], [0.5, -0.5, 0.5], a, b, c).expect("raw t");
    assert!((t4 - 2.0).abs() < 1e-12);
}

#[test]
fn segment_tri_parallel_and_degenerate_none() {
    let a = [0.0, 0.0, 0.0];
    let b = [2.0, 0.0, 0.0];
    let c = [0.0, 0.0, 2.0];
    // Parallel to the plane (constant y): no hit even directly above the face.
    assert!(segment_hits_tri([0.2, 1.0, 0.2], [1.0, 1.0, 1.0], a, b, c).is_none());
    // Colinear (zero-area) triangle: rejected by the det gate, no panic.
    let d = [1.0, 0.0, 0.0];
    let e = [2.0, 0.0, 0.0];
    let f = [3.0, 0.0, 0.0];
    assert!(segment_hits_tri([1.5, -1.0, 0.0], [1.5, 1.0, 0.0], d, e, f).is_none());
}

#[test]
fn cube_any_hit_center_misses_and_trange() {
    let (verts, tris) = cube([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
    let bvh = Bvh::build(&verts, &tris);
    // Straight through the middle.
    assert!(
        bvh.any_hit(&verts, &tris, [-3.0, 0.1, 0.2], [3.0, 0.1, 0.2], 0.0, 1.0)
            .is_some()
    );
    // Fully outside.
    assert!(
        bvh.any_hit(&verts, &tris, [-3.0, 5.0, 0.0], [3.0, 5.0, 0.0], 0.0, 1.0)
            .is_none()
    );
    // Segment toward the cube but t_hi excludes the face (face at x=-1 → t≈0.5 on a
    // segment from -3 to 1; cap at 0.4).
    assert!(
        bvh.any_hit(&verts, &tris, [-3.0, 0.0, 0.0], [1.0, 0.0, 0.0], 0.0, 0.4)
            .is_none()
    );
    // Inside → outside crosses the shell.
    assert!(
        bvh.any_hit(&verts, &tris, [0.0, 0.0, 0.0], [5.0, 0.0, 0.0], 0.0, 1.0)
            .is_some()
    );
}

#[test]
fn bvh_matches_brute_force_on_box_grid() {
    // 3×3×3 grid of cubes (324 tris) — a real tree, not a single leaf.
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
    // 30 coincident triangles: centroid extent is zero on every axis — the build must
    // terminate (forced leaf) and still answer queries.
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
    // Two disjoint cubes standing in for two COLL records: a segment that only
    // crosses the second must still hit (guards against first-record-only indexing).
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
    // Start exactly on the +x face, pointing away: strict [0,1] self-blocks at t=0…
    let p = [1.0, 0.0, 0.0];
    let q = [4.0, 0.0, 0.0];
    assert!(bvh.any_hit(&verts, &tris, p, q, 0.0, 1.0).is_some());
    // …a metric endpoint epsilon (0.01 m over a 3 m segment) frees it.
    let t_lo = 0.01 / 3.0;
    assert!(bvh.any_hit(&verts, &tris, p, q, t_lo, 1.0 - t_lo).is_none());
    // The same epsilon still sees the cube from the other side of the room.
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

/* ───────────────────────────── sidecar codec tests ───────────────────────────── */

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

/// (bytes, lifted verts, tris, freshly built Bvh) from the emitter pipeline under test.
type Emitted = (Vec<u8>, Vec<[f64; 3]>, Vec<[u32; 3]>, Bvh);

/// The emitter pipeline under test: quantize → lift → build → emit.
fn emit_scene(scene: &Scene) -> Emitted {
    let (verts, tris) = scene;
    let verts_f32 = quantize_verts(verts);
    let lifted = lift_verts(&verts_f32);
    let bvh = Bvh::build(&lifted, tris);
    let bytes = emit_bytes(&verts_f32, tris, &bvh);
    (bytes, lifted, tris.clone(), bvh)
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
    // Raycasts over the parsed sidecar are bit-identical to the emitter's build.
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
    // Re-emitting the parsed sidecar reproduces the file byte-for-byte.
    let verts_f32 = quantize_verts(&sc.verts);
    assert_eq!(emit_bytes(&verts_f32, &sc.tris, &sc.bvh), bytes);
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
    let expected =
        32 + scene.0.len() * 12 + tris.len() * 12 + bvh.node_count() * 32 + tris.len() * 4;
    assert_eq!(bytes.len(), expected);
}

#[test]
fn root_leaf_single_node_round_trips() {
    // ≤ LEAF_MAX triangles → a one-node tree (root is a leaf); the format and the
    // validator must both accept it.
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

/// Section offsets of an emitted sidecar, derived from its own header.
struct Offs {
    verts: usize,
    tris: usize,
    nodes: usize,
    order: usize,
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
    Offs {
        verts,
        tris,
        nodes,
        order,
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
    use BvhParseError as E;
    // Base: one cube — root internal + two leaves, enough structure for every case.
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
        BvhSidecar::parse(&patch(&base, 4, &2u32.to_le_bytes())).unwrap_err(),
        E::UnsupportedVersion(2)
    );
    assert_eq!(
        BvhSidecar::parse(&patch(&base, 20, &[1])).unwrap_err(),
        E::NonZeroReserved
    );
    // Counts are judged before the length equation, so a zeroed count reads as an
    // empty mesh, not a length mismatch.
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
    // Node 1 is a leaf (root splits 12 tris into two leaves): blow its count.
    assert_eq!(
        BvhSidecar::parse(&patch(
            &base,
            o.nodes + 32 + 28,
            &(o.ntris + 1).to_le_bytes()
        ))
        .unwrap_err(),
        E::LeafRangeOutOfBounds { node: 1 }
    );
    // Root is internal: left_first = 0 breaks the forward-only rule.
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

/// Hand-assemble a sidecar from raw node tables (the emitter cannot produce these
/// shapes; a hostile file can).
fn craft(nodes: Vec<BvhNode>, ntris: u32) -> Vec<u8> {
    let verts_f32 = vec![[0.0f32; 3], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]];
    let tris = vec![[0u32, 1, 2]; ntris as usize];
    let tri_order: Vec<u32> = (0..ntris).collect();
    emit_bytes(&verts_f32, &tris, &Bvh { nodes, tri_order })
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
    use BvhParseError as E;
    // Diamond: nodes 0 and 1 both parent the pair (2, 3) → node 2 reachable twice.
    let diamond = craft(vec![internal(1), internal(2), leaf(0, 1), leaf(1, 1)], 2);
    assert_eq!(
        BvhSidecar::parse(&diamond).unwrap_err(),
        E::NodeRevisited { node: 2 }
    );
    // Orphan: root leaf covers everything; node 1 is unreachable.
    let orphan = craft(vec![leaf(0, 2), leaf(0, 2)], 2);
    assert_eq!(
        BvhSidecar::parse(&orphan).unwrap_err(),
        E::OrphanNodes {
            visited: 1,
            nnodes: 2
        }
    );
    // Overlapping leaves: both children cover slot 0 (per-node ranges legal).
    let overlap = craft(vec![internal(1), leaf(0, 2), leaf(0, 1)], 3);
    assert!(matches!(
        BvhSidecar::parse(&overlap).unwrap_err(),
        E::LeafCoverageMismatch { .. }
    ));
    // Forward-only chain deeper than the parse bound: internals 0, 1, 3, 5, …
    // (left child = next internal, right child = leaf), 61 internal levels.
    let levels = 61u32;
    let nnodes = 2 * levels + 3; // internals + interleaved leaves + final pair
    let mut nodes = Vec::new();
    let mut slot = 0u32;
    nodes.push(internal(1));
    for k in 0..levels {
        let idx = 2 * k + 1;
        // Left child chains to the next internal (the last one parents the final pair).
        nodes.push(internal(idx + 2));
        nodes.push(leaf(slot, 1)); // right child of the previous internal
        slot += 1;
    }
    nodes.push(leaf(slot, 1));
    nodes.push(leaf(slot + 1, 1));
    let ntris = slot + 2;
    assert_eq!(nodes.len() as u32, nnodes);
    let deep = craft(nodes, ntris);
    assert_eq!(BvhSidecar::parse(&deep).unwrap_err(), E::TreeTooDeep);
}
