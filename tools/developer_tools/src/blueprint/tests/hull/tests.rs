use super::*;

fn area_and_closed(points: &[[f64; 3]], tris: &[[u32; 3]]) -> (f64, bool) {
    let mut area = 0.0;
    let mut edges: std::collections::HashMap<(u32, u32), i32> = std::collections::HashMap::new();
    for t in tris {
        let (a, b, c) = (
            points[t[0] as usize],
            points[t[1] as usize],
            points[t[2] as usize],
        );
        area += 0.5 * norm(cross(sub(b, a), sub(c, a)));
        for (p, q) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            *edges.entry((p.min(q), p.max(q))).or_default() += if p < q { 1 } else { -1 };
        }
    }
    // A closed, consistently wound surface uses every edge exactly once each way.
    let closed = edges.values().all(|&c| c == 0);
    (area, closed)
}

#[test]
fn cube_hull_is_twelve_outward_triangles() {
    let mut pts = Vec::new();
    for c in 0..8 {
        pts.push([
            if c & 1 != 0 { 1.0 } else { -1.0 },
            if c & 2 != 0 { 2.0 } else { 0.0 },
            if c & 4 != 0 { 0.5 } else { -0.5 },
        ]);
    }
    // Interior + coplanar extra points must not add faces.
    pts.push([0.0, 1.0, 0.0]);
    pts.push([0.0, 2.0, 0.0]);
    let tris = hull_triangles(&pts);
    assert_eq!(tris.len(), 12);
    let (area, closed) = area_and_closed(&pts, &tris);
    assert!(
        (area - (2.0 * (2.0 * 2.0 + 2.0 * 1.0 + 2.0 * 1.0))).abs() < 1e-9,
        "{area}"
    );
    assert!(closed);
    // Outward winding: each face normal points away from the centroid.
    let centroid = [0.0, 1.0, 0.0];
    for t in &tris {
        let (a, b, c) = (pts[t[0] as usize], pts[t[1] as usize], pts[t[2] as usize]);
        let n = cross(sub(b, a), sub(c, a));
        assert!(dot(n, sub(a, centroid)) > 0.0, "inward face {t:?}");
    }
}

#[test]
fn tetrahedron_and_degenerate_inputs() {
    let tet = [
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 1.0],
    ];
    let tris = hull_triangles(&tet);
    assert_eq!(tris.len(), 4);
    assert!(area_and_closed(&tet, &tris).1);
    assert!(hull_triangles(&tet[..3]).is_empty());
    let flat = [
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [1.0, 1.0, 0.0],
    ];
    assert!(
        hull_triangles(&flat).len() <= 4,
        "a flat quad yields at most its two sides"
    );
}

#[test]
fn prism_like_trunk_hull_closes() {
    // Ten points shaped like the farmhouse-scale trunk collider: two pentagons.
    let mut pts = Vec::new();
    for ring in [-11.3, 11.3] {
        for i in 0..5 {
            let a = i as f64 * std::f64::consts::TAU / 5.0;
            pts.push([0.4 * a.cos(), ring, 0.4 * a.sin()]);
        }
    }
    let tris = hull_triangles(&pts);
    let (_, closed) = area_and_closed(&pts, &tris);
    assert!(closed, "{} tris", tris.len());
    assert_eq!(tris.len(), 2 * 3 + 5 * 2);
}
