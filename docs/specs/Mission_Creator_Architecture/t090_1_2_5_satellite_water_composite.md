# T-090.1.2.5 — Satellite basemap water (ocean + inland)

**Ticket:** T-090 · **Slice:** T-090.1.2.5  
**Status:** **QUEUED** — inland lakes/rivers missing; ocean reads as grey seabed on SAP ortho  
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

## Related

- Interim ocean tint: `TBD_SatelliteExportPlugin.c` → `oceanBright` / `oceanDark`
- SAP seabed note: `.ai/artifacts/t090_1_2_verify_log.md`
- Region model: **T-090.8** `waterBody`
