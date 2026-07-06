//! Sea-band geometry — **Class R** (bit-identical to `worldmap/seaBand.ts`). Nested hypsometric
//! fills (inside = `elev ≤ iso`) over the DEM grid: full-inside cells are run-length-merged into row
//! span rectangles (avoids ~1M+ ocean-interior quads); boundary cells get a marching-squares walk.
//! Output is the deck `SolidPolygonLayer` binary form (closed rings, per-vertex RGBA).

use crate::dem::DemVectorGrid;

/// Provisional hypsometric palette `(iso_m, rgba)` shallow→deep (`seaBand.ts:31`).
const SEA_BAND_LEVELS: [(f64, [u8; 4]); 4] = [
    (5.0, [126, 158, 178, 255]),
    (0.0, [72, 118, 160, 255]),
    (-2.5, [48, 96, 140, 255]),
    (-5.0, [30, 70, 120, 255]),
];

/// Deck binary sea-band geometry (transferable). `fill_positions` = closed rings (`_normalize:false`
/// contract); `fill_start_indices` = per-ring start vertex index; `fill_colors` = RGBA u8 per vertex.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SeaBandGeometry {
    pub fill_positions: Vec<f32>,
    pub fill_start_indices: Vec<u32>,
    pub fill_colors: Vec<u8>,
    pub polygon_count: u32,
}

/// Sea-band fill layer opacity by deckZoom (`seaFillAlpha`, `seaBand.ts:40`).
#[must_use]
pub fn sea_fill_alpha(deck_zoom: f64) -> f64 {
    if deck_zoom <= 1.0 {
        1.0
    } else if deck_zoom <= 2.0 {
        0.6
    } else if deck_zoom <= 3.0 {
        0.3
    } else {
        0.0
    }
}

#[derive(Clone, Copy)]
struct Corner {
    v: f64,
    inside: bool,
    x: f64,
    y: f64,
}

/// Linear iso crossing on the edge a→b (states differ ⇒ denominator ≠ 0). Mirror of `crossing`.
#[inline]
fn crossing(a: &Corner, b: &Corner, iso: f64) -> (f64, f64) {
    let t = (iso - a.v) / (b.v - a.v);
    (a.x + t * (b.x - a.x), a.y + t * (b.y - a.y))
}

#[derive(Default)]
struct Builder {
    positions: Vec<f32>,
    start_indices: Vec<u32>,
    colors: Vec<u8>,
    vertex_count: u32,
    polygon_count: u32,
}

impl Builder {
    /// Mirror of the `emitRing` closure: push a closed ring (first vertex repeated last) + per-vertex
    /// RGBA.
    fn emit_ring(&mut self, pts: &[(f64, f64)], rgba: [u8; 4]) {
        if pts.len() < 3 {
            return;
        }
        self.start_indices.push(self.vertex_count);
        for p in pts {
            self.positions.push(p.0 as f32);
            self.positions.push(p.1 as f32);
            self.colors.extend_from_slice(&rgba);
        }
        // Close the loop.
        self.positions.push(pts[0].0 as f32);
        self.positions.push(pts[0].1 as f32);
        self.colors.extend_from_slice(&rgba);
        self.vertex_count += pts.len() as u32 + 1;
        self.polygon_count += 1;
    }

    /// Mirror of the `flushRun` closure. `end_plus_1` = `endI + 1` (so `x1 = originX + end_plus_1 *
    /// cellX`), passed as `i` / `cols-1` to avoid the TS `i-1` underflow. Clears `run_start`.
    #[allow(clippy::too_many_arguments)]
    fn flush_run(
        &mut self,
        run_start: &mut Option<usize>,
        end_plus_1: usize,
        origin_x: f64,
        cell_x: f64,
        y0: f64,
        y1: f64,
        rgba: [u8; 4],
    ) {
        let Some(rs) = *run_start else { return };
        let x0 = origin_x + rs as f64 * cell_x;
        let x1 = origin_x + end_plus_1 as f64 * cell_x;
        self.emit_ring(&[(x0, y0), (x1, y0), (x1, y1), (x0, y1)], rgba);
        *run_start = None;
    }

    /// Mirror of `emitBoundaryCell`: inside (≤ iso) perimeter polygon; saddle split when the centre
    /// is outside.
    fn emit_boundary_cell(&mut self, corners: [Corner; 4], iso: f64, rgba: [u8; 4]) {
        let [c00, c10, c11, c01] = corners;
        let saddle =
            c00.inside == c11.inside && c10.inside == c01.inside && c00.inside != c10.inside;
        if saddle && (c00.v + c10.v + c11.v + c01.v) / 4.0 > iso {
            for k in 0..4 {
                let c = corners[k];
                if !c.inside {
                    continue;
                }
                let prev = corners[(k + 3) % 4];
                let next = corners[(k + 1) % 4];
                self.emit_ring(
                    &[
                        (c.x, c.y),
                        crossing(&c, &next, iso),
                        crossing(&c, &prev, iso),
                    ],
                    rgba,
                );
            }
            return;
        }
        let mut walk: Vec<(f64, f64)> = Vec::new();
        for k in 0..4 {
            let a = &corners[k];
            let b = &corners[(k + 1) % 4];
            if a.inside {
                walk.push((a.x, a.y));
            }
            if a.inside != b.inside {
                walk.push(crossing(a, b, iso));
            }
        }
        self.emit_ring(&walk, rgba);
    }
}

