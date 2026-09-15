//! Role: node.
//! Position: `spatial/bvh` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Canonical aabb pad value.
pub(crate) const AABB_PAD: f64 = 1e-3;

/// Canonical leaf max value.
pub(crate) const LEAF_MAX: usize = 8;

/// Canonical max depth value.
pub(crate) const MAX_DEPTH: usize = 32;

/// Canonical det eps value.
pub(crate) const DET_EPS: f64 = 1e-12;

/// Canonical bary eps value.
pub(crate) const BARY_EPS: f64 = 1e-9;

/// Parse-time depth bound. [`Bvh::any_hit`] walks with a fixed 64-slot stack (net +1 per level); rejecting > 60 at parse keeps hostile-but-forward files from overflowing it.
pub(crate) const MAX_PARSE_DEPTH: u32 = 60;

/// Sub.
pub fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

/// Cross.
pub fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// Dot.
pub fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// Both-sided Möller–Trumbore for segment p→q against triangle (a, b, c). Winding is ignored. Returns the raw segment parameter t — the CALLER applies the [t_lo, t_hi] range check (traversal, tests, and diagnostics each own their range).
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

/// Flat BVH node, 32 bytes, Wald layout: an internal node's children are adjacent (`left_first` and `left_first + 1`); a leaf covers `tri_order[left_first .. left_first + count]`. This layout IS the sidecar's node record — the size assert below doubles as the format's stride guarantee.
#[derive(Debug, Clone, Copy)]
pub(crate) struct BvhNode {
    /// Min.
    pub(crate) min: [f32; 3],

    /// Max.
    pub(crate) max: [f32; 3],

    /// Left first.
    pub(crate) left_first: u32,

    /// 0 = internal node, > 0 = leaf triangle count.
    pub(crate) count: u32,
}

/// Tri info.
pub(crate) struct TriInfo {
    /// Lo.
    pub(crate) lo: [f64; 3],

    /// Hi.
    pub(crate) hi: [f64; 3],

    /// Centroid.
    pub(crate) centroid: [f64; 3],
}

/// Builder.
pub(crate) struct Builder<'a> {
    /// Nodes.
    pub(crate) nodes: Vec<BvhNode>,

    /// Tri order.
    pub(crate) tri_order: Vec<u32>,

    /// Info.
    pub(crate) info: &'a [TriInfo],
}

impl Builder<'_> {
    /// Build into.
    pub(crate) fn build_into(&mut self, node: usize, start: usize, end: usize, depth: usize) {
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
