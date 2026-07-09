//! Polygon triangulation for wgpu `PolygonFill` (T-151.4 L1/L8).
//!
//! Uses [`earcutr`] (Mapbox earcut port) for robust simple-ring and multi-hole triangulation —
//! land-cover hulls on Everon carry up to hundreds of hole rings.
//!
//! **Area conservation (L8):** Σ triangle areas == |outer − holes| within ULP-scaled tolerance.

/// One triangulated mesh: interleaved positions (`[x,y]…`) + triangle-list indices.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TriMesh {
    pub positions: Vec<f32>,
    pub indices: Vec<u32>,
}

/// Absolute area of a closed (or unclosed) ring via the shoelace formula.
#[must_use]
pub fn ring_area(ring: &[[f64; 2]]) -> f64 {
    signed_area(&unclose(ring)).abs()
}

fn signed_area(pts: &[[f64; 2]]) -> f64 {
    let n = pts.len();
    if n < 3 {
        return 0.0;
    }
    let mut sum = 0.0;
    for i in 0..n {
        let a = pts[i];
        let b = pts[(i + 1) % n];
        sum += a[0] * b[1] - b[0] * a[1];
    }
    sum * 0.5
}

/// Σ absolute triangle areas in the mesh.
#[must_use]
pub fn triangle_area_sum(mesh: &TriMesh) -> f64 {
    let mut sum = 0.0;
    let pos = &mesh.positions;
    for tri in mesh.indices.chunks_exact(3) {
        let i0 = tri[0] as usize * 2;
        let i1 = tri[1] as usize * 2;
        let i2 = tri[2] as usize * 2;
        if i0 + 1 >= pos.len() || i1 + 1 >= pos.len() || i2 + 1 >= pos.len() {
            continue;
        }
        let ax = f64::from(pos[i0]);
        let ay = f64::from(pos[i0 + 1]);
        let bx = f64::from(pos[i1]);
        let by = f64::from(pos[i1 + 1]);
        let cx = f64::from(pos[i2]);
        let cy = f64::from(pos[i2 + 1]);
        sum += ((bx - ax) * (cy - ay) - (cx - ax) * (by - ay)).abs() * 0.5;
    }
    sum
}

/// L8 tolerance: max of absolute 1e-4 m² and relative 1e-6 of the ring area.
#[must_use]
pub fn area_tolerance(ring_area: f64) -> f64 {
    (1e-4_f64).max(ring_area.abs() * 1e-6)
}

/// Strip trailing close + consecutive duplicates.
#[must_use]
pub fn unclose(ring: &[[f64; 2]]) -> Vec<[f64; 2]> {
    let mut pts: Vec<[f64; 2]> = Vec::with_capacity(ring.len());
    for &p in ring {
        if let Some(prev) = pts.last()
            && (prev[0] - p[0]).abs() < 1e-12
            && (prev[1] - p[1]).abs() < 1e-12
        {
            continue;
        }
        pts.push(p);
    }
    while pts.len() > 1 {
        let first = pts[0];
        let last = *pts.last().expect("len > 1");
        if (first[0] - last[0]).abs() > 1e-12 || (first[1] - last[1]).abs() > 1e-12 {
            break;
        }
        pts.pop();
    }
    pts
}

/// Flatten rings into earcutr's data + hole-index format.
/// `data` = concatenated `[x0,y0,x1,y1,…]`; `hole_indices` = start vertex index of each hole
/// (not byte offset).
fn flatten_rings(outer: &[[f64; 2]], holes: &[Vec<[f64; 2]>]) -> (Vec<f64>, Vec<usize>) {
    let outer = unclose(outer);
    let mut data =
        Vec::with_capacity((outer.len() + holes.iter().map(Vec::len).sum::<usize>()) * 2);
    for p in &outer {
        data.push(p[0]);
        data.push(p[1]);
    }
    let mut hole_indices = Vec::with_capacity(holes.len());
    let mut cursor = outer.len();
    for hole in holes {
        let h = unclose(hole);
        if h.len() < 3 {
            continue;
        }
        hole_indices.push(cursor);
        for p in &h {
            data.push(p[0]);
            data.push(p[1]);
        }
        cursor += h.len();
    }
    (data, hole_indices)
}

/// Ear-clip a simple polygon (no holes). Input may be closed.
#[must_use]
pub fn triangulate_simple(ring: &[[f64; 2]]) -> TriMesh {
    triangulate_with_holes(ring, &[])
}

/// Triangulate an outer ring with optional holes (land-cover multi-ring hulls).
#[must_use]
pub fn triangulate_with_holes(outer: &[[f64; 2]], holes: &[Vec<[f64; 2]>]) -> TriMesh {
    let (data, hole_indices) = flatten_rings(outer, holes);
    if data.len() < 6 {
        return TriMesh::default();
    }
    let dims = 2_usize;
    let tris = if hole_indices.is_empty() {
        earcutr::earcut(&data, &[], dims)
    } else {
        earcutr::earcut(&data, &hole_indices, dims)
    };
    let Ok(tris) = tris else {
        return TriMesh::default();
    };
    let n_verts = data.len() / 2;
    let mut positions = Vec::with_capacity(n_verts * 2);
    for k in 0..n_verts {
        positions.push(data[k * 2] as f32);
        positions.push(data[k * 2 + 1] as f32);
    }
    #[allow(clippy::cast_possible_truncation)]
    let indices: Vec<u32> = tris.iter().map(|&i| i as u32).collect();
    TriMesh { positions, indices }
}

