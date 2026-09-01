//! BVH any-hit raycaster over collision trimeshes + the binary `.bvh` sidecar codec —
//! the 3D occlusion lane (T-090.6) that retired the 2.5D interpretation model from LOS
//! duty. Moved here from `xtask/src/map_blueprint/bvh.rs` (step 1, verbatim) so the
//! sidecar loader, `evaluate_los` (step 3), and the viewshed pipeline (step 4) share one
//! implementation; xtask keeps only the CLI (`map bvh-parity` / `map bvh-emit`).
//!
//! The triangle test ignores winding (|det|): any-hit occlusion is orientation-agnostic,
//! which sidesteps the COLL winding-inversion trap entirely.
//!
//! # Sidecar format v1 (`<slug>.bvh`, little-endian throughout)
//!
//! | Section | Bytes |
//! |---|---|
//! | header | 32: magic `b"TBVH"` · version u32 = 1 · nverts u32 · ntris u32 · nnodes u32 · reserved u32×3 = 0 |
//! | verts | nverts × 3 × f32 |
//! | tris | ntris × 3 × u32 |
//! | nodes | nnodes × 32 (min f32×3 · max f32×3 · left_first u32 · count u32) |
//! | tri_order | ntris × u32 |
//!
//! Determinism contract: the emitter quantizes verts f64→f32 ([`quantize_verts`]), lifts
//! back ([`lift_verts`], exact), and builds the BVH over the lifted values — so raycasts
//! over a parsed sidecar are bit-identical to the emitter's, and
//! `emit(parse(emit(x))) == emit(x)`. The build itself uses only IEEE basic ops and
//! `total_cmp` sorts on a fixed input order — byte-identical output across runs and
//! targets. [`BvhSidecar::parse`] fully validates structure so [`Bvh::any_hit`] can
//! neither panic nor hang on arbitrary input bytes (sidecars are fetched over HTTP later).

/// Conservative pad on f64→f32 node-bound storage. AABBs only cull, so padding can only
/// cost extra node visits, never a false miss; at building scale (|coord| ≤ ~30 m) the
/// f32 cast error is ≤ ~4e-6 m, three orders under the pad.
const AABB_PAD: f64 = 1e-3;
const LEAF_MAX: usize = 8;
const MAX_DEPTH: usize = 32;
/// |det| below this = segment parallel to the triangle plane (or degenerate triangle) —
/// same threshold as the voxel marcher's Möller–Trumbore in xtask's `mesh.rs`.
const DET_EPS: f64 = 1e-12;
/// Barycentric slack (xtask `mesh.rs` precedent): closes shared-edge cracks between
/// 0.01-quantized COLL verts without measurably expanding any triangle.
const BARY_EPS: f64 = 1e-9;

/// Parse-time depth bound. [`Bvh::any_hit`] walks with a fixed 64-slot stack (net +1 per
/// level); rejecting > 60 at parse keeps hostile-but-forward files from overflowing it.
const MAX_PARSE_DEPTH: u32 = 60;

pub fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
pub fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
pub fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// Both-sided Möller–Trumbore for segment p→q against triangle (a, b, c). Winding is
/// ignored. Returns the raw segment parameter t — the CALLER applies the [t_lo, t_hi]
/// range check (traversal, tests, and diagnostics each own their range).
pub fn segment_hits_tri(
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
/// `tri_order[left_first .. left_first + count]`. This layout IS the sidecar's node
/// record — the size assert below doubles as the format's stride guarantee.
#[derive(Debug, Clone, Copy)]
struct BvhNode {
    min: [f32; 3],
    max: [f32; 3],
    left_first: u32,
    /// 0 = internal node, > 0 = leaf triangle count.
    count: u32,
}
const _: () = assert!(std::mem::size_of::<BvhNode>() == 32);

/// Owns no geometry: `build` and `any_hit` take the same `verts`/`tris` slices, and the
/// `Hit::tri` index is the ORIGINAL triangle index (valid into `tris` and any parallel
/// per-triangle table the caller keeps).
#[derive(Debug)]
pub struct Bvh {
    nodes: Vec<BvhNode>,
    tri_order: Vec<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
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
        // Parsed sidecars are depth-bounded to MAX_PARSE_DEPTH for the same reason.
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
                    ) && t >= t_lo
                        && t <= t_hi
                    {
                        return Some(Hit { t, tri });
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

    /// Closest-hit traversal over segment p→q: same slab walk as [`Bvh::any_hit`] but
    /// tracks the best t and shrinks the range instead of returning on first acceptance —
    /// `first_hit(..).is_none()` ⇔ `any_hit(..).is_none()` by construction (existence is
    /// decided by the same [t_lo, t_hi] test; only the returned t/tri differ).
    pub fn first_hit(
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
        let mut best: Option<Hit> = None;
        let mut t_best = t_hi;
        let mut stack = [0u32; 64];
        stack[0] = 0;
        let mut sp = 1usize;
        while sp > 0 {
            sp -= 1;
            let node = &self.nodes[stack[sp] as usize];
            let mut lo = t_lo;
            let mut hi = t_best;
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
                    ) && t >= t_lo
                        && t <= t_best
                    {
                        best = Some(Hit { t, tri });
                        t_best = t;
                    }
                }
            } else {
                debug_assert!(sp + 2 <= stack.len());
                stack[sp] = node.left_first;
                stack[sp + 1] = node.left_first + 1;
                sp += 2;
            }
        }
        best
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

