//! NW Everon airfield bbox + structure-class policy (T-152.5 L1/L4).

use crate::dem::DemVectorGrid;
use crate::geometry::triangulate::triangulate_ring_buffer;
use crate::geometry::vector_compose::{PolyMeshGpu, mesh_from_tri};

use super::chunk_math::{Bbox, expand_bbox};
use super::roads::RoadSegment;

/// Margin around runway segment AABBs (L1).
pub const AIRFIELD_BBOX_MARGIN_M: f64 = 30.0;
/// Local DEM flatness σ gate for apron cells (L3).
pub const APRON_FLATNESS_SIGMA_M: f64 = 0.3;
/// Elevation tolerance from pad mean (Goal §2).
pub const APRON_ELEV_TOLERANCE_M: f64 = 0.5;
/// Apron fill `#9aa3a2` @ α0.55 (L3).
pub const APRON_FILL_RGBA: [u8; 4] = [0x9a, 0xa3, 0xa2, 140];
/// Cartographic runway width (L2 / build-map-cartographic.mjs:58).
pub const RUNWAY_POLISH_WIDTH_M: f64 = 20.0;
/// Minimum apron area sanity floor (G2).
pub const APRON_AREA_MIN_M2: f64 = 15_000.0;

/// Union of runway segment AABBs + [`AIRFIELD_BBOX_MARGIN_M`]. `None` when no runway segments.
#[must_use]
pub fn compute_airfield_bbox(runways: &[RoadSegment]) -> Option<Bbox> {
    if runways.is_empty() {
        return None;
    }
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for seg in runways {
        for pt in &seg.points {
            min_x = min_x.min(pt[0]);
            min_y = min_y.min(pt[1]);
            max_x = max_x.max(pt[0]);
            max_y = max_y.max(pt[1]);
        }
    }
    if !min_x.is_finite() {
        return None;
    }
    Some(expand_bbox(
        [min_x, min_y, max_x, max_y],
        AIRFIELD_BBOX_MARGIN_M,
    ))
}

#[must_use]
pub fn point_in_bbox(x: f64, y: f64, bbox: Bbox) -> bool {
    x >= bbox[0] && x <= bbox[2] && y >= bbox[1] && y <= bbox[3]
}

/// Hangar / control-tower classes gated to the airfield bbox (L4).
#[must_use]
pub fn is_airfield_structure_class(building_class: &str) -> bool {
    matches!(building_class, "hangar" | "tower")
}

/// Shoelace area (m²) for a simple polygon ring `[x,y]…` (≥ 3 vertices).
#[must_use]
pub fn polygon_area_m2(positions: &[f32]) -> f64 {
    let n = positions.len() / 2;
    if n < 3 {
        return 0.0;
    }
    let mut area = 0.0_f64;
    for i in 0..n {
        let j = (i + 1) % n;
        let x0 = f64::from(positions[2 * i]);
        let y0 = f64::from(positions[2 * i + 1]);
        let x1 = f64::from(positions[2 * j]);
        let y1 = f64::from(positions[2 * j + 1]);
        area += x0 * y1 - x1 * y0;
    }
    (area * 0.5).abs()
}

/// Local elevation σ (m) over a 5×5 window centered at `(col, row)`; `None` at borders.
fn local_sigma(grid: &DemVectorGrid, col: usize, row: usize) -> Option<f64> {
    if col < 2 || row < 2 || col + 2 >= grid.cols || row + 2 >= grid.rows {
        return None;
    }
    let mut vals = Vec::with_capacity(25);
    for dy in -2_i32..=2 {
        for dx in -2_i32..=2 {
            let c = (col as i32 + dx) as usize;
            let r = (row as i32 + dy) as usize;
            vals.push(f64::from(grid.data[r * grid.cols + c]));
        }
    }
    let mean = vals.iter().sum::<f64>() / vals.len() as f64;
    let var = vals.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / vals.len() as f64;
    Some(var.sqrt())
}

