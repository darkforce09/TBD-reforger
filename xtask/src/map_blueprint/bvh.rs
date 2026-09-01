//! `cargo xtask map bvh-parity` — STEP 1 of the 3D-occlusion pivot: the one-number proof.
//!
//! Builds a midpoint-split BVH over the COLL fire-collision trimesh of a `.xob` (all
//! records, union, AS-IS — no decimation, no material taxonomy, binary solid) and replays
//! the Workbench parity oracle through a both-sided segment any-hit raycast. Prints
//! agree / model-clear-engine-blocked / model-blocked-engine-clear and nothing else decides
//! anything: report-only instrument, exactly like `map parity-report`.
//!
//! The triangle test ignores winding (|det|): any-hit occlusion is orientation-agnostic,
//! which sidesteps the COLL winding-inversion trap entirely.
//!
//! Usage: `--mesh <file.xob> --pairs <parity.json> [--record <i>] [--t-eps <meters>]
//!         [--dump-misses <path.jsonl>]`

use std::fs;
use std::io::Write as _;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};

use super::xob;
use crate::map_parity_report::ParityFile;

/// Conservative pad on f64→f32 node-bound storage. AABBs only cull, so padding can only
/// cost extra node visits, never a false miss; at building scale (|coord| ≤ ~30 m) the
/// f32 cast error is ≤ ~4e-6 m, three orders under the pad.
const AABB_PAD: f64 = 1e-3;
const LEAF_MAX: usize = 8;
const MAX_DEPTH: usize = 32;
/// |det| below this = segment parallel to the triangle plane (or degenerate triangle) —
/// same threshold as the marcher's Möller–Trumbore in `mesh.rs`.
const DET_EPS: f64 = 1e-12;
/// Barycentric slack, `mesh.rs` precedent: closes shared-edge cracks between
/// 0.01-quantized COLL verts without measurably expanding any triangle.
const BARY_EPS: f64 = 1e-9;

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// Both-sided Möller–Trumbore for segment p→q against triangle (a, b, c). Winding is
/// ignored. Returns the raw segment parameter t — the CALLER applies the [t_lo, t_hi]
/// range check (traversal, tests, and diagnostics each own their range).
fn segment_hits_tri(
    p: [f64; 3],
    q: [f64; 3],
    a: [f64; 3],
    b: [f64; 3],
    c: [f64; 3],
) -> Option<f64> {
    let dir = sub(q, p);
    let e1 = sub(b, a);
    let e2 = sub(c, a);
    let pvec = cross(dir, e2);
    let det = dot(e1, pvec);
    if det.abs() < DET_EPS {
        return None;
    }
    let inv = 1.0 / det;
    let tvec = sub(p, a);
    let u = dot(tvec, pvec) * inv;
    if !(-BARY_EPS..=1.0 + BARY_EPS).contains(&u) {
        return None;
    }
    let qvec = cross(tvec, e1);
    let v = dot(dir, qvec) * inv;
    if v < -BARY_EPS || u + v > 1.0 + BARY_EPS {
        return None;
    }
    Some(dot(e2, qvec) * inv)
}

/// Flat BVH node, 32 bytes, Wald layout: an internal node's children are adjacent
/// (`left_first` and `left_first + 1`); a leaf covers
/// `tri_order[left_first .. left_first + count]`.
#[derive(Clone, Copy)]
struct BvhNode {
    min: [f32; 3],
    max: [f32; 3],
    left_first: u32,
    /// 0 = internal node, > 0 = leaf triangle count.
    count: u32,
}
const _: () = assert!(std::mem::size_of::<BvhNode>() == 32);

/// Owns no geometry: `build` and `any_hit` take the same `verts`/`tris` slices, and the
/// `Hit::tri` index is the ORIGINAL triangle index (valid into `tris` / `tri_submesh`).
pub struct Bvh {
    nodes: Vec<BvhNode>,
    tri_order: Vec<u32>,
}

pub struct Hit {
    pub t: f64,
    pub tri: u32,
}

struct TriInfo {
    lo: [f64; 3],
    hi: [f64; 3],
    centroid: [f64; 3],
}

struct Builder<'a> {
    nodes: Vec<BvhNode>,
    tri_order: Vec<u32>,
    info: &'a [TriInfo],
}

