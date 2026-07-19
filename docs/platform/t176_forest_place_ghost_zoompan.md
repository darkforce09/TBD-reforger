# T-176 — Forest fidelity + place ghost + zoom+pan stutter

**Status:** SHIPPED @ tag **T-176** / `a5940fad` · **Branch:** `main`  
**Depends on:** T-175 (shipped)  
**Verify:** [`.ai/artifacts/t176_verify_log.md`](../../.ai/artifacts/t176_verify_log.md) · inventory [`.ai/artifacts/t176_inventory.md`](../../.ai/artifacts/t176_inventory.md)  
**Evidence:** [`.ai/artifacts/t176_operator_screens/`](../../.ai/artifacts/t176_operator_screens/)  
**Scope shipped:** `apps/website/frontend/**`, `crates/map-engine-*`, density re-bake via `tools/tbd-tools` (`world redensify` / gen-density-fixture). **Not** `apps/mod/`.

**No silent deferrals.** Soft “later / optional / fold forward” forbidden unless the operator explicitly says `defer X` / `skip X`.

## Shipped outcome

| ID | Result |
|----|--------|
| A1 | `push_landcover` after `set_viewport` (kills one-pass lag); A2 removes stranded wash layer. |
| A2 | **8 m** density re-bake + global canopy blur (`iso=2`); **drop forest-kind landcover wash**. Data: 501,861 trees census-exact; forested chunks 21–59% fill, water 0% — tight canopy, clearings are holes (was ~100% mega-hull wash). |
| B1 | Bind `SlotPlacePreview` to slot atlas in `draw_batches` (ghost visible for palette→map drag). |
| B2 | `CAMERA_GESTURE` flag → defer DEM-contour + 8 m-forest recompute during pan. |

**Forest render model (post T-176):** highlight = **8 m TBDD canopy mass** (`DENSITY_CELL_M=8`, box_blur_corners `r=1`, `CANOPY_MASS_ISO`); loose **32 m Path B landcover forest wash removed**. Retune: `CANOPY_KERNEL_RADIUS_CELLS` / `CANOPY_MASS_ISO` → `cargo run -p tbd-tools --bin world -- redensify --terrain everon` (committed-chunk path, no Workbench).

**Gates:** `make ci-local` PASS · schema-validate (S13 density fixture) · map-engine-core tests · leptos release build. `gate editor-suite` headless lavapipe/WebGPU wedge = pre-existing (t166), not T-176 code.

**Manual visual (operator G-A):** tight forest vs screens 01–03 · place ghost whole palette drag · `window.__editorBench(500)` on zoom+pan.

## Why (pre-ship)

Post T-175: forest highlight wrong until zoom-in→out; light wash far outside trees / painted clearings; palette place ghost invisible; zoom+pan stutter. Contours/names OK.

## Operator matrix (met)

| ID | Bug | Shipped fix |
|----|-----|-------------|
| A1 | Forest stale until zoom ritual | Landcover push order + wash removal |
| A2 | Wash outside canopy / clearings filled | 8 m canopy mass; drop forest landcover wash |
| B1 | Place ghost invisible from palette | SlotPlacePreview ↔ slot atlas bind |
| B2 | Zoom+pan stutter | Defer heavy recompute during `CAMERA_GESTURE` |

## Locked (historical)

1. Fix A1–A2, B1–B2 completely.  
2. Contours / town labels: no regress.  
3. No density-heatmap green glow (T-174).  
4. Prefer live-path / committed redensify over Workbench.  
5. `apps/mod/**` OFF LIMITS.  
