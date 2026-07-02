# T-090.1.2.4 — Engine render ortho (110% satellite source)

**Ticket:** T-090 · **Slice:** T-090.1.2.4  
**Status:** **SHIPPED** @ `0d6fe485` — **P0 FAIL (honest)** — no engine API for grid-free sat-class ortho; SAP + `.2.2` bridge remains production source  
**Git tag on ship:** **T-090.1.2.4**  
**Executor:** claude-code (Workbench mod) + operator time  
**Depends on:** **T-090.1.2.2** shipped @ `a3efdf6` (apron-bridge — ticket goal met; 110% needs new source)  
**Blocks 110%:** **T-090.1.2.8** unified texture delivery · **T-090.1.2.5** water (prefer engine ortho base)

**Authority:** [`t090_091_map_terrain_program.md`](t090_091_map_terrain_program.md) · post-mortem [`.ai/artifacts/t090_1_2_2_verify_log.md`](../../../.ai/artifacts/t090_1_2_2_verify_log.md)

---

## In one sentence

Replace the **2500-cell SAP stitch** with a **single continuous engine-rendered terrain-color ortho** @ ≥12800² — same alignment contract as T-090.1 — so the Satellite basemap has **no 256 m grid** in the source image.

---

## Why this slice is active now

### T-090.1.2.2 — ticket complete, 110% incomplete

| Layer | Verdict |
|-------|---------|
| **Ticket goal** (measure + repair stitch + rebuild pyramid) | **Shipped** @ `a3efdf6` — automation PASS |
| **Operator 110% bar** (no visible periodic grid @ max zoom) | **Not met** — soft smear grid on **98/98** interior seams |

**Proven on shipped ortho** (cursor @ world 4929, 5661 — operator screenshot):

- SAP seam band gradient **~0.4** vs field interior **~3.1** (8 m bridge band)
- Pyramid tile cuts @ 200 m show **~3.2** — **not** the visible artifact
- Root cause: **baked ~3–4 px apron per SAP cell** + BC7; compositing cannot invent missing detail

**Do not revert T-090.1.2.2.** SAP + bridge remains production source — **T-090.1.2.8** owns delivery pivot (see post-ship).

---

## Post-ship (@ `0d6fe485`)

| Result | Detail |
|--------|--------|
| **P0** | **FAIL** — exhaustive Workbench MCP api_search; no orthographic projection, no per-point terrain colour, no RTT readback |
| **P1** | Skipped (gated on P0 PASS) |
| **Artifacts** | [`.ai/artifacts/t090_1_2_4_engine_render_spike.json`](../../../.ai/artifacts/t090_1_2_4_engine_render_spike.json) · [verify log](../../../.ai/artifacts/t090_1_2_4_verify_log.md) |
| **Plugin** | `TBD_EngineOrthoExportPlugin.c` (SPIKE viewport screenshot — optional operator run) |
| **Next** | **T-090.1.2.8** unified GPU texture + mips (Reforger zoom feel) |

**Do not revert T-090.1.2.2.** Chasing a new source is a dead end on current APIs.

### Why not more SAP compositing

Apron-bridge (strategy A) removes flat strips but leaves a **detectable soft band every 256 m** on 100% of evaluated seams. z7+ pyramid upsamples the same band. See hub §Satellite 110% path.

---

## Goal

### P0 — Feasibility spike (gate — do not skip)

| Question | PASS criterion |
|----------|----------------|
| Can Workbench render **terrain surface color** top-down to bitmap? | ≥1 PNG/TGA crop @ known world coords |
| Max practical resolution? | Document time + RAM; target **12800²** RGB |
| **No 256 m grid** in crop? | Gradient sweep shows no periodic 256 px line |
| Beats SAP on same patch? | A/B vs SAP ortho @ field + roof (operator eyeball + metric) |
| Aligns `[0,0,12800,12800]` north-up? | `verify-sap-ortho`-style orientation guard on export |

**FAIL →** document blockers in `t090_1_2_4_engine_render_spike.json`; stay on SAP; do not fake PASS.

### P1 — Production export (only if P0 PASS)

- Workbench plugin (extend or sibling to `TBD_SatelliteExportPlugin.c`):
  - Export path → `packages/map-assets/everon/staging/engine/everon-engine-ortho.png` (gitignored)
  - `TBD_SatExport_meta.json` fields: `source: engine-render-ortho`, `captureMethodId` TBD
- **Do not** run `build-tile-pyramid.sh` in this slice unless spec **T-090.1.2.8** is still queued — pyramid is interim; 110% delivery is unified texture (see `.2.8`).
- Optional: commit `full.webp` only for dev fallback until `.2.8` ships.

---

## Locked decisions

