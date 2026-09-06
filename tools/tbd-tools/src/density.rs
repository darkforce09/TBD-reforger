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

/// Global corner-grid side length for a square world (**1601** for Everon 12800).
///
/// T-597: this doc said `401` — the pre-T-176 value, from when `DENSITY_CELL_M` was 32 m.
/// The 8 m migration (T-176 A2, `a5940fad9`) moved it to `12800/8 + 1 = 1601` and updated
/// neither this line nor `corner_partition_identity` below.
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

    /// The 625 committed everon density tiles (`objects/density/*.bin`), sorted.
    ///
    /// A missing or short corpus is a FAILURE, never a skip: the T-935.5 acceptance is *all 625*
    /// tiles, and "the directory was not there" is the shape of a green run that examined nothing.
    fn everon_density_tiles() -> Vec<std::path::PathBuf> {
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../packages/map-assets/everon/objects/density");
        let rd = std::fs::read_dir(&dir)
            .unwrap_or_else(|e| panic!("T-935.5: {} could not be read ({e})", dir.display()));
        let mut files: Vec<std::path::PathBuf> = rd
            .map(|e| e.expect("density dir entry").path())
            .filter(|p| p.extension().is_some_and(|x| x == "bin"))
            .collect();
        files.sort();
        assert_eq!(
            files.len(),
            625,
            "expected 625 everon density tiles in {}, found {}",
            dir.display(),
            files.len()
        );
        files
    }

    /// T-935.5 main goal — **the 625 committed tiles stay valid byte for byte**.
    ///
    /// Decode each tile with the new `cast_slice` decoder and re-emit it through the *unchanged*
    /// `encode_tbdd`; the result must be the file, byte for byte. This is the independent half of
    /// the parity pin in `map_engine_core::geometry::tbdd`: that one proves the two decoders agree
    /// with each other, this one proves the pair still agrees with what is on disk — the emitter
    /// and the decoder could have drifted together and neither test alone would notice.
    #[test]
    fn committed_everon_tiles_survive_decode_then_re_emit_byte_for_byte() {
        let mut nonzero = 0u64;
        for path in &everon_density_tiles() {
            let on_disk = std::fs::read(path).expect("read density tile");
            assert_eq!(
                on_disk.len(),
                TBDD_FILE_BYTES,
                "{} is {} B, not TBDD_FILE_BYTES ({TBDD_FILE_BYTES}) — a `vers…` prefix here means \
                 this checkout holds an LFS POINTER, not the payload",
                path.display(),
                on_disk.len()
            );
            let g = decode_tbdd(&on_disk).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            assert_eq!(
                (g.cols, g.rows, g.cell_m, g.version),
                (DENSITY_COLS, DENSITY_ROWS, DENSITY_CELL_M, TBDD_VERSION),
                "{} header disagrees with this module's constants",
                path.display()
            );
            assert_eq!(g.channels.len(), DENSITY_CHANNELS.len());
            let refs: Vec<&[u16]> = g.channels.iter().map(Vec::as_slice).collect();
            let re_emitted = encode_tbdd(DENSITY_CELL_M, DENSITY_COLS, DENSITY_ROWS, &refs);
            if re_emitted != on_disk {
                let at = re_emitted
                    .iter()
                    .zip(&on_disk)
                    .position(|(a, b)| a != b)
                    .unwrap_or_else(|| on_disk.len().min(re_emitted.len()));
                panic!(
                    "T-935.5: {} changed on decode→encode at byte {at} ({} B out vs {} B on disk)",
                    path.display(),
                    re_emitted.len(),
                    on_disk.len()
                );
            }
            nonzero += g
                .channels
                .iter()
                .flatten()
                .filter(|v| **v != 0)
                .count()
                .try_into()
                .unwrap_or(u64::MAX);
        }
        assert!(
            nonzero > 0,
            "every cell in all 625 tiles is zero — the round trip above compared nothing but \
             padding"
        );
    }

    /// T-935.5 — a synthetic tile emitted through this module's own pipeline (accumulate → blur →
    /// slice → `encode_tbdd`) decodes back to exactly the corner values that were sliced, and the
    /// bytes match a header/payload string spelled out independently of `encode_tbdd`.
    #[test]
    fn synthetic_tile_emit_decode_round_trip() {
        let world = 1024.0; // two 512 m chunks per side
        let pts: Vec<(f64, f64)> = (0..4096)
            .map(|i| (f64::from(i * 13 % 1024), f64::from(i * 29 % 1024)))
            .collect();
        let (raw, size) = accumulate_corners(pts.iter().copied(), world);
        let blurred = box_blur_corners(&raw, size, CANOPY_KERNEL_RADIUS_CELLS);
        let tree = slice_chunk_corners(&blurred, size, 1, 0);
        let rock = slice_chunk_corners(&raw, size, 1, 0);
        assert!(
            tree.iter().any(|v| *v != 0) && rock.iter().any(|v| *v != 0),
            "the synthetic tile is all zeros — it would round-trip vacuously"
        );

        let buf = encode_tbdd(DENSITY_CELL_M, DENSITY_COLS, DENSITY_ROWS, &[&tree, &rock]);
        assert_eq!(buf.len(), TBDD_FILE_BYTES);

        // Independent byte oracle: the layout spelled from this module's constants, not from
        // `encode_tbdd`'s body.
        let mut want = Vec::with_capacity(TBDD_FILE_BYTES);
        want.extend_from_slice(b"TBDD");
        want.extend_from_slice(&TBDD_VERSION.to_le_bytes());
        want.extend_from_slice(&DENSITY_CELL_M.to_le_bytes());
        want.extend_from_slice(&DENSITY_COLS.to_le_bytes());
        want.extend_from_slice(&DENSITY_ROWS.to_le_bytes());
        want.push(u8::try_from(DENSITY_CHANNELS.len()).expect("2 channels"));
        want.extend_from_slice(&[0, 0, 0]);
        for ch in [&tree, &rock] {
            for v in ch {
                want.extend_from_slice(&v.to_le_bytes());
            }
        }
        assert_eq!(want.len(), TBDD_HEADER_BYTES + 2 * tree.len() * 2);
        assert_eq!(buf, want, "the emitted TBDD layout moved");

        let g = decode_tbdd(&buf).expect("decode");
        assert_eq!(
            (g.cols, g.rows, g.cell_m),
            (DENSITY_COLS, DENSITY_ROWS, DENSITY_CELL_M)
        );
        assert_eq!(
            g.channels[0], tree,
            "tree channel did not survive the round trip"
        );
        assert_eq!(
            g.channels[1], rock,
            "rock channel did not survive the round trip"
        );
    }

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
        // T-597: was `401`. The partition identity above (sum == count) is cell-size agnostic and
        // was always correct; only this literal was stale. T-176 A2 (`a5940fad9`) took
        // DENSITY_CELL_M from 32 m to 8 m, so corner_grid_size(12800) went 12800/32 + 1 = 401 to
        // 12800/8 + 1 = 1601, and this assertion has been RED on every run since.
        //
        // Deliberately an INDEPENDENT literal and not `12800 / DENSITY_CELL_M as usize + 1`:
        // spelling the formula here would just restate `corner_grid_size`'s body, so it would
        // agree with any cell size including a wrong one — an assertion that cannot fail. A flat
        // 1601 is the thing a reader can check against the T-178 Class-R pin of the same number.
        assert_eq!(corner_grid_size(world), 1601);
    }

    /// T-298 — SplitMix64, the reference constant set, inlined.
    ///
    /// `tbd-tools` carries no `rand` dependency and a partition pin is not a reason to grow the
    /// dependency graph. Seeded once below, so the 64 grids are the *same* 64 grids on every
    /// machine and in CI: a randomized test that cannot be reproduced from its own source is a
    /// test whose red nobody can act on.
    struct Rng(u64);

    impl Rng {
        fn next_u64(&mut self) -> u64 {
            self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = self.0;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^ (z >> 31)
        }

        /// `0..n`. The modulo bias is irrelevant here — nothing in this test is a statistical
        /// claim, the draws only have to be varied and reproducible.
        fn below(&mut self, n: u64) -> u64 {
            self.next_u64() % n
        }

        /// One instance coordinate. Deliberately NOT uniform over the world: a sixth of the draws
        /// land exactly on a window seam (`k * DENSITY_CELL_M / 2` — the `[c-cell/2, c+cell/2)`
        /// boundary, where a half-open window either keeps the point or hands it to its
        /// neighbour), and two sixths fall OUTSIDE `[0, world]`, where `corner_of` clamps into the
        /// edge corner. Both are cases the hand-built lattice above cannot reach, and in both the
        /// identity must survive: a clamped point is still exactly one point.
        fn coord(&mut self, world: f64) -> f64 {
            let half = u64::from(DENSITY_CELL_M) / 2;
            let span = world as u64;
            let outside = 4 * u64::from(DENSITY_CELL_M);
            match self.below(6) {
                0 => (self.below(span / half + 1) * half) as f64,
                1 => -((self.below(outside) + 1) as f64),
                2 => world + (self.below(outside) + 1) as f64,
                _ => self.below(span * 100) as f64 / 100.0,
            }
        }
    }

    /// T-298 — the corner partition identity, seeded and randomized over 64 grids.
    ///
    /// `corner_partition_identity` above is ONE sample: one world size, one hand-built lattice of
    /// 1000 points, none of them on a window seam, none outside the world (so the clamp is never
    /// exercised) and never sliced back out per chunk. It is also, with
    /// `encode_decode_round_trip_and_fixture`, the whole of this module's coverage — and until
    /// T-298 both ran in NO workflow (ci.yml had no tbd-tools step; the wave gate's
    /// `test xtask+tbd-tools` is local-only and the mod gate scopes itself to `enf::`), which is
    /// how the stale `401` in the test above sat red from T-176 to T-597, four weeks. This is the
    /// sweep that goes with the CI lane: fixed seed, 64 pseudo-random worlds, three oracles that
    /// each fail for a different reason.
    #[test]
    fn seeded_random_corner_partition_identity() {
        // The world exporter's chunk side (module doc: corner (i,j) of chunk (cx,cy) sits at
        // `cx*512 + i*DENSITY_CELL_M`). A chunk's corner window has to span exactly one chunk —
        // that is the relation T-176 A2 moved (32 m → 8 m, so 17 → 65 corners) and that nothing
        // was checking. A flat 512 for the same reason the 1601 above is flat.
        const CHUNK_M: usize = 512;
        let cols = DENSITY_COLS as usize;
        let rows = DENSITY_ROWS as usize;
        let stride = cols - 1;
        assert_eq!(
            CHUNK_M / DENSITY_CELL_M as usize + 1,
            cols,
            "DENSITY_COLS must span one {CHUNK_M} m chunk at {DENSITY_CELL_M} m per cell"
        );
        assert_eq!(cols, rows, "the chunk corner window is square");

        let mut rng = Rng(0x0000_0298_5EED_1601);
        for grid in 0..64u32 {
            let chunks = 1 + rng.below(6) as usize;
            let world = (chunks * CHUNK_M) as f64;
            let count = 8 + rng.below(193) as usize;
            let pts: Vec<(f64, f64)> = (0..count)
                .map(|_| (rng.coord(world), rng.coord(world)))
                .collect();

            let (corners, size) = accumulate_corners(pts.iter().copied(), world);

            // ORACLE 1 — geometry. `chunks` chunk-windows wide, minus the borders they share.
            // Reds if `corner_grid_size` or `DENSITY_CELL_M` moves without the other following.
            let want = chunks * stride + 1;
            assert_eq!(
                size, want,
                "grid {grid}: {chunks} chunks of {CHUNK_M} m must give {want} corners"
            );
            assert_eq!(corners.len(), size * size, "grid {grid}: grid is not size²");

            // ORACLE 2 — partition. Every instance lands in exactly one corner; the ones outside
            // the world clamp into an edge corner instead of vanishing. So the counts sum to the
            // number of instances fed in, at any cell size (PH-P2-5).
            let sum: u64 = corners.iter().copied().map(u64::from).sum();
            assert_eq!(
                sum, count as u64,
                "grid {grid}: corner counts sum to {sum}, fed {count} instances"
            );

            // ORACLE 3 — the per-chunk slices are that same partition, re-cut. Each chunk owns its
            // `stride`×`stride` interior; the last one in each direction also owns the shared
            // border it has no neighbour to hand on to. Re-assembled it must reproduce ORACLE 2's
            // sum — a slice stride that is not `DENSITY_COLS - 1` double-counts a shared border
            // (tiled > sum) or steps over a column (tiled < sum).
            let mut tiled = 0u64;
            for cy in 0..chunks {
                for cx in 0..chunks {
                    let win = slice_chunk_corners(&corners, size, cx, cy);
                    assert_eq!(
                        win.len(),
                        cols * rows,
                        "grid {grid}: chunk ({cx},{cy}) window is not COLS×ROWS"
                    );
                    let w = if cx + 1 == chunks { cols } else { stride };
                    let h = if cy + 1 == chunks { rows } else { stride };
                    for j in 0..h {
                        for i in 0..w {
                            tiled += u64::from(win[j * cols + i]);
                        }
                    }
                }
            }
            assert_eq!(
                tiled, sum,
                "grid {grid}: chunk slices re-assemble to {tiled}, global grid holds {sum}"
            );
        }
    }
}