/* ───────────────────────────── the .bvh sidecar codec ───────────────────────────── */

pub const SIDECAR_MAGIC: [u8; 4] = *b"TBVH";
pub const SIDECAR_VERSION: u32 = 1;
const HEADER_LEN: usize = 32;

/// The ONE authority for the determinism cast: quantize f64→f32 per component, then
/// build the BVH over [`lift_verts`] of the result — never over the raw f64s.
pub fn quantize_verts(verts: &[[f64; 3]]) -> Vec<[f32; 3]> {
    verts
        .iter()
        .map(|v| [v[0] as f32, v[1] as f32, v[2] as f32])
        .collect()
}

/// Exact widening — `lift_verts(quantize_verts(v))` round-trips every representable f32.
pub fn lift_verts(verts: &[[f32; 3]]) -> Vec<[f64; 3]> {
    verts
        .iter()
        .map(|v| [f64::from(v[0]), f64::from(v[1]), f64::from(v[2])])
        .collect()
}

/// Every way [`BvhSidecar::parse`] rejects bytes. The battery in `bvh_tests.rs` exercises
/// one case per variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BvhParseError {
    TooShort { len: usize },
    BadMagic([u8; 4]),
    UnsupportedVersion(u32),
    NonZeroReserved,
    EmptyMesh,
    LengthMismatch { expected: u64, actual: u64 },
    NonFiniteVert { vert: u32 },
    TriIndexOutOfBounds { tri: u32 },
    NonFiniteNodeBound { node: u32 },
    LeafRangeOutOfBounds { node: u32 },
    ChildOutOfBounds { node: u32 },
    NodeRevisited { node: u32 },
    OrphanNodes { visited: u32, nnodes: u32 },
    TreeTooDeep,
    LeafCoverageMismatch { covered: u64, ntris: u32 },
    TriOrderOutOfBounds { slot: u32 },
    TriOrderNotPermutation { tri: u32 },
}

impl core::fmt::Display for BvhParseError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::TooShort { len } => write!(f, "sidecar too short: {len} bytes < 32-byte header"),
            Self::BadMagic(m) => write!(f, "bad magic {m:?} (want TBVH)"),
            Self::UnsupportedVersion(v) => write!(f, "unsupported sidecar version {v} (want 1)"),
            Self::NonZeroReserved => write!(f, "reserved header words are not zero"),
            Self::EmptyMesh => write!(f, "empty mesh (zero verts, tris, or nodes)"),
            Self::LengthMismatch { expected, actual } => {
                write!(f, "header implies {expected} bytes, file has {actual}")
            }
            Self::NonFiniteVert { vert } => write!(f, "vertex {vert} has a non-finite component"),
            Self::TriIndexOutOfBounds { tri } => {
                write!(f, "triangle {tri} indexes past the vertex table")
            }
            Self::NonFiniteNodeBound { node } => write!(f, "node {node} has a non-finite bound"),
            Self::LeafRangeOutOfBounds { node } => {
                write!(f, "leaf node {node} range exceeds tri_order")
            }
            Self::ChildOutOfBounds { node } => {
                write!(
                    f,
                    "internal node {node} children out of bounds or not forward-only"
                )
            }
            Self::NodeRevisited { node } => write!(f, "node {node} is reachable twice (diamond)"),
            Self::OrphanNodes { visited, nnodes } => {
                write!(
                    f,
                    "only {visited} of {nnodes} nodes reachable from the root"
                )
            }
            Self::TreeTooDeep => write!(f, "tree depth exceeds {MAX_PARSE_DEPTH}"),
            Self::LeafCoverageMismatch { covered, ntris } => {
                write!(
                    f,
                    "leaves do not tile tri_order exactly ({covered} of {ntris} slots)"
                )
            }
            Self::TriOrderOutOfBounds { slot } => {
                write!(f, "tri_order slot {slot} indexes past the triangle table")
            }
            Self::TriOrderNotPermutation { tri } => {
                write!(f, "tri_order repeats triangle {tri}")
            }
        }
    }
}