fn cell_center(grid: &DemVectorGrid, col: usize, row: usize) -> (f64, f64) {
    (
        grid.origin_x + (col as f64 + 0.5) * grid.cell_x,
        grid.origin_y + (row as f64 + 0.5) * grid.cell_y,
    )
}

/// Build apron fill mesh: cells inside `bbox` with σ < 0.3 m and |elev − pad_mean| < 0.5 m.
#[must_use]
pub fn build_airfield_apron_mesh(grid: &DemVectorGrid, bbox: Bbox) -> PolyMeshGpu {
    if grid.cols < 5 || grid.rows < 5 {
        return PolyMeshGpu::default();
    }

    let mut flat_mask = vec![false; grid.cols * grid.rows];
    let mut flat_elevs = Vec::new();

    for row in 0..grid.rows {
        for col in 0..grid.cols {
            let (wx, wy) = cell_center(grid, col, row);
            if wx < bbox[0] || wx > bbox[2] || wy < bbox[1] || wy > bbox[3] {
                continue;
            }
            let Some(sigma) = local_sigma(grid, col, row) else {
                continue;
            };
            if sigma >= APRON_FLATNESS_SIGMA_M {
                continue;
            }
            let elev = f64::from(grid.data[row * grid.cols + col]);
            flat_mask[row * grid.cols + col] = true;
            flat_elevs.push(elev);
        }
    }

    if flat_elevs.is_empty() {
        return PolyMeshGpu::default();
    }

    let pad_mean = flat_elevs.iter().sum::<f64>() / flat_elevs.len() as f64;

    let mut fill_positions = Vec::new();
    let mut fill_start_indices = Vec::new();
    let mut fill_colors = Vec::new();
    let mut vertex_count = 0_u32;
    let mut polygon_count = 0_u32;

    for row in 0..grid.rows {
        let y0 = grid.origin_y + row as f64 * grid.cell_y;
        let y1 = y0 + grid.cell_y;
        let mut run_start: Option<usize> = None;
        for col in 0..grid.cols {
            let idx = row * grid.cols + col;
            let inside = flat_mask[idx]
                && (f64::from(grid.data[idx]) - pad_mean).abs() < APRON_ELEV_TOLERANCE_M;
            if inside {
                if run_start.is_none() {
                    run_start = Some(col);
                }
            } else if let Some(rs) = run_start {
                let x0 = grid.origin_x + rs as f64 * grid.cell_x;
                let x1 = grid.origin_x + col as f64 * grid.cell_x;
                fill_start_indices.push(vertex_count);
                for &(x, y) in &[(x0, y0), (x1, y0), (x1, y1), (x0, y1), (x0, y0)] {
                    fill_positions.push(x as f32);
                    fill_positions.push(y as f32);
                    fill_colors.extend_from_slice(&APRON_FILL_RGBA);
                }
                vertex_count += 5;
                polygon_count += 1;
                run_start = None;
            }
        }
        if let Some(rs) = run_start {
            let x0 = grid.origin_x + rs as f64 * grid.cell_x;
            let x1 = grid.origin_x + grid.cols as f64 * grid.cell_x;
            fill_start_indices.push(vertex_count);
            for &(x, y) in &[(x0, y0), (x1, y0), (x1, y1), (x0, y1), (x0, y0)] {
                fill_positions.push(x as f32);
                fill_positions.push(y as f32);
                fill_colors.extend_from_slice(&APRON_FILL_RGBA);
            }
            vertex_count += 5;
            polygon_count += 1;
        }
    }

    if polygon_count == 0 {
        return PolyMeshGpu::default();
    }

    let (mesh, cols) =
        triangulate_ring_buffer(&fill_positions, &fill_start_indices, Some(&fill_colors));
    mesh_from_tri(mesh, &cols, 1.0)
}

