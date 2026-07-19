# T-178 — verify log (density-shader forest + YouTube guides)

**Branch:** `main` · **Base:** T-177 `e97a01c6` · **Hygiene:** `61be15a4` · **Commit:**  · **Executor:** Cursor/Grok (operator override — Claude tokens exhausted)

## Result

| Gate | Result |
|------|--------|
| `make leptos-gates` (`gate doctor` + 18 editor smokes) | **PASS** — all `pass: true` |
| `make ci-local` | **PASS** (exit 0) |
| `cargo test -p website-frontend` | **74/74** |
| `cargo test -p map-engine-core density_island` | **4/4** (stitch / pack / Y-flip / dims) |
| fmt + clippy (wasm32 + api) | clean |

## A1 approach (locked)

**Island TBDD density texture + `fs_forest_density`** — not sticky-settle mesh.

1. Fetch all **625** Everon density bins once (`FETCH_CONCURRENCY=12`).
2. Stitch shared-border corners → **1601×1601** u16 tree channel.
3. Y-flip pack to RGBA8 (R=lo, G=hi) for `vs_textured` north-up UVs.
4. `forest_density_upload` → Nearest sampler + `fs_forest_density` (hard `count >= 2.0` = `CANOPY_MASS_ISO`; `fwidth` rim when outline LOD on).
5. Zoom settle: **`forest_density_set_params` only** (no remarch / re-fetch).
6. Role-5 progressive mesh fill **retired** from the live editor path.

## Class-R pins (fullmap smoke)

```json
{
  "pins": {
    "forest_density": 1601,
    "forest_bins": 625,
    "landcover": 0,
    "default_zoom": -2.0
  },
  "checks": {
    "A_density_dims": true,
    "A_density_mode": true,
    "A_forest": true,
    "A_outline_boot": true,
    "A_outline_probe": true,
    "A_lc": true,
    "A_trees_off": true,
    "A_trees_on": true
  },
  "pass": true
}
```

| Bridge key | Boot (z≈−2) | Outline probe (`__editorCamSet(6400,6400,-1)`) |
|------------|-------------|-----------------------------------------------|
| `forest_mode` | `"density"` | `"density"` |
| `forest_density_w/h` | `1601` | `1601` |
| `forest_polygons` | `625` | `625` |
| `forest_outline_segments` | `0` | `1` |

## A2–A4

| ID | Done | Verified |
|----|------|----------|
| **A2** | Dual `"Outliner"` header removed; sole **"Editor Layers"** | `a6_docksMounted` (undo/chrome) |
| **A3** | Continuous YouTube stems (`inset-y-0 w-px` + mid stubs) in outliner + palette | unit + visual |
| **A4** | `data-guide-toggle` click collapses; chevron re-expands | `a4_guideToggle` PASS |

## Harness notes

- Mid-pan zoom in `smoke_pan` uses synthetic `WheelEvent` on canvas (same delivery as `smoke_editor`). CDP `mouseWheel` while RMB is held does not reliably reach the capture listener on the pinned Chrome.
- Density boot is never started mid-`CAMERA_GESTURE` (params-only after upload).

## Explicit not done (legal)

- **T-071.1** ORBAT Manager CRUD
- **`apps/mod/**`**
- Reintroducing 32 m landcover forest wash

## Inventory

Locked equations: [`.ai/artifacts/t178_inventory.md`](t178_inventory.md)