impl std::error::Error for BvhParseError {}

/// A parsed sidecar, ready to raycast: verts are pre-lifted to f64 so
/// `sc.bvh.any_hit(&sc.verts, &sc.tris, ..)` is the whole call.
#[derive(Debug)]
pub struct BvhSidecar {
    pub verts: Vec<[f64; 3]>,
    pub tris: Vec<[u32; 3]>,
    pub bvh: Bvh,
}

fn u32le(b: &[u8]) -> u32 {
    u32::from_le_bytes([b[0], b[1], b[2], b[3]])
}
fn f32le(b: &[u8]) -> f32 {
    f32::from_le_bytes([b[0], b[1], b[2], b[3]])
}

impl BvhSidecar {
    /// Full structural validation — after `parse` succeeds, [`Bvh::any_hit`] can neither
    /// panic nor hang no matter what the bytes were. See the module doc for the format.
    pub fn parse(bytes: &[u8]) -> Result<BvhSidecar, BvhParseError> {
        use BvhParseError as E;
        if bytes.len() < HEADER_LEN {
            return Err(E::TooShort { len: bytes.len() });
        }
        let magic: [u8; 4] = bytes[0..4].try_into().unwrap();
        if magic != SIDECAR_MAGIC {
            return Err(E::BadMagic(magic));
        }
        let version = u32le(&bytes[4..8]);
        if version != SIDECAR_VERSION {
            return Err(E::UnsupportedVersion(version));
        }
        let nverts = u32le(&bytes[8..12]);
        let ntris = u32le(&bytes[12..16]);
        let nnodes = u32le(&bytes[16..20]);
        if bytes[20..32].iter().any(|&b| b != 0) {
            return Err(E::NonZeroReserved);
        }
        if nverts == 0 || ntris == 0 || nnodes == 0 {
            return Err(E::EmptyMesh);
        }
        // Entirely in u64 (max term < 2^37): only after this equality do usize section
        // offsets exist, which keeps 32-bit wasm free of overflow by construction. Exact
        // equality also rejects trailing garbage.
        let expected = HEADER_LEN as u64
            + 12 * u64::from(nverts)
            + 12 * u64::from(ntris)
            + 32 * u64::from(nnodes)
            + 4 * u64::from(ntris);
        if bytes.len() as u64 != expected {
            return Err(E::LengthMismatch {
                expected,
                actual: bytes.len() as u64,
            });
        }

        let mut off = HEADER_LEN;
        let mut verts_f32: Vec<[f32; 3]> = Vec::with_capacity(nverts as usize);
        for (i, rec) in bytes[off..off + nverts as usize * 12]
            .chunks_exact(12)
            .enumerate()
        {
            let v = [f32le(&rec[0..4]), f32le(&rec[4..8]), f32le(&rec[8..12])];
            if v.iter().any(|c| !c.is_finite()) {
                return Err(E::NonFiniteVert { vert: i as u32 });
            }
            verts_f32.push(v);
        }
        off += nverts as usize * 12;

        let mut tris: Vec<[u32; 3]> = Vec::with_capacity(ntris as usize);
        for (i, rec) in bytes[off..off + ntris as usize * 12]
            .chunks_exact(12)
            .enumerate()
        {
            let t = [u32le(&rec[0..4]), u32le(&rec[4..8]), u32le(&rec[8..12])];
            if t.iter().any(|&idx| idx >= nverts) {
                return Err(E::TriIndexOutOfBounds { tri: i as u32 });
            }
            tris.push(t);
        }
        off += ntris as usize * 12;

        let mut nodes: Vec<BvhNode> = Vec::with_capacity(nnodes as usize);
        for (i, rec) in bytes[off..off + nnodes as usize * 32]
            .chunks_exact(32)
            .enumerate()
        {
            let min = [f32le(&rec[0..4]), f32le(&rec[4..8]), f32le(&rec[8..12])];
            let max = [
                f32le(&rec[12..16]),
                f32le(&rec[16..20]),
                f32le(&rec[20..24]),
            ];
            if min.iter().chain(max.iter()).any(|c| !c.is_finite()) {
                return Err(E::NonFiniteNodeBound { node: i as u32 });
            }
            let left_first = u32le(&rec[24..28]);
            let count = u32le(&rec[28..32]);
            if count > 0 {
                if u64::from(left_first) + u64::from(count) > u64::from(ntris) {
                    return Err(E::LeafRangeOutOfBounds { node: i as u32 });
                }
            } else {
                // Forward-only children: guarantees the reachability walk (and any_hit)
                // terminates; the adjacency pair must both exist.
                if u64::from(left_first) + 1 >= u64::from(nnodes) || left_first as usize <= i {
                    return Err(E::ChildOutOfBounds { node: i as u32 });
                }
            }
            nodes.push(BvhNode {
                min,
                max,
                left_first,
                count,
            });
        }
        off += nnodes as usize * 32;

        let mut tri_order: Vec<u32> = Vec::with_capacity(ntris as usize);
        let mut seen_tri = vec![false; ntris as usize];
        for (slot, rec) in bytes[off..].chunks_exact(4).enumerate() {
            let t = u32le(rec);
            if t >= ntris {
                return Err(E::TriOrderOutOfBounds { slot: slot as u32 });
            }
            if seen_tri[t as usize] {
                return Err(E::TriOrderNotPermutation { tri: t });
            }
            seen_tri[t as usize] = true;
            tri_order.push(t);
        }

        // Structural walk: every node reachable exactly once, depth bounded, and the
        // leaves must tile tri_order exactly (no gaps, no overlaps).
        let mut visited = vec![false; nnodes as usize];
        let mut slot_covered = vec![false; ntris as usize];
        let mut covered = 0u64;
        let mut visit_count = 0u32;
        let mut stack: Vec<(u32, u32)> = vec![(0, 0)];
        while let Some((n, depth)) = stack.pop() {
            if depth > MAX_PARSE_DEPTH {
                return Err(E::TreeTooDeep);
            }
            let ni = n as usize;
            if visited[ni] {
                return Err(E::NodeRevisited { node: n });
            }
            visited[ni] = true;
            visit_count += 1;
            let node = &nodes[ni];
            if node.count > 0 {
                for k in node.left_first..node.left_first + node.count {
                    if slot_covered[k as usize] {
                        return Err(E::LeafCoverageMismatch { covered, ntris });
                    }
                    slot_covered[k as usize] = true;
                    covered += 1;
                }
            } else {
                stack.push((node.left_first, depth + 1));
                stack.push((node.left_first + 1, depth + 1));
            }
        }
        if visit_count != nnodes {
            return Err(E::OrphanNodes {
                visited: visit_count,
                nnodes,
            });
        }
        if covered != u64::from(ntris) {
            return Err(E::LeafCoverageMismatch { covered, ntris });
        }

        Ok(BvhSidecar {
            verts: lift_verts(&verts_f32),
            tris,
            bvh: Bvh { nodes, tri_order },
        })
    }
}