| Decision | Choice |
|----------|--------|
| **Product bar** | **Satellite / aerial imagery only** — photographic ground texture (roofs, fields, forest canopy). **Not** the stylized cartographic “game map”. |
| **Primary win** | **No cell grid in source** — continuous sat-class RGB, not “sharper BC7” alone |
| **SAT vs NOT-SAT** | **SAT:** SAP supertexture class, engine terrain **surface-color** ortho, runtime sat capture. **NOT SAT:** `MapDataExporter`, landcover-type recolor, height-shaded palette blocks — those belong on **T-090.1.1 Map view**, never Satellite |
| **P0 / P1 PASS** | Grid-free **and** sat-class imagery. Grid-free stylized = **FAIL** (wrong product) |
| **Escalation before FAIL** | Workbench sat APIs → runtime RenderTarget sat dump. **No** screenshot-stitch in this slice |
| **SAP pipeline** | Keep decode/stitch scripts; **production fallback** until sat source ships |
| **T-090.1.2.2 bridge** | Keep in stitch path for SAP fallback |
| **Pyramid rebuild** | **Out of this slice** — **T-090.1.2.8** owns delivery after ortho exists |
| **MapDataExporter.ExportRasterization** | **NOT SAT** — plumbing reference only |
| **AI upscale / z7** | Forbidden |
| **Executor** | claude-code on `apps/mod/tbd-framework` + export scripts |

---

## Investigation starting points

| Asset | Path |
|-------|------|
| Existing raster export | `apps/mod/tbd-framework/Scripts/WorkbenchGame/TBD_SatelliteExportPlugin.c` |
| SAP stitch (fallback) | `scripts/map-assets/stitch-sap-ortho.mjs` |
| Alignment gates | `scripts/map-assets/verify-sap-ortho.mjs` |
| Workbench MCP | `docs/mod/MCP_TOOLING.md` · `make mcp-smoke` |

**API search (P0):** Enfusion Workbench — off-screen camera, terrain material color, orthographic top-down, `MapDataExporter` alternatives, render target size limits.

---

## Artifacts

| File | Purpose |
|------|---------|
| `.ai/artifacts/t090_1_2_4_engine_render_spike.json` | P0 PASS/FAIL + API notes + timings |
| `.ai/artifacts/t090_1_2_4_ab_crops/` | SAP vs engine PNG crops (same world rects) |
| `.ai/artifacts/t090_1_2_4_verify_log.md` | Ship log |

---

## Manual acceptance (P0)

| ID | Pass |
|----|------|
| **E1** | Engine crop @ operator field (≈4929, 5661) — **no visible 256 m line** |
| **E2** | Detail ≥ SAP on same crop (roof edge / field texture) |
| **E3** | Orientation + bounds match manifest |

---

## Out of scope

- Unified GPU/binary delivery — **T-090.1.2.8**
- Water composite — **T-090.1.2.5** (after engine ortho)
- Tile prefetch — **T-090.1.2.3** (interim pyramid band-aid; superseded by `.2.8` for 110%)
- Map cartographic view — **T-090.1.1**

---

## Documentation sync (Cursor — after ship)

Registry: `T-090.1.2.4` shipped_at · `active_slice` → **T-090.1.2.8** (or `.2.8` parallel if P1 ortho only).

---

## Claude Code prompt — T-090.1.2.4 (copy-paste)

Authority: this spec + handoff. **Do not edit docs/registry.**  
Extract: `./scripts/ticket prompt T-090` · standard: [`.ai/tickets/CLAUDE_CODE_PROMPT.md`](../../../.ai/tickets/CLAUDE_CODE_PROMPT.md)