impl Bvh {
    /// Midpoint split on the longest centroid axis, median-split fallback when the
    /// midpoint partition degenerates (guarantees strict progress; `MAX_DEPTH` is
    /// belt-and-suspenders). Panics on an empty mesh — callers bail first.
    pub fn build(verts: &[[f64; 3]], tris: &[[u32; 3]]) -> Bvh {
        assert!(!tris.is_empty(), "Bvh::build on empty mesh");
        let info: Vec<TriInfo> = tris
            .iter()
            .map(|t| {
                let mut lo = [f64::MAX; 3];
                let mut hi = [f64::MIN; 3];
                for &i in t {
                    let v = verts[i as usize];
                    for a in 0..3 {
                        lo[a] = lo[a].min(v[a]);
                        hi[a] = hi[a].max(v[a]);
                    }
                }
                let centroid = [
                    0.5 * (lo[0] + hi[0]),
                    0.5 * (lo[1] + hi[1]),
                    0.5 * (lo[2] + hi[2]),
                ];
                TriInfo { lo, hi, centroid }
            })
            .collect();
        let mut b = Builder {
            nodes: Vec::with_capacity(2 * tris.len()),
            tri_order: (0..tris.len() as u32).collect(),
            info: &info,
        };
        b.nodes.push(BvhNode {
            min: [0.0; 3],
            max: [0.0; 3],
            left_first: 0,
            count: 0,
        });
        b.build_into(0, 0, tris.len(), 0);
        Bvh {
            nodes: b.nodes,
            tri_order: b.tri_order,
        }
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Stack-based any-hit over segment p→q: accepts the FIRST triangle whose raw t lands
    /// in [t_lo, t_hi] — not the nearest; sufficient for occlusion and miss diagnostics.
    /// Slab math runs f64 against the f32 bounds. When p sits exactly on a slab bound
    /// with dir 0 on that axis, `0.0 * inf` yields NaN; `f64::min`/`f64::max` return the
    /// non-NaN operand, so NaN degrades to "visit the node" — conservative, never a cull.
    pub fn any_hit(
        &self,
        verts: &[[f64; 3]],
        tris: &[[u32; 3]],
        p: [f64; 3],
        q: [f64; 3],
        t_lo: f64,
        t_hi: f64,
    ) -> Option<Hit> {
        let dir = sub(q, p);
        let inv = [1.0 / dir[0], 1.0 / dir[1], 1.0 / dir[2]];
        // Stack depth ≤ MAX_DEPTH + 2: each internal pop nets +1 entry along one path.
        let mut stack = [0u32; 64];
        stack[0] = 0;
        let mut sp = 1usize;
        while sp > 0 {
            sp -= 1;
            let node = &self.nodes[stack[sp] as usize];
            let mut lo = t_lo;
            let mut hi = t_hi;
            for a in 0..3 {
                let t1 = (f64::from(node.min[a]) - p[a]) * inv[a];
                let t2 = (f64::from(node.max[a]) - p[a]) * inv[a];
                lo = lo.max(t1.min(t2));
                hi = hi.min(t1.max(t2));
            }
            if lo > hi {
                continue;
            }
            if node.count > 0 {
                for k in node.left_first..node.left_first + node.count {
                    let tri = self.tri_order[k as usize];
                    let [ia, ib, ic] = tris[tri as usize];
                    if let Some(t) = segment_hits_tri(
                        p,
                        q,
                        verts[ia as usize],
                        verts[ib as usize],
                        verts[ic as usize],
                    ) {
                        if t >= t_lo && t <= t_hi {
                            return Some(Hit { t, tri });
                        }
                    }
                }
            } else {
                debug_assert!(sp + 2 <= stack.len());
                stack[sp] = node.left_first;
                stack[sp + 1] = node.left_first + 1;
                sp += 2;
            }
        }
        None
    }
}

impl Builder<'_> {
    fn build_into(&mut self, node: usize, start: usize, end: usize, depth: usize) {
        let mut lo = [f64::MAX; 3];
        let mut hi = [f64::MIN; 3];
        let mut c_lo = [f64::MAX; 3];
        let mut c_hi = [f64::MIN; 3];
        for &t in &self.tri_order[start..end] {
            let ti = &self.info[t as usize];
            for a in 0..3 {
                lo[a] = lo[a].min(ti.lo[a]);
                hi[a] = hi[a].max(ti.hi[a]);
                c_lo[a] = c_lo[a].min(ti.centroid[a]);
                c_hi[a] = c_hi[a].max(ti.centroid[a]);
            }
        }
        // Pad in f64 FIRST, then cast: the cast rounds to nearest, and the pad dwarfs it.
        let min = [
            (lo[0] - AABB_PAD) as f32,
            (lo[1] - AABB_PAD) as f32,
            (lo[2] - AABB_PAD) as f32,
        ];
        let max = [
            (hi[0] + AABB_PAD) as f32,
            (hi[1] + AABB_PAD) as f32,
            (hi[2] + AABB_PAD) as f32,
        ];
        let len = end - start;
        let extent = [c_hi[0] - c_lo[0], c_hi[1] - c_lo[1], c_hi[2] - c_lo[2]];
        let splittable = extent.iter().any(|&e| e > 0.0);
        if len <= LEAF_MAX || depth >= MAX_DEPTH || !splittable {
            self.nodes[node] = BvhNode {
                min,
                max,
                left_first: start as u32,
                count: len as u32,
            };
            return;
        }
        let axis = (0..3)
            .max_by(|&a, &b| extent[a].total_cmp(&extent[b]))
            .unwrap();
        let mid = 0.5 * (c_lo[axis] + c_hi[axis]);
        let mut i = start;
        let mut j = end;
        while i < j {
            if self.info[self.tri_order[i] as usize].centroid[axis] < mid {
                i += 1;
            } else {
                j -= 1;
                self.tri_order.swap(i, j);
            }
        }
        let mut split = i;
        if split == start || split == end {
            // Midpoint degenerated (all centroids one side): median split for progress.
            self.tri_order[start..end].sort_unstable_by(|&a, &b| {
                self.info[a as usize].centroid[axis]
                    .total_cmp(&self.info[b as usize].centroid[axis])
            });
            split = start + len / 2;
        }
        let l = self.nodes.len();
        let placeholder = BvhNode {
            min: [0.0; 3],
            max: [0.0; 3],
            left_first: 0,
            count: 0,
        };
        self.nodes.push(placeholder);
        self.nodes.push(placeholder);
        self.nodes[node] = BvhNode {
            min,
            max,
            left_first: l as u32,
            count: 0,
        };
        self.build_into(l, start, split, depth + 1);
        self.build_into(l + 1, split, end, depth + 1);
    }
}

