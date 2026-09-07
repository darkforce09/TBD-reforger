# REPORT T-938.4 — Section cut via BVH and sparse HeightField

## pwd_branch

```
/run/media/system/Disk_2/Projects/TBD-Reforger/.ai/artifacts/worktrees/T-938.4
slice/T-938.4
```

HOST Fedora. Native cargo. `export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target`. Tests that raced the shared cache used `/home/Samuel/.cache/tbd-target-T-938.4`. Wave gate ignored the inherited dir and picked its own. No distrobox-host-exec, no hcargo. Did not merge, push, or drop the worktree.

## defect_verified_on_main

`section_at_owned` walked `occl.tris` in order (building_section.rs:~292). `HeightField::empty` did `vec![None; cols*rows]` of `Option<f64>` (16 B/cell).

**FarmHouse_E_1L01_Wood.bvh** (materialized sidecar: 3170 verts, 2883 tris, 1125 BVH nodes):

| pin | BEFORE (brute / dense) | AFTER (y-index / sparse) |
|---|---|---|
| triangles visited per cut at y=1.2 | **2883** (every triangle) | **443** (BVH y-interval candidates) |
| HeightField plan size | cols=91 rows=117 (cap 2048) | same cols/rows; MAX_PLAN_DIM unchanged |
| empty HeightField bytes | **170352** (91×117×16) | **0** (no plan cells) |
| built HeightField tile bytes (clip y=1.2) | 170352 dense | **34816** |
| 2048² dense Option\<f64\> cap | **67108864** (67 MB) | tiles only on write |

Sidecar header: `TBVH` v2, nverts=3170, ntris=2883, nnodes=1125.

## changes

| path | why |
|---|---|
| `crates/map-engine-core/src/building_section_index.rs` | NEW. `YIntervalIndex` (Wald 1-D BVH on triangle y-extents); `triangles_overlapping_y` culls against existing `Bvh::root_bounds` then walks the y-tree; `SparseHeights` is f32 + NaN in 16×16 tiles allocated on first write. Golden equality + visit/byte tests. |
| `crates/map-engine-core/src/building_section.rs` | `section_at_owned` iterates y-interval candidates; `HeightField.h` is `SparseHeights`; `set` / `allocated_bytes` / `iter_stored` accessors. |
| `crates/map-engine-core/src/lib.rs` | Register `building_section_index` behind `blueprint`. |
| `crates/map-engine-core/src/building_section_tests.rs` | `hf.h[i] = …` → `hf.set`; height pins use 1e-5 (f32 store). Path-include of `building_section.rs`. |

Cut geometry stays f64; the index only culls. Equality vs brute-force is exact after endpoint-sort (order may differ).

## perturbation

Collapsed the y-interval to zero height: `if y_hi <= y_lo` so closed plane `[y, y]` returns no candidates.

**red_output VERBATIM:**

```
thread 'building_section_index::tests::golden_section_cut_equals_brute_force' (23159) panicked at crates/map-engine-core/src/building_section_index.rs:532:17:
FarmHouse_E_1L01_Wood y=0.45: indexed 0 segs vs brute 316
```

Restored `y_hi < y_lo` (closed `[y, y]` valid), touched with a comment, re-ran: **GREEN** (`golden_section_cut_equals_brute_force` ok).

## gate_verdict_tail

```
  T-437 destroy inert      PASS
  T-586 route tags         PASS
  T-296 reporter identity  PASS
  T-452 player identity    PASS
  no-python (T-620)        PASS

  gate verdict PASS @ 9f4a8c7d1dff recorded: .ai/artifacts/verdicts/T-938.4.json
SLICE GATE: PASS
```

Lock wait ~120s (holders T-937.4 then T-930) then acquired. No `skip:` lines.

## files_outside_owns

- `crates/map-engine-core/src/building_section_tests.rs` — `#[path]` companion of owned `building_section.rs`; required so HeightField accessors and f32 pins compile.

## found_not_fixed

- `HeightField::build` still scans every triangle; only `section_at_owned` uses the y-interval index.
- `packages/map-assets/everon/prefabs/buildings/` materializes one `.bvh` (FarmHouse Wood). Other house shells under `prefabs/blas/` are LFS pointers here. Golden equality used FarmHouse Wood plus five Class-R synthetic meshes from the blueprint suite (room, stairwell, treads, slope, tower).
- `cargo test -p map-engine-core --all-features`: **1036 passed, 1 failed, 2 ignored**. The failure is `dem::peaks::tests::everon_peaks_max_above_350` (`Invalid PNG signature`) because `packages/map-assets/everon/dem/everon-dem-16bit.png` is an LFS pointer. This slice does not own dem.
- Building viewer still walks every cell via `HeightField::at` (frontend, not in owns).

## deviations

- Brief verify named `cargo test -p map-engine-core --all-features`; the DEM LFS failure is environmental, not this diff.
- Tests that raced the shared `CARGO_TARGET_DIR` used `/home/Samuel/.cache/tbd-target-T-938.4`.

## commits

(filled after commit)
