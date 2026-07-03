# T-090.1.2.5 — Satellite basemap water (ocean + inland)

**Ticket:** T-090 · **Slice:** T-090.1.2.5  
**Status:** **READY** — inland lakes/rivers missing; ocean reads as grey seabed on SAP ortho  
**Executor:** claude-code  
**Depends on:** **T-090.1.2.2** (seam-fixed ortho preferred before another full pyramid rebuild) · **T-090.1.2.1** @ `19bc785`  
**Authority:** [`t090_091_map_terrain_program.md`](t090_091_map_terrain_program.md)

---

## In one sentence

Make the **Satellite** basemap show **recognizable water** — **blue ocean** and **inland lakes/rivers** — aligned to Everon world bounds, composited onto the SAP ortho (or an engine-sourced mask) before the lossless tile pyramid rebuild.

---

## Problem (operator-reported)

| Water type | Interim T-090.1 (`ExportRasterization`) | Current SAP (T-090.1.2+) |
|------------|-------------------------------------------|----------------------------|
| **Ocean / coast** | Blue tint via `SetupColors` (oceanBright/oceanDark) | Grey **seabed SAP texture** (authentic ground, not readable as “water”) |
| **Inland water** | **Missing** — lakes/rivers not visible | **Missing** — same SAP cells over lake beds |

Mission makers need to see **where water is** when placing units and reading terrain — not grey ripples or dry lake beds.

**Note:** T-090.1.2 verify log deliberately avoided synthetic blue on SAP seabed (“no fake detail”). This slice is an **explicit product call** to restore **readable hydrology** using **real engine/world data**, not arbitrary paint.

---

## Goal

1. **Ocean:** coast reads as water (blue or game-faithful water color) — improve on grey seabed where appropriate.
2. **Inland:** lakes, ponds, rivers visible at map zoom (Everon has significant inland water).
3. **Alignment:** same `[0,0,12800,12800]` contract; no shift vs slots/DEM.
4. **Rebuild** lossless z0–6 pyramid from composited ortho (same pipeline as T-090.1.2.1).
5. **Document** water mask source in ops log (`map_export_everon.json`).

---

## Investigation (P0 — pick source before compositing)

| Candidate source | Pros | Cons |
|------------------|------|------|
| **A. `MapDataExporter` water / depth mask** | Same family as interim ocean tint; engine knows hydrology | May be 4096²; need upscale + inland channel |
| **B. DEM height ≤ sea level (+ buffer)** | Already have 6400² DEM; fast mask | Rivers may need width; coast line precision |
| **C. Engine water volume / lake entities** (export T-090.3+) | Ground truth | Needs export path; may be sparse |
| **D. `waterBody` regions** (T-090.8) | Polygon accuracy | Blocked until region export exists |
| **E. Separate SAP / material pass in Workbench** | Game-faithful if exists | R&D; see **T-090.1.2.4** |

Spike artifact: `.ai/artifacts/t090_1_2_5_water_source_spike.json` — pick **one primary mask** + optional DEM refine.

**Forbidden:** hand-painted lakes; solid blue rectangle; AI hallucinated rivers.

---

## Implementation sketch (after P0 PASS)

1. Script `scripts/map-assets/composite-water-ortho.mjs` (or extend stitch pipeline):
   - Input: `everon-sap-ortho.png` + water mask raster (aligned, north-up)
   - Apply: ocean + inland classes (may differ color/alpha); preserve land SAP detail outside mask
2. Re-run `verify-sap-ortho` + orientation guard on composited result
3. Rebuild lossless pyramid (`build-tile-pyramid.sh --lossless --maxzoom 6`)
4. Optional: manifest `tiles.satellite.waterComposite: true` + schema field

---

## Manual acceptance

| ID | Pass |
|----|------|
| **W1** | Everon **coast** — ocean reads as water (not ambiguous grey seabed) |
| **W2** | At least **two named inland bodies** visible (operator picks from BI map — e.g. lake NE of airfield, river segments) |
| **W3** | Land SAP detail unchanged outside water mask (no green bleed into water) |
| **W4** | H1/H2 alignment + north-up unchanged |

Log: `.ai/artifacts/t090_1_2_5_verify_log.md`

---

## Out of scope

- Map cartographic view water styling (**T-090.1.1**)
- Interactive waterBody region editor (**T-090.8** / **T-090.5**)
- Wave animation / reflectance
- Arland (Everon first)

---

## Queue position

Run **after T-090.1.2.2** (seams) — each ortho change triggers full pyramid rebuild. May parallel **T-090.1.2.3** (frontend prefetch) since that slice is FE-only.

