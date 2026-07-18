//! T-165.4 — TBDD corner-density grid (port of `scripts/map-assets/lib/density-grid.mjs`).
//! Pure + deterministic; the byte codec itself lives in `map_engine_core::geometry::tbdd`
//! (`encode_tbdd`/`decode_tbdd`) — this module carries the density-grid constants + the global
//! corner accumulation/slicing used by the world builder + gates.
//!
//! Corner definition: corner (i,j) of chunk (cx,cy) sits at world
//! (cx*512 + i*`DENSITY_CELL_M`, cy*512 + j*`DENSITY_CELL_M`); its count = instances whose
//! rounded-2dp (x,y) falls in [X-cell/2, X+cell/2) × [Y-cell/2, Y+cell/2).
//!
//! T-176 A2 — the tree channel written to disk is the raw corner counts **box-blurred** into a
//! canopy field (`box_blur_corners` at `CANOPY_KERNEL_RADIUS_CELLS`); the rock channel stays raw.

// T-176 A2 — finer forest fidelity: 8 m cells (was 32 m; operator "32 m too big"). A 512 m chunk /
// 8 m = 64 cells → 65 shared-border corners. `TBDD_FILE_BYTES` + `corner_grid_size` cascade.
pub const DENSITY_CELL_M: u16 = 8;
pub const DENSITY_COLS: u16 = 65;
pub const DENSITY_ROWS: u16 = 65;
/// T-176 A2 — canopy box-blur radius in cells applied to the tree channel at bake time (global,
/// pre-slice → seamless per-chunk marching). At 8 m cells r=1 = a 3×3 (~24 m) window: bridges the
/// normal tree spacing (~11 m on Everon) into solid canopy while leaving clearings ≥ ~24 m as holes.
/// Tune together with `map_engine_core::geometry::forest_mass::CANOPY_MASS_ISO`.
pub const CANOPY_KERNEL_RADIUS_CELLS: usize = 1;
pub const DENSITY_CHANNELS: [&str; 2] = ["tree", "rock"];
pub const TBDD_VERSION: u16 = 1;
pub const TBDD_HEADER_BYTES: usize = 16;
pub const TBDD_FILE_BYTES: usize =
    TBDD_HEADER_BYTES + DENSITY_CHANNELS.len() * DENSITY_COLS as usize * DENSITY_ROWS as usize * 2;

/// Global corner-grid side length for a square world (401 for Everon 12800).
#[must_use]
pub fn corner_grid_size(world_size_m: f64) -> usize {
    (world_size_m / f64::from(DENSITY_CELL_M)).floor() as usize + 1
}

/// Global corner index of a coordinate (half-open window [corner-16, corner+16)).
#[must_use]
pub fn corner_of(coord: f64, world_size_m: f64) -> usize {
    let n = corner_grid_size(world_size_m) as i64;
    let g = ((coord + f64::from(DENSITY_CELL_M) / 2.0) / f64::from(DENSITY_CELL_M)).floor() as i64;
    g.clamp(0, n - 1) as usize
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

/// T-176 A2 — separable box-SUM blur of a global corner grid (radius `r` cells, clamped edges).
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
mod tests {
    use super::*;
    use map_engine_core::geometry::tbdd::{decode_tbdd, encode_tbdd};

    /// S13-style synthetic round-trip + the committed fixture decodes with our constants.
    #[test]
    fn encode_decode_round_trip_and_fixture() {
        let cells = DENSITY_COLS as usize * DENSITY_ROWS as usize;
        let a: Vec<u16> = (0..cells as u16).collect();
        let b: Vec<u16> = (0..cells as u16).map(|v| v.wrapping_mul(3)).collect();
        let buf = encode_tbdd(DENSITY_CELL_M, DENSITY_COLS, DENSITY_ROWS, &[&a, &b]);
        assert_eq!(buf.len(), TBDD_FILE_BYTES);
        let g = decode_tbdd(&buf).expect("decode");
        assert_eq!((g.cols, g.rows), (DENSITY_COLS, DENSITY_ROWS));

        let fixture = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../packages/tbd-schema/golden/map-objects/density/density-fixture.bin");
        if fixture.exists() {
            let bytes = std::fs::read(&fixture).unwrap();
            let g = decode_tbdd(&bytes).expect("fixture decode");
            assert_eq!((g.cols, g.rows), (65, 65)); // T-176 A2 — 8 m grid (65 corners / 512 m chunk)
        }
    }

    #[test]
    fn corner_partition_identity() {
        // Every instance lands in exactly one global corner → sum == count (PH-P2-5 identity).
        let world = 12_800.0;
        let pts: Vec<(f64, f64)> = (0..1000)
            .map(|i| ((i * 7 % 12800) as f64, (i * 13 % 12800) as f64))
            .collect();
        let (grid, _) = accumulate_corners(pts.iter().copied(), world);
        let sum: u64 = grid.iter().copied().map(u64::from).sum();
        assert_eq!(sum, 1000);
        assert_eq!(corner_grid_size(world), 401);
    }
}
