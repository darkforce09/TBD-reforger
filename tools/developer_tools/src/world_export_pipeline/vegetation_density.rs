//! TBDD corner-density grid.
//! Pure + deterministic; the byte codec itself lives in `website_map_engine::geometry::tbdd`
//! (`encode_tbdd`/`decode_tbdd`) — this module carries the density-grid constants + the global
//! corner accumulation/slicing used by the world builder + gates.
//!
//! Corner definition: corner (i,j) of chunk (cx,cy) sits at world
//! (cx*512 + i*`DENSITY_CELL_M`, cy*512 + j*`DENSITY_CELL_M`); its count = instances whose
//! rounded-2dp (x,y) falls in [X-cell/2, X+cell/2) × [Y-cell/2, Y+cell/2).
//!
//! The tree channel written to disk is the raw corner counts **box-blurred** into a
//! canopy field (`box_blur_corners` at `CANOPY_KERNEL_RADIUS_CELLS`); the rock channel stays raw.

// Forest fidelity: 8 m cells. A 512 m chunk /
// 8 m = 64 cells → 65 shared-border corners. `TBDD_FILE_BYTES` + `corner_grid_size` cascade.
pub const DENSITY_CELL_M: u16 = 8;
pub const DENSITY_COLS: u16 = 65;
pub const DENSITY_ROWS: u16 = 65;
/// Canopy box-blur radius in cells applied to the tree channel at bake time (global,
/// pre-slice → seamless per-chunk marching). At 8 m cells r=1 = a 3×3 (~24 m) window: bridges the
/// normal tree spacing (~11 m on Everon) into solid canopy while leaving clearings ≥ ~24 m as holes.
/// Tune together with `website_map_engine::geometry::forest_mass::CANOPY_MASS_ISO`.
pub const CANOPY_KERNEL_RADIUS_CELLS: usize = 1;
pub const DENSITY_CHANNELS: [&str; 2] = ["tree", "rock"];
pub const TBDD_VERSION: u16 = 1;
pub const TBDD_HEADER_BYTES: usize = 16;
pub const TBDD_FILE_BYTES: usize =
    TBDD_HEADER_BYTES + DENSITY_CHANNELS.len() * DENSITY_COLS as usize * DENSITY_ROWS as usize * 2;

/// Global corner-grid side length for a square world (**1601** for Everon 12800).
///
/// At the 8 m cell size the grid is `12800 / 8 + 1 = 1601` corners per axis; the number is
/// pinned independently by `corner_partition_identity` below.
#[must_use]
pub fn corner_grid_size(world_size_m: f64) -> usize {
    (world_size_m / f64::from(DENSITY_CELL_M)).floor() as usize + 1
}

/// Global corner index of a coordinate on a grid of `n` corners per side (half-open window
/// [corner-cell/2, corner+cell/2); a coordinate outside the world clamps into the edge corner).
///
/// Separate from `corner_of` so `sample_corners` can index a grid it was **handed**
/// rather than one it re-derives from a world size: the two could disagree, and a sampler that
/// silently reads the wrong corner is the signature defect in miniature.
fn corner_index(coord: f64, n: usize) -> usize {
    let g = ((coord + f64::from(DENSITY_CELL_M) / 2.0) / f64::from(DENSITY_CELL_M)).floor() as i64;
    g.clamp(0, n as i64 - 1) as usize
}

/// Global corner index of a coordinate (half-open window [corner-4, corner+4) at 8 m cells).
#[must_use]
pub fn corner_of(coord: f64, world_size_m: f64) -> usize {
    corner_index(coord, corner_grid_size(world_size_m))
}

