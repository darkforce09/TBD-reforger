//! Convex-hull triangulation of a small point cloud (T-090.11.2) — the COLL chunk's
//! `UCX_*` convex colliders (shape type 4) store their vertices plus face/edge tables whose
//! exact layout is not decoded; the hull of the vertex set IS the collider, so its faces are
//! rebuilt here. Sizes are tiny (a trunk hull has 10 vertices, a conifer canopy 19), so a
//! brute-force plane enumeration is the simplest robust choice: every point triple that
//! leaves all other points on one side spans a hull face plane; coplanar points are merged
//! into one convex polygon per plane and fan-triangulated, so a ray crosses each face once.

/// Triangles (outward-wound) of the convex hull of `points`. Empty for fewer than four
/// non-coplanar points. `O(n⁴)` — fine up to a few hundred vertices.
#[must_use]
pub fn hull_triangles(points: &[[f64; 3]]) -> Vec<[u32; 3]> {
    let n = points.len();
    if n < 4 {
        return Vec::new();
    }
    // Scale-aware tolerance.
    let mut lo = [f64::INFINITY; 3];
    let mut hi = [f64::NEG_INFINITY; 3];
    for p in points {
        for a in 0..3 {
            lo[a] = lo[a].min(p[a]);
            hi[a] = hi[a].max(p[a]);
        }
    }
    let extent = (0..3)
        .map(|a| hi[a] - lo[a])
        .fold(0.0f64, f64::max)
        .max(1e-6);
    let eps = extent * 1e-6;

    // Unique outward face planes, keyed by rounded (normal, offset).
    let mut planes: Vec<([f64; 3], f64)> = Vec::new();
    let mut keys: std::collections::HashSet<[i64; 4]> = std::collections::HashSet::new();
    for i in 0..n {
        for j in i + 1..n {
            for k in j + 1..n {
                let (a, b, c) = (points[i], points[j], points[k]);
                let mut nrm = cross(sub(b, a), sub(c, a));
                let len = norm(nrm);
                if len < eps * eps {
                    continue;
                }
                nrm = [nrm[0] / len, nrm[1] / len, nrm[2] / len];
                let mut d = dot(nrm, a);
                let mut pos = false;
                let mut neg = false;
                for (m, p) in points.iter().enumerate() {
                    if m == i || m == j || m == k {
                        continue;
                    }
                    let s = dot(nrm, *p) - d;
                    if s > eps {
                        pos = true;
                    } else if s < -eps {
                        neg = true;
                    }
                    if pos && neg {
                        break;
                    }
                }
                if pos && neg {
                    continue;
                }
                if pos {
                    // Points lie on the positive side: flip so the normal points outward.
                    nrm = [-nrm[0], -nrm[1], -nrm[2]];
                    d = -d;
                }
                let key = [
                    (nrm[0] / 1e-5).round() as i64,
                    (nrm[1] / 1e-5).round() as i64,
                    (nrm[2] / 1e-5).round() as i64,
                    (d / eps.max(1e-9)).round() as i64,
                ];
                if keys.insert(key) {
                    planes.push((nrm, d));
                }
            }
        }
    }

    let mut tris = Vec::new();
    for (nrm, d) in planes {
        // Every point on this plane, projected into an in-plane 2D basis.
        let (u, v) = basis(nrm);
        let mut on: Vec<(usize, [f64; 2])> = points
            .iter()
            .enumerate()
            .filter(|(_, p)| (dot(nrm, **p) - d).abs() <= eps)
            .map(|(idx, p)| (idx, [dot(*p, u), dot(*p, v)]))
            .collect();
        if on.len() < 3 {
            continue;
        }
        // Andrew's monotone chain → CCW polygon in (u, v); (u, v, n) is right-handed, so
        // CCW in the (u, v) plane is outward-wound about `n`.
        on.sort_by(|a, b| {
            a.1[0]
                .partial_cmp(&b.1[0])
                .unwrap()
                .then(a.1[1].partial_cmp(&b.1[1]).unwrap())
        });
        on.dedup_by(|a, b| (a.1[0] - b.1[0]).abs() <= eps && (a.1[1] - b.1[1]).abs() <= eps);
        if on.len() < 3 {
            continue;
        }
        let cross2 = |o: [f64; 2], a: [f64; 2], b: [f64; 2]| {
            (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])
        };
        let mut lower: Vec<(usize, [f64; 2])> = Vec::new();
        for p in &on {
            while lower.len() >= 2
                && cross2(lower[lower.len() - 2].1, lower[lower.len() - 1].1, p.1) <= eps * eps
            {
                lower.pop();
            }
            lower.push(*p);
        }
        let mut upper: Vec<(usize, [f64; 2])> = Vec::new();
        for p in on.iter().rev() {
            while upper.len() >= 2
                && cross2(upper[upper.len() - 2].1, upper[upper.len() - 1].1, p.1) <= eps * eps
            {
                upper.pop();
            }
            upper.push(*p);
        }
        lower.pop();
        upper.pop();
        let ring: Vec<usize> = lower.into_iter().chain(upper).map(|(i, _)| i).collect();
        if ring.len() < 3 {
            continue;
        }
        for k in 1..ring.len() - 1 {
            tris.push([ring[0] as u32, ring[k] as u32, ring[k + 1] as u32]);
        }
    }
    tris
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn norm(a: [f64; 3]) -> f64 {
    dot(a, a).sqrt()
}

/// Two unit vectors spanning the plane orthogonal to `n`, with `(u, v, n)` right-handed.
fn basis(n: [f64; 3]) -> ([f64; 3], [f64; 3]) {
    let helper = if n[0].abs() < 0.9 {
        [1.0, 0.0, 0.0]
    } else {
        [0.0, 1.0, 0.0]
    };
    let mut u = cross(helper, n);
    let l = norm(u);
    u = [u[0] / l, u[1] / l, u[2] / l];
    let v = cross(n, u);
    (u, v)
}

#[cfg(test)]
#[path = "../tests/hull/tests.rs"]
mod tests;
