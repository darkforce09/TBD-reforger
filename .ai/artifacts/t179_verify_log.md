# T-179 — verify log (soft density canopy + MS outlines)

**Branch:** `main` · **Base:** T-178 `58d839c7` · **Commit:** `e7e178c1` · **Executor:** Cursor/Grok (operator override)  
**Hub:** [`docs/platform/t179_forest_canopy_fix.md`](../../docs/platform/t179_forest_canopy_fix.md)

## Result

| Gate | Result |
|------|--------|
| `make leptos-gates` (`gate doctor` + editor smokes) | **PASS** — fullmap `A_bins` / `A_outline_probe` true |
| `cargo test -p map-engine-core` | **100/100** |
| `make ci-local` | **PASS** (exit 0; `CARGO_TARGET_DIR=target-ci` after clearing full `/tmp` sandbox cache) |

## Approach (locked A→B→C)

1. **A — Bin reliability:** fetch 625 TBDD bins with per-bin retry (≤3); arm GPU only when `bins_ok == 625`; bridge `forest_bins_ok`.
2. **B — Soft fill:** pack island as RGBA8 count-in-R (`pack_island_r8_yflip`), **Linear** sampler, corner-centered UVs, `fs_forest_density` = `smoothstep` on `count - CANOPY_MASS_ISO` with `fwidth` AA. Hard Nearest/`textureLoad` path retired. (Plan preferred R16Float; RGBA8×255 kept for WebGL2 gate `force=webgl` compatibility — same Linear AA contract.)
3. **C — Real MS outlines:** one-shot `forest_outline_segments_from_corners` → `upload_hairline_segments` role **6**; `forest_outline_segments` = real count (not fake `0|1`). LOD show/hide via `forest_density_set_params` + `class_visible("forestOutline")`.

## Class-R pins (fullmap smoke)

Measured this checkout:

```json
{
  "pins": {
    "forest_density": 1601,
    "forest_bins": 625,
    "forest_outline_segments_at_z_neg1": 99374,
    "default_zoom": -2.0
  },
  "checks": {
    "A_bins": true,
    "A_density_dims": true,
    "A_density_mode": true,
    "A_forest": true,
    "A_outline_boot": true,
    "A_outline_probe": true,
    "A_lc": true
  },
  "pass": true
}
```

| Bridge key | Boot (z≈−2) | Outline probe (`__editorCamSet(…, -1)`) |
|------------|-------------|-----------------------------------------|
| `forest_mode` | `"density"` | `"density"` |
| `forest_bins_ok` | `625` | `625` |
| `forest_density_w/h` | `1601` | `1601` |
| `forest_polygons` | `625` | `625` |
| `forest_outline_segments` | `0` (LOD off) | **≥ 50000** (measured **99374**) |
| `landcover_polygons` | `0` | `0` |

Smoke floor: `OUTLINE_SEGS_FLOOR = 50_000` in `tools/tbd-tools/src/smokes.rs` (fails stub `segments=1`).

## Explicit not done (legal)

- Full-island MS **fill** mesh (soft Linear fill kept — plan)
- Reintroducing 32 m landcover forest wash
- Progressive per-chunk `push_composite`
- Retuning `CANOPY_MASS_ISO` / redensify
- T-071.1 / `apps/mod/**`

## Manual G-A (operator)

Island canopy smooth (no 8 m stairs), clearings open, no 512 m holes, outline hairlines visible in `z ∈ [-1.5, 0)`.
