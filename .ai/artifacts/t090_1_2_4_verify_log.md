# T-090.1.2.4 — Engine render ortho · verify log

**Terrain:** everon · **Date:** 2026-07-01 · **Executor:** claude-code · **Parent:** T-090.1.2.2 apron-bridge @ `a3efdf6`
**Verdict:** **P0 FAIL (honest)** — no engine API can render a continuous grid-free SAT-class 1 m/px north-up ortho. SAP + T-090.1.2.2 bridge stays the production fallback (unchanged). **Do not** revert it.

Spike JSON (full apisTried + blockers + recommendation): [`t090_1_2_4_engine_render_spike.json`](t090_1_2_4_engine_render_spike.json).

---

## Method

Live Workbench MCP (`make mcp-smoke` PASS — wb_connect/wb_state OK; Eden bounds `0..12800`). Exhaustive
`enfusion-mcp api_search` / `asset_search` / `wiki_search` over the 8,693-class index for a
**photographic / terrain-surface-colour top-down** capture (Q1 bar: photographic|surface-color only —
stylized/landcover/cartographic is FAIL). Every query is recorded in the spike JSON `apisTried[]`.

## P0 result — FAIL

| Probe | Finding |
|-------|---------|
| **Per-point terrain colour** (mirror the T-091.0 GetSurfaceY DEM resample) | **ABSENT.** Only `WorldEditorAPI.GetTerrainSurfaceY` (height) + `BaseWorld.GetSurfaceY`. `GetTerrainSurfaceColor`/`GetSurfaceProperties`/`GetTerrainTextureLayers`/`SampleSurface`/`GetSurfaceType`/`SurfaceColor`/`TerrainDiffuse` → no results. No colour analog. |
| **Orthographic camera projection** | **ABSENT.** No `Orthographic`/`SetOrthographic`/`Frustum`; only perspective `CameraBase.SetVerticalFOV`. A 1 m/px north-up **ortho is not renderable**; any capture is a **perspective** viewport. |
| **Scriptable editor camera** | **Unconfirmed.** No `WorldEditorAPI` camera setter surfaced; the lone `SCR_CoordsTool.SetCamera` hit is absent from the class dump. |
| **Pixel capture** | **EXISTS but perspective.** `System.MakeScreenshot(path)`→BMP of the current viewport; `MakeScreenshotRawData`/`MakeScreenshotTexture` → viewport region via callback. All capture the **perspective** viewport at screen res, with distance-LOD + baked lighting/shadows. |
| **Pixel → file write** | **POSSIBLE** — `TexTools.SaveImageData(path,w,h,int[] ABGR)` writes a DDS (decodable by our SAP decoder). The readback-to-file wall does **not** exist. |
| **RenderTargetWidget** (how the in-game 2D map draws) | `SetWorld(BaseWorld, camera)` renders the world offscreen, but exposes **no pixel readback**. The in-game 2D map renders the **SAP texture on a flat quad** + vector overlays — **not** a live 3D terrain render, so capturing it returns the SAP again (circular). |
| **MapDataExporter** | `ExportData`/`SetupColors`/`ExportRasterization` only — all **stylized cartographic**. BI wiki *2D Map Creation* confirms rasterization **is** BI's documented "Satellite Background Image" method (forests + shaded relief) → Q1 **FAIL** tier. |
| **defSatMap_BCR.edds** | Single **global default** sat map (low-res engine fallback), not Everon's real 1 m/px ground. |

**Root conclusion:** Everon's real 1 m/px terrain colour **is** the 2500-cell SAP supertexture (163 M px);
there is **no single continuous terrain-colour texture** to extract, and the ~256 m grid is **baked into
BI's supertexture cell aprons** (proven in T-090.1.2.2). No exposed API produces a cleaner continuous source.
The operator's "Reforger continuous-zoom feel" is **GPU-texture delivery with mips** = **T-090.1.2.8**, not a
different source.

## Q2 escalation — assessed, not buildable as specified

Locked Q2 rung-2 = *runtime ortho camera + RenderTarget pixel dump*. New evidence: runtime cameras are
**perspective** too (no ortho), `RenderTargetWidget` has **no pixel readback**, and the only file-writable
pixel source (`MakeScreenshotRawData` of the viewport) collapses into the **full-game screenshot-stitch that
Q2 explicitly excluded**. `TexTools.SaveImageData` writes pixels but adds neither ortho nor RTT readback.
→ The escalation cannot produce the contract ortho; honest FAIL is reached with the ladder assessed.

## P1 — not run

P1 (production export + `verify-engine-ortho.mjs`) is **gated on P0 PASS** (spec/handoff) → **skipped**.
No `staging/engine/everon-engine-ortho.png` exists; `verify-engine-ortho.mjs` is intentionally **not** built.

## A/B crops

`.ai/artifacts/t090_1_2_4_ab_crops/` (SAP baseline cut; north-up `px=worldX`, `py=12800−worldY`, 1 m/px):

| Crop | World | magick geometry | stddev (detail proxy) |
|------|-------|-----------------|-----------------------|
| `field_sap.png` | (4929, 5661) | `512x512+4673+6883` | 0.068 |
| `roof_sap.png` | House_Mountain_E_1I01 (1232.76, 6240.26) | `512x512+976+6304` | 0.044 |

Engine-side crops are **pending an optional operator run** of `TBD_EngineOrthoExportPlugin.c` (frame a
top-down nadir view over the landmark → run → copy the BMP to `staging/engine/`). The **structural verdict
does not depend on it** — one perspective screenshot cannot become the 12800² north-up ortho.

## Manual acceptance (E1–E3) — what the operator would see

| ID | Expected | Status |
|----|----------|--------|
| **E1** | Engine crop @ (4929,5661) — no 256 m grid line | **N/A (FAIL)** — no engine ortho produced; SAP still shows the inherent soft band |
| **E2** | Detail ≥ SAP on field + roof edge | **N/A (FAIL)** — a perspective viewport screenshot at map altitude streams low-mip ground + leans at edges → expected **sub-SAP** |
| **E3** | Bounds + north-up (`verify-engine-ortho` PASS) | **N/A (FAIL)** — no ortho export; no orthographic projection API to guarantee 1 m/px north-up |

## Files

- **New:** `apps/mod/tbd-framework/Scripts/WorkbenchGame/TBD_EngineOrthoExportPlugin.c` (validated via `mod_validate` — 0 errors on this file; the 20 report errors are pre-existing `EnfusionMCP/**` addon noise), this log, `t090_1_2_4_engine_render_spike.json`, `t090_1_2_4_ab_crops/{field,roof}_sap.png`.
- **Untouched (fallback intact):** SAP stitch/decode scripts, T-090.1.2.2 apron-bridge, tile pyramid, `docs/**`, registry.

## Recommendation

Ship **P0 FAIL**. Pivot 110% effort to **T-090.1.2.8** (unified GPU texture + mips — removes the 5461-tile
pop/flicker = the real "Reforger feel"). The residual soft ~256 m band is inherent to BI's baked supertexture
aprons and is invisible at in-game map zoom. Chasing a new *source* is a dead end on current engine APIs.