pub fn run_bvh_parity(args: &[String]) -> Result<u8> {
    let mut mesh_path: Option<PathBuf> = None;
    let mut pairs_path: Option<PathBuf> = None;
    let mut record: Option<u16> = None;
    let mut t_eps = 0.0f64;
    let mut dump_misses: Option<PathBuf> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--mesh" if i + 1 < args.len() => {
                mesh_path = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            "--pairs" if i + 1 < args.len() => {
                pairs_path = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            "--record" if i + 1 < args.len() => {
                record = Some(args[i + 1].parse().context("--record expects a u16")?);
                i += 2;
            }
            "--t-eps" if i + 1 < args.len() => {
                t_eps = args[i + 1]
                    .parse()
                    .context("--t-eps expects meters (f64)")?;
                i += 2;
            }
            "--dump-misses" if i + 1 < args.len() => {
                dump_misses = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            other => {
                eprintln!(
                    "bvh-parity: unknown arg {other} (usage: --mesh <file.xob> --pairs <parity.json> \
                     [--record <i>] [--t-eps <meters>] [--dump-misses <path.jsonl>])"
                );
                return Ok(1);
            }
        }
    }
    let mesh_path = mesh_path.context("--mesh <file.xob> is required")?;
    let pairs_path = pairs_path.context("--pairs <parity.json> is required")?;

    let bytes = fs::read(&mesh_path).with_context(|| mesh_path.display().to_string())?;
    let mut parsed = xob::parse_coll(&bytes)?;
    if let Some(rsel) = record {
        let mut tris = Vec::new();
        let mut subs = Vec::new();
        for (i, tri) in parsed.tris.iter().enumerate() {
            if parsed.tri_submesh[i] == rsel {
                tris.push(*tri);
                subs.push(rsel);
            }
        }
        if tris.is_empty() {
            bail!("--record {rsel}: no triangles in that record");
        }
        parsed.tris = tris;
        parsed.tri_submesh = subs;
    }

    let mut per_record: std::collections::BTreeMap<u16, usize> = std::collections::BTreeMap::new();
    for &r in &parsed.tri_submesh {
        *per_record.entry(r).or_default() += 1;
    }
    let recs: Vec<String> = per_record
        .iter()
        .map(|(r, n)| format!("{r}: {n}"))
        .collect();
    let (lo, hi) = xob::aabb(&parsed.verts);
    let bvh = Bvh::build(&parsed.verts, &parsed.tris);
    println!(
        "coll: {} verts · {} tris · records [{}] · aabb [{:.2},{:.2},{:.2}]..[{:.2},{:.2},{:.2}] · {} bvh nodes",
        parsed.verts.len(),
        parsed.tris.len(),
        recs.join(", "),
        lo[0],
        lo[1],
        lo[2],
        hi[0],
        hi[1],
        hi[2],
        bvh.node_count(),
    );

    let parity: ParityFile = serde_json::from_str(
        &fs::read_to_string(&pairs_path)
            .with_context(|| format!("read {}", pairs_path.display()))?,
    )
    .context("parse parity JSON")?;

    let mut agree = 0usize;
    let mut model_clear_engine_blocked = 0usize;
    let mut model_blocked_engine_clear = 0usize;
    let mut misses: Vec<String> = Vec::new();
    for (idx, &(ox, oy, oz, tx, ty, tz, engine_clear)) in parity.pairs.iter().enumerate() {
        let p = [ox, oy, oz];
        let q = [tx, ty, tz];
        let seg_len = dot(sub(q, p), sub(q, p)).sqrt();
        // Endpoint policy: strict [0,1] by default; --t-eps shrinks both ends by a metric
        // margin (an oracle endpoint on a 0.01-quantized surface must not self-block).
        let (t_lo, t_hi) = if t_eps > 0.0 && seg_len >= 2.0 * t_eps {
            (t_eps / seg_len, 1.0 - t_eps / seg_len)
        } else {
            (0.0, 1.0)
        };
        let hit = if seg_len < 1e-9 {
            None // degenerate pair: zero-length segment occludes nothing
        } else {
            bvh.any_hit(&parsed.verts, &parsed.tris, p, q, t_lo, t_hi)
        };
        let model_clear = hit.is_none();
        if model_clear == engine_clear {
            agree += 1;
        } else {
            if model_clear {
                model_clear_engine_blocked += 1;
            } else {
                model_blocked_engine_clear += 1;
            }
            if dump_misses.is_some() {
                let row = match &hit {
                    Some(h) => serde_json::json!({
                        "pair": idx,
                        "engine_clear": engine_clear,
                        "model_clear": model_clear,
                        "t": h.t,
                        "tri": h.tri,
                        "record": parsed.tri_submesh[h.tri as usize],
                        "hit": [ox + h.t * (tx - ox), oy + h.t * (ty - oy), oz + h.t * (tz - oz)],
                    }),
                    None => serde_json::json!({
                        "pair": idx,
                        "engine_clear": engine_clear,
                        "model_clear": model_clear,
                        "t": null,
                        "tri": null,
                        "record": null,
                        "hit": null,
                    }),
                };
                misses.push(row.to_string());
            }
        }
    }

    if let Some(path) = &dump_misses {
        let mut f = fs::File::create(path).with_context(|| path.display().to_string())?;
        for m in &misses {
            writeln!(f, "{m}")?;
        }
        println!("wrote {} misses → {}", misses.len(), path.display());
    }

    let total = parity.pairs.len().max(1);
    println!(
        "bvh-parity {}: {agree}/{} agree ({:.1}%) · model-clear/engine-blocked {} · model-blocked/engine-clear {}",
        parity.slug,
        parity.pairs.len(),
        agree as f64 * 100.0 / total as f64,
        model_clear_engine_blocked,
        model_blocked_engine_clear,
    );
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    type Scene = (Vec<[f64; 3]>, Vec<[u32; 3]>);

    /// Axis-aligned cuboid as 12 outward-wound triangles (quad table from the COLL box
    /// emitter in `xob.rs`).
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
    fn bvh_matches_brute_force_on_box_grid() {
        // 3×3×3 grid of cubes (324 tris) — a real tree, not a single leaf.
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
        let (verts, tris) = concat(&scenes);
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
}