/// Triangulate closed-ring Deck binary form (sea/forest mass): each ring independent.
#[must_use]
pub fn triangulate_ring_buffer(
    fill_positions: &[f32],
    fill_start_indices: &[u32],
    fill_colors_u8: Option<&[u8]>,
) -> (TriMesh, Vec<u8>) {
    let vertex_count = fill_positions.len() / 2;
    if vertex_count == 0 || fill_start_indices.is_empty() {
        return (TriMesh::default(), Vec::new());
    }

    let mut out_pos = Vec::new();
    let mut out_idx = Vec::new();
    let mut out_col = Vec::new();
    let mut base_vertex = 0_u32;

    for (ri, &start) in fill_start_indices.iter().enumerate() {
        let end = if ri + 1 < fill_start_indices.len() {
            fill_start_indices[ri + 1] as usize
        } else {
            vertex_count
        };
        let start = start as usize;
        if end <= start + 2 {
            continue;
        }
        let mut ring = Vec::with_capacity(end - start);
        for vi in start..end {
            ring.push([
                f64::from(fill_positions[vi * 2]),
                f64::from(fill_positions[vi * 2 + 1]),
            ]);
        }
        let mesh = triangulate_simple(&ring);
        if mesh.indices.is_empty() {
            continue;
        }
        let n_verts = mesh.positions.len() / 2;
        out_pos.extend_from_slice(&mesh.positions);
        for &ix in &mesh.indices {
            out_idx.push(base_vertex + ix);
        }
        let rgba = if let Some(cols) = fill_colors_u8 {
            let ci = start * 4;
            if ci + 3 < cols.len() {
                [cols[ci], cols[ci + 1], cols[ci + 2], cols[ci + 3]]
            } else {
                [255, 255, 255, 255]
            }
        } else {
            [255, 255, 255, 255]
        };
        for _ in 0..n_verts {
            out_col.extend_from_slice(&rgba);
        }
        base_vertex += n_verts as u32;
    }

    (
        TriMesh {
            positions: out_pos,
            indices: out_idx,
        },
        out_col,
    )
}

/// Land-cover: first ring outer, rest holes.
#[must_use]
pub fn triangulate_region_rings(rings: &[Vec<[f64; 2]>]) -> TriMesh {
    if rings.is_empty() {
        return TriMesh::default();
    }
    triangulate_with_holes(&rings[0], &rings[1..])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_area_conserved(expected: f64, mesh: &TriMesh) {
        let ta = triangle_area_sum(mesh);
        let tol = area_tolerance(expected);
        assert!(
            (expected - ta).abs() <= tol,
            "area conservation failed: expected={expected} tri={ta} tol={tol} |Δ|={}",
            (expected - ta).abs()
        );
    }

    #[test]
    fn unit_square_area_and_two_tris() {
        let ring = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        let mesh = triangulate_simple(&ring);
        assert_eq!(mesh.indices.len(), 6);
        assert_area_conserved(1.0, &mesh);
    }

    #[test]
    fn closed_ring_stripped() {
        let ring = [[0.0, 0.0], [2.0, 0.0], [2.0, 3.0], [0.0, 3.0], [0.0, 0.0]];
        let mesh = triangulate_simple(&ring);
        assert_area_conserved(6.0, &mesh);
    }

    #[test]
    fn ccw_triangle() {
        let ring = [[0.0, 0.0], [4.0, 0.0], [0.0, 3.0]];
        let mesh = triangulate_simple(&ring);
        assert_eq!(mesh.indices.len(), 3);
        assert_area_conserved(6.0, &mesh);
    }

    #[test]
    fn polygon_with_hole_area_conserved() {
        let outer = [[0.0, 0.0], [4.0, 0.0], [4.0, 4.0], [0.0, 4.0], [0.0, 0.0]];
        let hole = vec![[1.0, 1.0], [1.0, 2.0], [2.0, 2.0], [2.0, 1.0], [1.0, 1.0]];
        let mesh = triangulate_with_holes(&outer, std::slice::from_ref(&hole));
        let expected = ring_area(&outer) - ring_area(&hole);
        assert!((expected - 15.0).abs() < 1e-12);
        assert_area_conserved(expected, &mesh);
        assert!(!mesh.indices.is_empty());
    }

    #[test]
    fn ring_buffer_sea_style_rects() {
        let positions: Vec<f32> = vec![
            0.0, 0.0, 2.0, 0.0, 2.0, 1.0, 0.0, 1.0, 0.0, 0.0, //
            3.0, 0.0, 5.0, 0.0, 5.0, 2.0, 3.0, 2.0, 3.0, 0.0,
        ];
        let starts = [0_u32, 5];
        let colors = [72_u8, 118, 160, 255].repeat(10);
        let (mesh, cols) = triangulate_ring_buffer(&positions, &starts, Some(&colors));
        assert_eq!(mesh.indices.len() / 3, 4);
        assert_eq!(cols.len(), mesh.positions.len() / 2 * 4);
        assert_area_conserved(6.0, &mesh);
    }
}