```
Read CLAUDE.md first.

Implement **T-090.1.2.4** — engine render ortho: continuous terrain-color capture (no 2500-cell SAP grid).

═══ PREFLIGHT ═══
  git pull && git lfs pull && make map-assets-link
  ./scripts/ticket brief T-090
  export ENFUSION_GAME_PATH="${ENFUSION_GAME_PATH:-$HOME/.cache/enfusion-mcp-root}"
  make mcp-smoke
  command -v magick
  test -f packages/map-assets/everon/staging/sap/everon-sap-ortho.png \
    || node scripts/map-assets/stitch-sap-ortho.mjs TERRAIN=everon

═══ READ (in order — spec wins on conflict) ═══
  1. .ai/artifacts/t090_1_2_4_claude_code_handoff.md
  2. docs/specs/Mission_Creator_Architecture/t090_1_2_4_engine_render_ortho_spike.md
  3. apps/mod/tbd-framework/Scripts/WorkbenchGame/TBD_SatelliteExportPlugin.c  (plumbing ONLY)
  4. .ai/artifacts/t090_1_2_2_verify_log.md  (why SAP compositing cannot reach 110%)
  5. scripts/map-assets/verify-sap-ortho.mjs  (orientation/bounds guard pattern)
  6. scripts/map-assets/analyze-sap-seams.mjs + lib/sap-seam-metrics.mjs  (256 m grid metric)

═══ PROBLEM ═══
  Mission Creator Satellite still shows a visible ~256 m grid at max zoom. Root cause: 2500 SAP
  supertexture cells pasted edge-to-edge — each cell has a baked 3–4 px flat apron. T-090.1.2.2
  apron-bridge @ a3efdf6 shipped (ticket goal met) but leaves a soft smear on 98/98 seams (~14%
  band/interior ratio) — compositing cannot fix this. Operator landmark: world (4929, 5661).
  Need ONE continuous engine-rendered terrain-color ortho — like Reforger's in-game map texture,
  not a web tile pyramid (delivery is T-090.1.2.8 — out of scope here).

═══ SHIPPED (do not reopen) ═══
  - T-090.1.2.2 @ a3efdf6 — SAP apron-bridge; keep as fallback
  - T-090.1.2.1 @ 19bc785 — lossless pyramid encode (interim delivery)
  - T-090.1.2 @ c2730a3 — SAP decode/stitch pipeline

═══ LOCKED ═══
  - **SATELLITE ONLY** — aerial/photographic ground texture (SAP class). NOT the stylized cartographic map.
  - MapDataExporter, landcover recolor, height-shaded palette = NOT SAT → FAIL even if grid-free
  - NOT SAT belongs on T-090.1.1 Map view — never ship it on Satellite tab
  - P0 PASS = grid-free + sat-class imagery; honest FAIL if only NOT-SAT paths exist
  - Escalation before FAIL: Workbench sat APIs → runtime RenderTarget sat dump (no screenshot-stitch)
  - Contract: 12800×12800 m, worldBounds [0,0,12800,12800], metersPerPixel 1, north-up
  - No build-tile-pyramid.sh — T-090.1.2.8 owns unified binary/GPU delivery
  - No z7, AI upscale, grey fill, SAP script deletion, docs/registry edits
  - Full locked table: spec §Locked decisions

═══ DO ═══
  1. P0 — Load Eden in Workbench; API search (MCP, Enfusion, mod grep) for continuous terrain-color
     top-down capture — document every candidate in spike JSON
  2. P0 — Export test bitmap via best **SAT** candidate only; MapDataExporter / landcover recolor = NOT SAT
  3. P0 — If Workbench sat paths fail, escalate runtime RenderTarget sat dump before FAIL
  4. P0 — Grid test: no periodic flat/smear at x=256·k, y=256·k (same spirit as analyze-sap-seams)
  5. P0 — A/B 512² crops → .ai/artifacts/t090_1_2_4_ab_crops/
       field @ world (4929,5661) sap vs engine; roof/edge pair (document coords)
  6. P0 — Write .ai/artifacts/t090_1_2_4_engine_render_spike.json (schema in handoff; qualityTier must be photographic|surface-color)
  7. IF P0 PASS (SAT + grid-free) — new plugin TBD_EngineOrthoExportPlugin.c (or extend sibling): export ≥12800² PNG
     → packages/map-assets/everon/staging/engine/everon-engine-ortho.png + TBD_EngineOrtho_meta.json
  8. IF P0 PASS — scripts/map-assets/verify-engine-ortho.mjs (12800², stddev, orientation vs DEM,
     no 256 m grid); run PASS
  9. .ai/artifacts/t090_1_2_4_verify_log.md — method, dimensions, P0/P1, E1–E3, A/B notes
  10. Tag **T-090.1.2.4** · prefix **T-090.1.2.4:**  (SAT PASS or honest FAIL — NOT-SAT is FAIL)

═══ DO NOT ═══
  - Edit docs/**, `.ai/tickets/registry.json`, docs/TICKET_*.md, CLAUDE status markers
  - Ship NOT-SAT (MapDataExporter, landcover recolor) on Satellite — wrong tab; use T-090.1.1 Map view
  - Treat grid-free stylized output as P0 PASS
  - build-tile-pyramid.sh / LFS pyramid rebuild
  - Revert T-090.1.2.2 bridge or delete SAP stitch/decode scripts
  - Fake P0 PASS — if no terrain-color API exists, ship FAIL + blockers

═══ VERIFY (run what applies — all exit 0) ═══
  # P0 minimum (always):
  test -f .ai/artifacts/t090_1_2_4_engine_render_spike.json
  test -d .ai/artifacts/t090_1_2_4_ab_crops

  # P1 (only if P0 PASS):
  node scripts/map-assets/verify-engine-ortho.mjs TERRAIN=everon
  magick identify packages/map-assets/everon/staging/engine/everon-engine-ortho.png

═══ MANUAL ═══
  E1: crop @ (4929,5661) — no visible 256 m grid line on engine capture
  E2: same crop — detail ≥ SAP on field + roof edge (note in verify log)
  E3: bounds + north-up — verify-engine-ortho PASS (or manual DEM land-mask check @ P0)

═══ RETURN ═══
  - Commit SHA + tag T-090.1.2.4
  - captureMethodId + P0 PASS/FAIL + spike JSON path
  - apisTried summary (what worked / what didn't)
  - A/B crop paths + landmark bandMinGrad if measured
  - If P1: export path, meta path, verify-engine-ortho output
  - E1–E3 manual notes
  - **Ready for Cursor doc sync.**
```

---

## Related

- Handoff: [`.ai/artifacts/t090_1_2_4_claude_code_handoff.md`](../../../.ai/artifacts/t090_1_2_4_claude_code_handoff.md)
- Delivery (next): [`t090_1_2_8_unified_satellite_texture.md`](t090_1_2_8_unified_satellite_texture.md)
- SAP fallback: [`t090_1_2_sap_supertexture_satellite.md`](t090_1_2_sap_supertexture_satellite.md)
