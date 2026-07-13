//! DEM vector-source downsample — **Class R** (bit-identical to `worldmap/demGrid.ts`). Box-average
//! the 6400² meters cache into a ~1600² grid the sea-band + contour geometry march over, plus the
//! 2× reduction for the coarse-interval contour pyramid. Summation order (row-major within the
//! source window) is preserved so the f32 output is `memcmp`-equal.

use crate::js;

/// A regular meters-ASL grid in world space (row-major, `rows` rows of `cols` samples). Mirror of
/// the TS `DemVectorGrid` interface (`demGrid.ts:14`).
#[derive(Clone, Debug, PartialEq)]
pub struct DemVectorGrid {
    pub data: Vec<f32>,
    pub cols: usize,
    pub rows: usize,
    /// World meters between column samples.
    pub cell_x: f64,
    /// World meters between row samples.
    pub cell_y: f64,
    pub origin_x: f64,
    pub origin_y: f64,
    /// Max elevation in the grid (drives the contour level list).
    pub max_elev_m: f64,
}

/// Downsample factor for the base vector grid: 6400² @ 2 m/px → 1600² @ 8 m cells (`demGrid.ts:29`).
pub const DEM_VECTOR_GRID_FACTOR: usize = 4;
/// Coarser grid for NW airfield apron flatness (σ gate on engine-flattened pad; T-152.5 G2).
pub const APRON_DEM_DOWNSAMPLE_FACTOR: usize = 16;

/// Output dims for a source raster + factor. Mirror of `demGridDims` (`demGrid.ts:32`).
#[must_use]
pub fn dem_grid_dims(width: usize, height: usize, factor: usize) -> (usize, usize) {
    let cols = 2.max(js::round(width as f64 / factor as f64) as usize);
    let rows = 2.max(js::round(height as f64 / factor as f64) as usize);
    (cols, rows)
}

/// Per-output-index source windows `[a, b)` centered on the sample position. Mirror of the private
/// `sourceWindows` (`demGrid.ts:44`).
fn source_windows(out_count: usize, src_count: usize, factor: usize) -> Vec<u32> {
    let mut win = vec![0u32; 2 * out_count];
    let half = factor as f64 / 2.0;
    for i in 0..out_count {
        let center = if out_count > 1 {
            (i as f64 * (src_count as f64 - 1.0)) / (out_count as f64 - 1.0)
        } else {
            (src_count as f64 - 1.0) / 2.0
        };
        let mut a = js::round(center - half) as i64;
        let mut b = js::round(center + half) as i64;
        if a < 0 {
            a = 0;
        }
        if b > src_count as i64 {
            b = src_count as i64;
        }
        if b <= a {
            b = (src_count as i64).min(a + 1);
        }
        win[2 * i] = a as u32;
        win[2 * i + 1] = b as u32;
    }
    win
}

/// One-shot box-average downsample. Mirror of `downsampleDemGrid` (`demGrid.ts:98`) driving the full
/// grid through the same per-cell box-average as `downsampleDemGridBand` (bands are just disjoint
/// row ranges, so a single pass is byte-identical to the banded result). The max is the max over the
/// f64 quotients (pre-f32-store), exactly as the TS tracks it.
#[must_use]
pub fn downsample_dem_grid<T>(
    data: &[T],
    width: usize,
    height: usize,
    factor: usize,
    world_width_m: f64,
    world_height_m: f64,
) -> DemVectorGrid
where
    T: Copy + Into<f64>,
{
    let (cols, rows) = dem_grid_dims(width, height, factor);
    let col_win = source_windows(cols, width, factor);
    let row_win = source_windows(rows, height, factor);
    let mut out = vec![0f32; cols * rows];
    let mut max = f64::NEG_INFINITY;
    for j in 0..rows {
        let y0 = row_win[2 * j] as usize;
        let y1 = row_win[2 * j + 1] as usize;
        for i in 0..cols {
            let x0 = col_win[2 * i] as usize;
            let x1 = col_win[2 * i + 1] as usize;
            let mut sum = 0.0f64;
            for y in y0..y1 {
                let row_base = y * width;
                for x in x0..x1 {
                    sum += data[row_base + x].into();
                }
            }
            let v = sum / (((y1 - y0) * (x1 - x0)) as f64);
            out[j * cols + i] = v as f32;
            if v > max {
                max = v;
            }
        }
    }
    DemVectorGrid {
        data: out,
        cols,
        rows,
        cell_x: world_width_m / (cols as f64 - 1.0),
        cell_y: world_height_m / (rows as f64 - 1.0),
        origin_x: 0.0,
        origin_y: 0.0,
        max_elev_m: max,
    }
}