---

## Ship

Tag **`T-090.1.2.5`** · prefix **`T-090.1.2.5:`**

Handoff: [`.ai/artifacts/t090_1_2_5_claude_code_handoff.md`](../../../.ai/artifacts/t090_1_2_5_claude_code_handoff.md) · send-off [`.ai/artifacts/t090_1_2_5_SEND_TO_CLAUDE.md`](../../../.ai/artifacts/t090_1_2_5_SEND_TO_CLAUDE.md)

Resume: [`t090_1_2_satellite_backlog.md`](t090_1_2_satellite_backlog.md)

---

## Claude Code prompt — T-090.1.2.5 (copy-paste)

Extract: `./scripts/ticket prompt T-090 --slice T-090.1.2.5`

```
Read CLAUDE.md first.

Implement **T-090.1.2.5** — satellite water composite (ocean + inland on SAP ortho).

═══ PREFLIGHT ═══
  git pull && git lfs pull && make map-assets-link
  ./scripts/ticket brief T-090
  test -f packages/map-assets/everon/staging/sap/everon-sap-ortho.png

═══ READ (in order — spec wins on conflict) ═══
  1. .ai/artifacts/t090_1_2_5_claude_code_handoff.md
  2. docs/specs/Mission_Creator_Architecture/t090_1_2_5_satellite_water_composite.md
  3. scripts/map-assets/build-unified-satellite.mjs
  4. packages/map-assets/everon/manifest.json

═══ PROBLEM ═══
  Satellite basemap has no readable water: grey SAP seabed at coast; inland lakes/rivers
  look like dry ground. Product call: composite engine/world-sourced hydrology onto SAP ortho
  before shipping updated map assets. Work on **main** (parallel stream A).

═══ SHIPPED (do not reopen) ═══
  - T-090.1.2.8 @ db9057ef — unified everon-sat.tbd-sat delivery (rebuild bundle after ortho edit)
  - T-090.1.2.2 @ a3efdf6 — seam-fixed SAP ortho source
  - T-090.1.2.1 @ 19bc785 — pyramid encode reference (fallback only)

═══ LOCKED ═══
  - P0 spike must pick mask provenance — no hand-painted lakes or AI rivers
  - Same [0,0,12800,12800] alignment; north-up unchanged
  - Preserve land SAP detail outside water mask
  - Rebuild unified bundle (primary) + optional lossless pyramid fallback
  - No docs/registry edits

═══ DO ═══
  1. P0 — analyze-water-sources.mjs → .ai/artifacts/t090_1_2_5_water_source_spike.json
  2. composite-water-ortho.mjs — SAP ortho + aligned mask → staging PNG
  3. verify-sap-ortho + orientation guard on composited result
  4. build-unified-satellite.mjs from composited ortho → everon-sat.tbd-sat
  5. verify-unified-satellite.mjs + optional EXPECT_LOSSLESS=1 verify-tile-pyramid
  6. Update map_export_everon.json with water mask source
  7. .ai/artifacts/t090_1_2_5_verify_log.md — W1–W4
  8. Tag **T-090.1.2.5** · prefix **T-090.1.2.5:**

═══ DO NOT ═══
  - Edit docs/**, `.ai/tickets/registry.json`, CLAUDE status markers
  - Skip P0 spike — must document mask provenance
  - Solid blue rectangle or arbitrary paint

═══ VERIFY (all exit 0) ═══
  node scripts/map-assets/verify-sap-ortho.mjs TERRAIN=everon
  node scripts/map-assets/verify-unified-satellite.mjs TERRAIN=everon
  EXPECT_LOSSLESS=1 node scripts/map-assets/verify-tile-pyramid.mjs TERRAIN=everon
  make verify-terrain && cd apps/website/frontend && npm run build && npm run lint

═══ MANUAL ═══
  W1: coast reads as water (not grey seabed)
  W2: at least two inland bodies visible
  W3: land SAP unchanged outside mask
  W4: H1/H2 alignment + north-up unchanged

═══ RETURN ═══
  - Commit SHA + tag T-090.1.2.5
  - Spike JSON path + mask source choice
  - Automated verify output (PASS)
  - Manual notes for W1–W4
  - **Ready for Cursor doc sync.**
```

---

## Related

- Interim ocean tint: `TBD_SatelliteExportPlugin.c` → `oceanBright` / `oceanDark`
- SAP seabed note: `.ai/artifacts/t090_1_2_verify_log.md`
- Region model: **T-090.8** `waterBody`