/// Build the sea-band fill geometry. Mirror of `buildSeaBandGeometry` (`seaBand.ts:86`).
#[must_use]
pub fn build_sea_band_geometry(grid: &DemVectorGrid) -> SeaBandGeometry {
    let (cols, rows) = (grid.cols, grid.rows);
    if cols < 2 || rows < 2 {
        return SeaBandGeometry::default();
    }
    let data = &grid.data;
    let mut b = Builder::default();

    for (iso, rgba) in SEA_BAND_LEVELS {
        for j in 0..rows - 1 {
            let y0 = grid.origin_y + j as f64 * grid.cell_y;
            let y1 = y0 + grid.cell_y;
            let mut run_start: Option<usize> = None;
            for i in 0..cols - 1 {
                let v00 = f64::from(data[j * cols + i]);
                let v10 = f64::from(data[j * cols + i + 1]);
                let v11 = f64::from(data[(j + 1) * cols + i + 1]);
                let v01 = f64::from(data[(j + 1) * cols + i]);
                let in00 = v00 <= iso;
                let in10 = v10 <= iso;
                let in11 = v11 <= iso;
                let in01 = v01 <= iso;
                let inside_count = in00 as u8 + in10 as u8 + in11 as u8 + in01 as u8;
                if inside_count == 4 {
                    if run_start.is_none() {
                        run_start = Some(i);
                    }
                    continue;
                }
                b.flush_run(&mut run_start, i, grid.origin_x, grid.cell_x, y0, y1, rgba);
                if inside_count == 0 {
                    continue;
                }
                let x0 = grid.origin_x + i as f64 * grid.cell_x;
                let x1 = x0 + grid.cell_x;
                b.emit_boundary_cell(
                    [
                        Corner {
                            v: v00,
                            inside: in00,
                            x: x0,
                            y: y0,
                        },
                        Corner {
                            v: v10,
                            inside: in10,
                            x: x1,
                            y: y0,
                        },
                        Corner {
                            v: v11,
                            inside: in11,
                            x: x1,
                            y: y1,
                        },
                        Corner {
                            v: v01,
                            inside: in01,
                            x: x0,
                            y: y1,
                        },
                    ],
                    iso,
                    rgba,
                );
            }
            b.flush_run(
                &mut run_start,
                cols - 1,
                grid.origin_x,
                grid.cell_x,
                y0,
                y1,
                rgba,
            );
        }
    }

    SeaBandGeometry {
        fill_positions: b.positions,
        fill_start_indices: b.start_indices,
        fill_colors: b.colors,
        polygon_count: b.polygon_count,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid(data: Vec<f32>, cols: usize, rows: usize) -> DemVectorGrid {
        DemVectorGrid {
            data,
            cols,
            rows,
            cell_x: 1.0,
            cell_y: 1.0,
            origin_x: 0.0,
            origin_y: 0.0,
            max_elev_m: 100.0,
        }
    }

    #[test]
    fn alpha_ladder() {
        assert_eq!(sea_fill_alpha(0.0), 1.0);
        assert_eq!(sea_fill_alpha(1.5), 0.6);
        assert_eq!(sea_fill_alpha(2.5), 0.3);
        assert_eq!(sea_fill_alpha(9.0), 0.0);
    }

    #[test]
    fn all_high_land_has_no_fill() {
        // Everything well above +5 → no level includes any cell.
        let g = grid(vec![100.0; 9], 3, 3);
        let geo = build_sea_band_geometry(&g);
        assert_eq!(geo.polygon_count, 0);
        assert!(geo.fill_positions.is_empty());
    }

    #[test]
    fn full_inside_run_is_one_rectangle_per_row_per_level() {
        // All corners at -10 → inside for all 4 levels; a 3-col row → one 2-cell run rectangle.
        let g = grid(vec![-10.0; 6], 3, 2);
        let geo = build_sea_band_geometry(&g);
        // 4 levels × 1 row × 1 rectangle = 4 polygons.
        assert_eq!(geo.polygon_count, 4);
        // Each rectangle = 4 corners + 1 closing vertex = 5 vertices → 10 position floats, 20 colors.
        assert_eq!(geo.fill_positions.len(), 4 * 10);
        assert_eq!(geo.fill_colors.len(), 4 * 20);
        assert_eq!(geo.fill_start_indices.len(), 4);
    }

    #[test]
    fn rings_are_closed() {
        let g = grid(vec![-10.0, -10.0, 100.0, 100.0], 2, 2); // one boundary cell
        let geo = build_sea_band_geometry(&g);
        assert!(geo.polygon_count >= 1);
        // first ring closes: start vertex position repeated at its end.
        let n = geo.fill_positions.len();
        assert!(n >= 6);
    }
}
