# T-090.1.2.4 — Claude Code handoff (engine render ortho)

**Slice:** T-090.1.2.4 · **Executor:** claude-code · **Branch:** `ticket/T-090`  
**Parent shipped:** T-090.1.2.2 @ `a3efdf6` (SAP apron-bridge — ticket goal met; 110% not met)  
**Spec (authority):** [`docs/specs/Mission_Creator_Architecture/t090_1_2_4_engine_render_ortho_spike.md`](../docs/specs/Mission_Creator_Architecture/t090_1_2_4_engine_render_ortho_spike.md)

---

## Operator report

- Satellite @ max zoom still shows a **periodic ~256 m grid** everywhere — soft smear bands, not invisible.
- Pan/zoom **flickers** — 5461 WebP tiles (separate slice **T-090.1.2.8**).
- Wants **Arma Reforger feel**: one continuous texture, smooth zoom — not a web tile pyramid.
- **Satellite view only** — aerial/photographic ground texture. **Not** the stylized green/brown cartographic map (`MapDataExporter`, landcover recolor). Map view is **T-090.1.1** — different product.
- **Screenshot landmark:** world **X ≈ 4929, Y ≈ 5661** (field patch — worst visible seam band on SAP ortho).

---

## What you are building

```text
Workbench (Eden loaded)
  → P0: find + prove API for continuous terrain-color top-down capture (NOT MapDataExporter stylized raster)
  → A/B crop vs SAP @ landmark + roof edge
  → P1 (if P0 PASS): plugin exports ≥12800² PNG → staging/engine/
  → verify orientation/bounds + no 256 m periodic grid
  → spike JSON + verify log + tag T-090.1.2.4

NO pyramid rebuild — T-090.1.2.8 owns delivery.
```

---

## Why SAP compositing failed 110% (read-only — do not reopen)

From [`.ai/artifacts/t090_1_2_2_verify_log.md`](t090_1_2_2_verify_log.md):

- Each Eden `_supertexture.edds` cell has a **baked 3–4 px flat apron** on mip0 edges.
- Edge-to-edge paste → **~8 px dead band every 256 m**; apron-bridge replaces flat strip with **linear smear**.
- **98/98** interior seams still show band/interior gradient ratio **~14%** — visible planet-wide.
- Pyramid tile cuts @ 200 m are **not** the artifact — the **source ortho** is.

**This slice:** bypass the 2500-cell paste entirely — render **one continuous** terrain-color image from the engine.

---

## SAT vs NOT-SAT (operator product line — non-negotiable)

| | **SAT (this slice)** | **NOT SAT (reject / wrong tab)** |
|---|----------------------|----------------------------------|
| **Looks like** | Aerial photo — roof edges, field texture, forest mottling | Flat green/brown blocks, palette landcover, height-shaded “game map” |
| **Examples** | SAP supertexture (despite seams), engine **surface-color** ortho | `MapDataExporter.ExportRasterization`, per-point landcover recolor |
| **Ship?** | Yes (if grid-free) | **Never** on Satellite — park for **T-090.1.1 Map view** |

`TBD_SatelliteExportPlugin.c` / MapDataExporter = **NOT SAT**. Useful only for Workbench plumbing (`winRc`, Proton paths). **Do not** call P0 PASS if the best capture is cartographic/stylized even when grid-free.

**P0 must search for something closer to in-game terrain surface color / satellite photography** — off-screen ortho camera, render target, terrain material sampling, or an undocumented export API. Document every API tried in spike JSON.

---

## Do not

| Forbidden | Why |
|-----------|-----|
| Revert / weaken T-090.1.2.2 SAP bridge | Fallback until engine ortho ships |
| `build-tile-pyramid.sh` in this slice | **T-090.1.2.8** owns delivery |
| z7 pyramid, AI upscale, grey fill | Fake detail / policy |
| Edit `docs/**`, registry, `TICKET_*.md` | Cursor doc sync after merge |
| Delete SAP stitch/decode scripts | Fallback pipeline |

---

## Execution order (strict)

### Phase P0 — Feasibility spike (STOP if FAIL)

1. Load Eden in Workbench; confirm bounds `[0,0]`–`[12800,12800]`, anchor Y plausible (`GetTerrainSurfaceY(4839,6620)` land, `(6400,6400)` peak 50–400 m).
2. **API search** (Enfusion docs, Workbench MCP, grep mod/framework, BI wiki): list candidates for **photographic / surface-color** top-down capture. Try the most promising first.
3. Export **at least one test bitmap** — full frame or large crop — using the best candidate API.
4. **Grid test:** on the export, sweep lines at **x = 256·k** and **y = 256·k** for k = 1…49. PASS = **no periodic flat/smear band** matching SAP (gradient at seam lines within **2×** of adjacent interior lines — same test spirit as `analyze-sap-seams.mjs`).
5. **A/B crops** (512² PNG) saved to `.ai/artifacts/t090_1_2_4_ab_crops/`:
   - `field_sap.png` / `field_engine.png` — world center **(4929, 5661)**
   - `roof_sap.png` / `roof_engine.png` — pick a high-contrast roof/building edge (document world coords)
6. Write `.ai/artifacts/t090_1_2_4_engine_render_spike.json`:

```json
{
  "p0": "PASS | FAIL",
  "captureMethodId": "string — e.g. offscreen-camera-terrain-color",
  "apisTried": [{ "name": "", "result": "", "rc": 0 }],
  "maxDimension": { "width": 0, "height": 0, "limitReason": "" },
  "gridPeriodic256m": true,
  "landmarkField": { "worldX": 4929, "worldY": 5661, "bandMinGradEngine": 0, "bandMinGradSap": 0.12 },
  "orientationNote": "",
  "blockers": []
}
```

**Escalation before FAIL:** Workbench sat APIs exhausted → try **runtime RenderTarget** sat dump in `tbd-framework`. No screenshot-stitch in this slice.

**P0 FAIL →** commit spike JSON + verify log explaining blockers; tag **`T-090.1.2.4`** anyway (honest FAIL ship); SAP stays fallback. **Do not** ship NOT-SAT as Satellite. **Do not** fake PASS.

### Phase P1 — Production export (only if P0 PASS)

1. New or extended Workbench plugin under `apps/mod/tbd-framework/Scripts/WorkbenchGame/` (sibling naming OK: `TBD_EngineOrthoExportPlugin.c`).
2. Export **≥12800×12800** RGB PNG to:
   - `packages/map-assets/everon/staging/engine/everon-engine-ortho.png` (gitignored)
   - Meta: `packages/map-assets/everon/staging/engine/TBD_EngineOrtho_meta.json`
3. Meta **required fields:** `source: "engine-render-ortho"`, `captureMethodId`, `worldBounds: [0,0,12800,12800]`, `metersPerPixel: 1`, `width`, `height`, `exportedAt`.
4. Add `scripts/map-assets/verify-engine-ortho.mjs` — adapt checks from `verify-sap-ortho.mjs`:
   - 12800², stddev > 0.02, orientation vs DEM land-mask, **no 256 m periodic grid** (reuse or import seam metric helpers from `lib/sap-seam-metrics.mjs` if useful).
5. `.ai/artifacts/t090_1_2_4_verify_log.md` — P0/P1, method, dimensions, E1–E3, A/B notes.

---

## Preflight

```bash
git pull && git lfs pull && make map-assets-link
./scripts/ticket brief T-090
export ENFUSION_GAME_PATH="${ENFUSION_GAME_PATH:-$HOME/.cache/enfusion-mcp-root}"
make mcp-smoke   # Workbench MCP — see docs/mod/MCP_TOOLING.md
command -v magick
test -f packages/map-assets/everon/staging/sap/everon-sap-ortho.png \
  && magick identify packages/map-assets/everon/staging/sap/everon-sap-ortho.png
```

SAP ortho required for A/B — re-stitch only if missing: `node scripts/map-assets/stitch-sap-ortho.mjs TERRAIN=everon`

---

## Key files

| File | Role |
|------|------|
| `apps/mod/tbd-framework/Scripts/WorkbenchGame/TBD_SatelliteExportPlugin.c` | **Reference only** — MapDataExporter plumbing, NOT quality target |
| `scripts/map-assets/verify-sap-ortho.mjs` | Orientation + bounds guard pattern |
| `scripts/map-assets/analyze-sap-seams.mjs` | Grid metric reference |
| `scripts/map-assets/lib/sap-seam-metrics.mjs` | Reusable gradient/band helpers |
| `packages/map-assets/everon/manifest.json` | Contract: `worldBounds [0,0,12800,12800]`, `metersPerPixel: 1` |
| `packages/map-assets/everon/staging/sap/everon-sap-ortho.png` | SAP A/B baseline |
| `.ai/artifacts/t090_1_2_2_verify_log.md` | Why compositing failed 110% |

---

## Coordinate contract

| System | Rule |
|--------|------|
| World | Everon 12800×12800 m, origin bottom-left, +Y north |
| Ortho pixel | `(px, py)` ↔ world `(px, py)` at 1 m/px when `metersPerPixel: 1` |
| Orientation | North-up in editor — match DEM land-mask (see `verify-sap-ortho.mjs` ORIENT_MAX 0.2) |
| Export flip | Document if engine writes upside-down TGA; apply **one** V-flip in post if needed |

---

## Manual acceptance

| ID | Pass |
|----|------|
| **E1** | Engine crop @ **(4929, 5661)** — operator cannot see a 256 m grid line |
| **E2** | Same crop — detail **≥ SAP** on field texture + roof edge (subjective; note in verify log) |
| **E3** | Bounds + north-up — automated verify PASS |

---

## Verify commands (automated — run what applies)

```bash
# After P1 export exists:
node scripts/map-assets/verify-engine-ortho.mjs TERRAIN=everon

# Mod compile sanity (if plugin changed):
# Workbench build or project-specific mod verify per docs/mod/

# Frontend untouched expected — but if touched:
make ci-local-frontend
```

---

## Return contract

- Commit SHA + tag **`T-090.1.2.4`** · prefix **`T-090.1.2.4:`**
- `.ai/artifacts/t090_1_2_4_engine_render_spike.json` (always)
- `.ai/artifacts/t090_1_2_4_verify_log.md`
- A/B crops in `.ai/artifacts/t090_1_2_4_ab_crops/` (P0 minimum)
- If P1: export path + meta path + verify-engine-ortho PASS output
- E1–E3 manual notes (even if FAIL — document what operator would see)
- **Ready for Cursor doc sync.**