/// 2× reduction for the coarse-interval contour pyramid. Mirror of `reduceGrid2x` (`demGrid.ts:126`).
#[must_use]
pub fn reduce_grid_2x(grid: &DemVectorGrid) -> DemVectorGrid {
    let cols = 2.max(grid.cols.div_ceil(2));
    let rows = 2.max(grid.rows.div_ceil(2));
    let mut out = vec![0f32; cols * rows];
    let mut max = f64::NEG_INFINITY;
    for j in 0..rows {
        let sj = (2 * j).min(grid.rows - 1);
        let sj1 = (sj + 1).min(grid.rows - 1);
        for i in 0..cols {
            let si = (2 * i).min(grid.cols - 1);
            let si1 = (si + 1).min(grid.cols - 1);
            let v = (f64::from(grid.data[sj * grid.cols + si])
                + f64::from(grid.data[sj * grid.cols + si1])
                + f64::from(grid.data[sj1 * grid.cols + si])
                + f64::from(grid.data[sj1 * grid.cols + si1]))
                / 4.0;
            out[j * cols + i] = v as f32;
            if v > max {
                max = v;
            }
        }
    }
    DemVectorGrid {
        data: out,
        cols,
        rows,
        cell_x: grid.cell_x * 2.0,
        cell_y: grid.cell_y * 2.0,
        origin_x: grid.origin_x,
        origin_y: grid.origin_y,
        max_elev_m: max,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dims_and_factor() {
        assert_eq!(DEM_VECTOR_GRID_FACTOR, 4);
        assert_eq!(dem_grid_dims(6400, 6400, 4), (1600, 1600));
        assert_eq!(dem_grid_dims(1, 1, 4), (2, 2)); // clamped to min 2
    }

    #[test]
    fn factor_one_is_identity_with_endpoint_anchoring() {
        // factor 1 → out dims == src dims; each window is a single source cell.
        let data: Vec<f32> = (0..16).map(|v| v as f32).collect();
        let g = downsample_dem_grid(&data, 4, 4, 1, 12.0, 12.0);
        assert_eq!((g.cols, g.rows), (4, 4));
        assert_eq!(g.data, data);
        assert_eq!(g.cell_x, 4.0); // 12 / (4-1)
    }

    #[test]
    fn box_average_of_constant_is_constant() {
        let data = vec![7.0f32; 64];
        let g = downsample_dem_grid(&data, 8, 8, 4, 100.0, 100.0);
        assert!(g.data.iter().all(|&v| (v - 7.0).abs() < 1e-6));
        assert!((g.max_elev_m - 7.0).abs() < 1e-9);
    }

    #[test]
    fn reduce_2x_block_average() {
        // 4×4 grid → 2×2, each out cell = mean of a 2×2 block.
        let data: Vec<f32> = (0..16).map(|v| v as f32).collect();
        let g = DemVectorGrid {
            data,
            cols: 4,
            rows: 4,
            cell_x: 8.0,
            cell_y: 8.0,
            origin_x: 0.0,
            origin_y: 0.0,
            max_elev_m: 15.0,
        };
        let r = reduce_grid_2x(&g);
        assert_eq!((r.cols, r.rows), (2, 2));
        // block {0,1,4,5} → 2.5
        assert!((r.data[0] - 2.5).abs() < 1e-6);
        assert_eq!(r.cell_x, 16.0);
    }
}
