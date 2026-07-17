//! T-165.4 — TBDD corner-density grid (port of `scripts/map-assets/lib/density-grid.mjs`).
//! Pure + deterministic; the byte codec itself lives in `map_engine_core::geometry::tbdd`
//! (`encode_tbdd`/`decode_tbdd`) — this module carries the density-grid constants + the global
//! corner accumulation/slicing used by the world builder + gates.
//!
//! Corner definition: corner (i,j) of chunk (cx,cy) sits at world (cx*512 + i*32, cy*512 + j*32);
//! its count = instances whose rounded-2dp (x,y) falls in [X-16, X+16) × [Y-16, Y+16).

pub const DENSITY_CELL_M: u16 = 32;
pub const DENSITY_COLS: u16 = 17;
pub const DENSITY_ROWS: u16 = 17;
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

/// Accumulate a global corner grid from instance positions.
#[must_use]
pub fn accumulate_corners(
    positions: impl Iterator<Item = (f64, f64)>,
    world_size_m: f64,
) -> Vec<u16> {
    let n = corner_grid_size(world_size_m);
    let mut grid = vec![0u16; n * n];
    for (x, y) in positions {
        let gx = corner_of(x, world_size_m);
        let gy = corner_of(y, world_size_m);
        grid[gy * n + gx] = grid[gy * n + gx].saturating_add(1);
    }
    grid
}

/// Slice a chunk's 17×17 corner window out of the global grid (row-major j*17+i).
#[must_use]
pub fn slice_chunk(global: &[u16], world_size_m: f64, cx: usize, cy: usize) -> Vec<u16> {
    let n = corner_grid_size(world_size_m);
    let cols = DENSITY_COLS as usize;
    let rows = DENSITY_ROWS as usize;
    let corners_per_chunk = 512 / DENSITY_CELL_M as usize; // 16
    let mut out = vec![0u16; cols * rows];
    for j in 0..rows {
        for i in 0..cols {
            let gx = cx * corners_per_chunk + i;
            let gy = cy * corners_per_chunk + j;
            if gx < n && gy < n {
                out[j * cols + i] = global[gy * n + gx];
            }
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
        assert_eq!(
            (g.cols, g.rows),
            (DENSITY_COLS, DENSITY_ROWS)
        );

        let fixture = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../packages/tbd-schema/golden/map-objects/density/density-fixture.bin");
        if fixture.exists() {
            let bytes = std::fs::read(&fixture).unwrap();
            let g = decode_tbdd(&bytes).expect("fixture decode");
            assert_eq!((g.cols, g.rows), (17, 17));
        }
    }

    #[test]
    fn corner_partition_identity() {
        // Every instance lands in exactly one global corner → sum == count (PH-P2-5 identity).
        let world = 12_800.0;
        let pts: Vec<(f64, f64)> = (0..1000)
            .map(|i| ((i * 7 % 12800) as f64, (i * 13 % 12800) as f64))
            .collect();
        let grid = accumulate_corners(pts.iter().copied(), world);
        let sum: u64 = grid.iter().map(|&v| u64::from(v)).sum();
        assert_eq!(sum, 1000);
        assert_eq!(corner_grid_size(world), 401);
    }
}
