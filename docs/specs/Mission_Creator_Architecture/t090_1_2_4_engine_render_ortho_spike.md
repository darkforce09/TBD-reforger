# T-090.1.2.4 — Engine render ortho export (future spike)

**Ticket:** T-090 · **Slice:** T-090.1.2.4  
**Status:** **IDEA** — deferred; do not implement until T-090.1.2.2 + T-090.1.2.3 ship  
**Executor:** claude-code (Workbench mod) + operator time  
**Depends on:** **T-090.1.2.1** shipped @ `19bc785` (SAP + lossless pyramid = current production source)

---

## In one sentence

**Research spike:** can a custom Workbench plugin **render terrain surface color** from the live Eden world at full world resolution — bypassing BC7 `.edds` decode — and beat (or replace) the SAP supertexture stitch for Satellite detail?

---

## Why this exists (not active work)

Operator asked whether a **“raw export”** could beat BC7 blockiness. Answer today:

| Path | Verdict |
|------|---------|
| SAP decode → PNG → lossless VP8L | **Production** — best honest detail from shipped game files |
| `MapDataExporter.ExportRasterization` | Uncompressed TGA but **4096² stylized** — worse, not better |
| **z7+ pyramid / AI upscale** | Fake detail — rejected |
| **Engine render ortho (this ticket)** | **Unknown** — worth a tool if APIs exist; **future future** |

**Product call (2026-07):** resolution is **good enough for now**. Fix **seams (T-090.1.2.2)** and **pan UX (T-090.1.2.3)** first. This slice stays **`idea`** until those ship and someone promotes it.

---

## Hypothesis

The live renderer may sample terrain materials at higher effective precision than BC7-compressed supertexture cells stored in `.pak`. A plugin that **off-screen renders** the world top-down (or exports a tile grid) could produce:

- Uncompressed RGBA ortho @ ≥12800² (or higher if API allows)
- Same alignment contract as T-090.1 (`worldBounds`, north-up, manifest)
- Optional side benefit: **reusable export tool** for Arland / future terrains without `.edds` decode pipeline

**Risk:** No public “render ortho at N×N” API documented; may be infeasible or no better than SAP.

---

## Spike scope (when promoted to `ready`)

### P0 — Feasibility (PASS/FAIL gate)

| Question | Method |
|----------|--------|
| Can Workbench render world color to bitmap off-screen? | MCP `api_search` + small plugin experiment |
| Max practical resolution? | Time + memory on operator machine |
| Beats BC7 SAP on same patch? | A/B crop @ 1 m scale (field edge, roof line) |
| Aligns to `[0,0,12800,12800]`? | Same anchor probes as T-090.1 |

**FAIL →** close ticket `cancelled` or leave `idea`; keep SAP pipeline.

**PASS →** new plugin `TBD_TerrainColorRenderExportPlugin.c` (name TBD), staging ortho path, optional replace `sap-supertexture-stitch` in manifest `source` enum (`engine-render-ortho`).

### Out of spike

- Replacing T-090.1.2 decode/stitch without A/B proof
- Map cartographic (`.topo`) — **T-090.1.1**
- Production pyramid rebuild unless spike beats SAP on operator sign-off

---

## Artifacts (when run)

- `.ai/artifacts/t090_1_2_4_engine_render_spike.json`
- Sample PNG crop comparison vs SAP ortho
- Plugin source under `apps/mod/tbd-framework/Scripts/WorkbenchGame/`

---

## Related

- Current satellite: [`t090_1_2_sap_supertexture_satellite.md`](t090_1_2_sap_supertexture_satellite.md)
- Interim rasterizer (lower res, stylized): `TBD_SatelliteExportPlugin.c` → `ExportRasterization`
- Active fixes: **T-090.1.2.2** seams · **T-090.1.2.3** prefetch