/// Read the 8 m corner grid at a world position: the value of the corner
/// [`corner_of`] would assign `(x, y)` to, on a `size`×`size` grid produced by
/// [`accumulate_corners`] (optionally through [`box_blur_corners`], which preserves the shape).
///
/// This is the read the forest-ring smoother (`world::forest_smooth`) samples: the Path B rings
/// are quantised to the 32 m region lattice, and this grid is the only 4×-finer evidence the
/// exporter holds about where the canopy boundary actually runs inside a boundary cell.
///
/// **Read-only — the TBDD format, its header and its writers are untouched.** Out-of-world
/// coordinates clamp into the edge corner (same rule as `corner_of`), and a grid shorter than
/// `size * size` reads 0 rather than panicking, so a mis-sized grid degrades to "no evidence"
/// instead of taking the exporter down.
#[must_use]
pub fn sample_corners(grid: &[u32], size: usize, x: f64, y: f64) -> u32 {
    if size == 0 || grid.len() < size * size {
        return 0;
    }
    grid[corner_index(y, size) * size + corner_index(x, size)]
}

/// Accumulate a global corner grid from instance positions (u32 counts — clamped to u16 only at
/// slice time, exactly like the .mjs).
#[must_use]
pub fn accumulate_corners(
    positions: impl Iterator<Item = (f64, f64)>,
    world_size_m: f64,
) -> (Vec<u32>, usize) {
    let n = corner_grid_size(world_size_m);
    let mut grid = vec![0u32; n * n];
    for (x, y) in positions {
        let gx = corner_of(x, world_size_m);
        let gy = corner_of(y, world_size_m);
        grid[gy * n + gx] += 1;
    }
    (grid, n)
}

/// Separable box-SUM blur of a global corner grid (radius `r` cells, clamped edges).
/// Output corner = Σ raw counts in the (2r+1)² window ≈ "trees within ~(2r+1)·cell m". Turns the
/// sparse fine tree-count grid into a smooth canopy-density field so `forest_mass_from_corners` at
/// `CANOPY_MASS_ISO` hugs real clusters (holes at clearings) instead of speckling. Applied to the
/// **global** grid before per-chunk slicing so adjacent chunks share identical blurred border
/// corners (no seams). Sum (not average) keeps values as integer tree counts, so the marching iso
/// stays a tree-count threshold.
#[must_use]
pub fn box_blur_corners(grid: &[u32], size: usize, r: usize) -> Vec<u32> {
    if r == 0 || size == 0 {
        return grid.to_vec();
    }
    let mut h = vec![0u32; size * size];
    for y in 0..size {
        let row = y * size;
        for x in 0..size {
            let lo = x.saturating_sub(r);
            let hi = (x + r).min(size - 1);
            let mut s = 0u32;
            for v in &grid[row + lo..=row + hi] {
                s += *v;
            }
            h[row + x] = s;
        }
    }
    let mut out = vec![0u32; size * size];
    for x in 0..size {
        for y in 0..size {
            let lo = y.saturating_sub(r);
            let hi = (y + r).min(size - 1);
            let mut s = 0u32;
            for k in lo..=hi {
                s += h[k * size + x];
            }
            out[y * size + x] = s;
        }
    }
    out
}

/// Slice a chunk's `DENSITY_COLS`×`DENSITY_ROWS` corner window out of the global grid (row-major
/// j*COLS+i; stride = `(COLS-1)` shared-border corners per chunk; out-of-range corners read 0; u16
/// clamp @ 65535).
#[must_use]
pub fn slice_chunk_corners(grid: &[u32], size: usize, cx: usize, cy: usize) -> Vec<u16> {
    let cols = DENSITY_COLS as usize;
    let rows = DENSITY_ROWS as usize;
    let mut out = vec![0u16; cols * rows];
    for j in 0..rows {
        let gy = cy * (rows - 1) + j;
        for i in 0..cols {
            let gx = cx * (cols - 1) + i;
            let v = if gx < size && gy < size {
                grid[gy * size + gx]
            } else {
                0
            };
            out[j * cols + i] = v.min(65_535) as u16;
        }
    }
    out
}

#[cfg(test)]
#[path = "tests/vegetation_density/tests.rs"]
mod tests;
