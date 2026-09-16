//! Role: grid.
//! Position: `world/terrain/dem` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// A regular meters-ASL grid in world space (row-major, `rows` rows of `cols` samples). Mirror of the TS `DemVectorGrid` interface (`demGrid.ts:14`).
#[derive(Clone, Debug, PartialEq)]
pub struct DemVectorGrid {
    /// Data.
    pub data: Vec<f32>,

    /// Cols.
    pub cols: usize,

    /// Rows.
    pub rows: usize,

    /// World meters between column samples.
    pub cell_x: f64,

    /// World meters between row samples.
    pub cell_y: f64,

    /// Origin x.
    pub origin_x: f64,

    /// Origin y.
    pub origin_y: f64,

    /// Max elevation in the grid (drives the contour level list).
    pub max_elev_m: f64,
}

/// Downsample factor for the base vector grid: 6400² @ 2 m/px → 1600² @ 8 m cells (`demGrid.ts:29`).
pub const DEM_VECTOR_GRID_FACTOR: usize = 4;

/// Canonical apron dem downsample factor value.
pub const APRON_DEM_DOWNSAMPLE_FACTOR: usize = 16;

/// Output dims for a source raster + factor. Mirror of `demGridDims` (`demGrid.ts:32`).
#[must_use]
pub fn dem_grid_dims(width: usize, height: usize, factor: usize) -> (usize, usize) {
    let cols = 2.max(crate::camera::math::shaping::round(width as f64 / factor as f64) as usize);
    let rows = 2.max(crate::camera::math::shaping::round(height as f64 / factor as f64) as usize);
    (cols, rows)
}

fn source_windows(out_count: usize, src_count: usize, factor: usize) -> Vec<u32> {
    let mut win = vec![0u32; 2 * out_count];
    let half = factor as f64 / 2.0;
    for i in 0..out_count {
        let center = if out_count > 1 {
            (i as f64 * (src_count as f64 - 1.0)) / (out_count as f64 - 1.0)
        } else {
            (src_count as f64 - 1.0) / 2.0
        };
        let mut a = crate::camera::math::shaping::round(center - half) as i64;
        let mut b = crate::camera::math::shaping::round(center + half) as i64;
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

/// One-shot box-average downsample. Mirror of `downsampleDemGrid` (`demGrid.ts:98`) driving the full grid through the same per-cell box-average as `downsampleDemGridBand` (bands are just disjoint row ranges, so a single pass is byte-identical to the banded result). The max is the max over the f64 quotients (pre-f32-store), exactly as the TS tracks it.
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

/// Sample grid meters.
#[must_use]
pub fn sample_grid_meters(grid: &DemVectorGrid, x: f64, y: f64) -> Option<f64> {
    if grid.cols == 0 || grid.rows == 0 || grid.cell_x <= 0.0 || grid.cell_y <= 0.0 {
        return None;
    }
    let px = (x - grid.origin_x) / grid.cell_x;
    let py = (y - grid.origin_y) / grid.cell_y;
    if px < 0.0 || py < 0.0 || px > (grid.cols - 1) as f64 || py > (grid.rows - 1) as f64 {
        return None;
    }
    Some(crate::world::terrain::dem::sampling::bilinear_sample(
        &grid.data, grid.cols, grid.rows, px, py,
    ))
}

#[cfg(test)]
#[path = "tests/grid_tests.rs"]
mod tests;