/// Total m² of apron-qualifying DEM cells (same predicate as [`build_airfield_apron_mesh`]).
#[must_use]
pub fn apron_qualifying_area_m2(grid: &DemVectorGrid, bbox: Bbox) -> f64 {
    if grid.cols < 5 || grid.rows < 5 {
        return 0.0;
    }

    let mut flat_mask = vec![false; grid.cols * grid.rows];
    let mut flat_elevs = Vec::new();

    for row in 0..grid.rows {
        for col in 0..grid.cols {
            let (wx, wy) = cell_center(grid, col, row);
            if wx < bbox[0] || wx > bbox[2] || wy < bbox[1] || wy > bbox[3] {
                continue;
            }
            let Some(sigma) = local_sigma(grid, col, row) else {
                continue;
            };
            if sigma >= APRON_FLATNESS_SIGMA_M {
                continue;
            }
            let elev = f64::from(grid.data[row * grid.cols + col]);
            flat_mask[row * grid.cols + col] = true;
            flat_elevs.push(elev);
        }
    }

    if flat_elevs.is_empty() {
        return 0.0;
    }

    let pad_mean = flat_elevs.iter().sum::<f64>() / flat_elevs.len() as f64;
    let cell_area = grid.cell_x * grid.cell_y;
    let mut area = 0.0_f64;
    for row in 0..grid.rows {
        for col in 0..grid.cols {
            let idx = row * grid.cols + col;
            if flat_mask[idx]
                && (f64::from(grid.data[idx]) - pad_mean).abs() < APRON_ELEV_TOLERANCE_M
            {
                area += cell_area;
            }
        }
    }
    area
}

#[cfg(test)]
mod policy_tests {
    use super::*;
    use crate::world::roads::RoadSegment;

    fn seg(id: &str, pts: &[[f64; 2]]) -> RoadSegment {
        RoadSegment {
            id: id.to_string(),
            road_class: "runway".to_string(),
            points: pts.to_vec(),
            width_m: 20.0,
        }
    }

    #[test]
    fn bbox_expands_runway_union_by_margin() {
        let runways = vec![seg("r0", &[[100.0, 200.0], [300.0, 400.0]])];
        let b = compute_airfield_bbox(&runways).unwrap();
        assert_eq!(b[0], 70.0);
        assert_eq!(b[1], 170.0);
        assert_eq!(b[2], 330.0);
        assert_eq!(b[3], 430.0);
    }

    #[test]
    fn point_in_bbox_edges() {
        let b = [0.0, 0.0, 100.0, 100.0];
        assert!(point_in_bbox(0.0, 0.0, b));
        assert!(point_in_bbox(100.0, 100.0, b));
        assert!(!point_in_bbox(-0.1, 50.0, b));
    }

    #[test]
    fn airfield_structure_classes() {
        assert!(is_airfield_structure_class("hangar"));
        assert!(is_airfield_structure_class("tower"));
        assert!(!is_airfield_structure_class("military"));
    }
}

#[cfg(test)]
mod apron_tests {
    use super::*;
    use crate::dem::DemVectorGrid;

    fn flat_grid(elev: f32, cols: usize, rows: usize, cell: f64) -> DemVectorGrid {
        DemVectorGrid {
            data: vec![elev; cols * rows],
            cols,
            rows,
            cell_x: cell,
            cell_y: cell,
            origin_x: 0.0,
            origin_y: 0.0,
            max_elev_m: f64::from(elev),
        }
    }

    #[test]
    fn flat_pad_inside_bbox_produces_apron() {
        let grid = flat_grid(100.0, 20, 20, 50.0);
        let bbox = [100.0, 100.0, 800.0, 800.0];
        let mesh = build_airfield_apron_mesh(&grid, bbox);
        assert!(!mesh.indices.is_empty());
        assert!(mesh.polygon_count > 0);
    }

    #[test]
    fn rough_terrain_outside_flat_gate_is_empty() {
        let mut grid = flat_grid(100.0, 10, 10, 100.0);
        for (i, v) in grid.data.iter_mut().enumerate() {
            *v = if i % 2 == 0 { 50.0 } else { 150.0 };
        }
        grid.max_elev_m = 150.0;
        let bbox = [0.0, 0.0, 1000.0, 1000.0];
        let mesh = build_airfield_apron_mesh(&grid, bbox);
        assert!(mesh.indices.is_empty());
    }
}