/// Serialize a sidecar. Infallible: malformed *bytes* only enter through [`BvhSidecar::parse`];
/// a mismatched input here is programmer error. Contract: `bvh` was built over
/// `lift_verts(verts_f32)` and exactly these `tris`.
pub fn emit_bytes(verts_f32: &[[f32; 3]], tris: &[[u32; 3]], bvh: &Bvh) -> Vec<u8> {
    assert!(!tris.is_empty(), "emit_bytes on empty mesh");
    assert_eq!(
        bvh.tri_order.len(),
        tris.len(),
        "bvh was built over a different mesh"
    );
    assert!(u32::try_from(verts_f32.len()).is_ok() && u32::try_from(bvh.nodes.len()).is_ok());
    let mut out = Vec::with_capacity(
        HEADER_LEN + verts_f32.len() * 12 + tris.len() * 12 + bvh.nodes.len() * 32 + tris.len() * 4,
    );
    out.extend_from_slice(&SIDECAR_MAGIC);
    out.extend_from_slice(&SIDECAR_VERSION.to_le_bytes());
    out.extend_from_slice(&(verts_f32.len() as u32).to_le_bytes());
    out.extend_from_slice(&(tris.len() as u32).to_le_bytes());
    out.extend_from_slice(&(bvh.nodes.len() as u32).to_le_bytes());
    out.extend_from_slice(&[0u8; 12]);
    for v in verts_f32 {
        for c in v {
            out.extend_from_slice(&c.to_le_bytes());
        }
    }
    for t in tris {
        for i in t {
            out.extend_from_slice(&i.to_le_bytes());
        }
    }
    for n in &bvh.nodes {
        for c in n.min {
            out.extend_from_slice(&c.to_le_bytes());
        }
        for c in n.max {
            out.extend_from_slice(&c.to_le_bytes());
        }
        out.extend_from_slice(&n.left_first.to_le_bytes());
        out.extend_from_slice(&n.count.to_le_bytes());
    }
    for t in &bvh.tri_order {
        out.extend_from_slice(&t.to_le_bytes());
    }
    out
}

#[cfg(test)]
#[path = "bvh_tests.rs"]
pub(crate) mod tests;
