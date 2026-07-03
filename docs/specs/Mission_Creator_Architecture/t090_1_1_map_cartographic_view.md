# T-090.1.1 — Map cartographic view (pyramid + UI switch)

**Ticket:** T-090 · **Slice:** T-090.1.1  
**Status:** **shipped** @ `6e06e679` (tag **T-090.1.1**) — **Next:** **T-090.1.2.3** (prefetch, queued) or **T-090.3** per operator  
**Executor:** claude-code  
**Depends on:** **T-090.1** @ `564419e`, **T-090.1.2.8** @ `db9057ef`, **T-090.1.2.5.2** @ `1c07d97a` (water styling inputs)  
**Cross-cutting UX:** [`t090_basemap_dual_view.md`](t090_basemap_dual_view.md) (N8/N9/N10, V1–V7)  
**Authority:** [`t090_091_map_terrain_program.md`](t090_091_map_terrain_program.md)

---

## In one sentence

Ship the **Map** basemap — stylized cartographic raster under the grid (Google Maps “Map” tab) — as a **complete WebP pyramid** at `tiles/map/`, wire the Mission Settings **Map** radio, and remove the T-127 **`map`→`satellite` coercion** so both views load with the same alignment contract as Satellite.

---

## Product bar

| View | Source today | After this slice |
|------|--------------|------------------|
| **Satellite** | SAP unified bundle + pyramid fallback | **unchanged** |
| **Map** | Manifest stub only (`source: synthesized-cartographic`, no tiles) | **Real pyramid** + enabled radio |

**Not Satellite.** `MapDataExporter.ExportRasterization`, landcover recolor, and height-shaded palette blocks are **correct for Map view** (they were wrongly rejected on the Satellite tab @ T-090.1.2.4). Do **not** merge Map raster into the SAP satellite bundle.

---

## Prerequisites (already shipped)

| Gate | Evidence |
|------|----------|
| Dual manifest slot | `packages/map-assets/everon/manifest.json` → `tiles.map` stub |
| Satellite alignment | H1/H2/H2b @ T-090.1 + `.2.8` unified delivery |
| `.topo` decoder | `scripts/map-assets/decode-topo.mjs` — road/airfield vectors (optional overlay input) |
| Water mask recipe | T-090.1.2.5.x — blue water on **Satellite**; Map view may reuse for stylized water tint |
| Frontend stub | `basemapView.ts`, disabled Map radio in `MissionSettingsDialog.tsx` |
| Spike K4 | [`.ai/artifacts/map_export_everon.json`](../../../.ai/artifacts/map_export_everon.json) — real cartographic path exists; N9 synth **not strictly required** |

---

## P0 — Source spike (mandatory before pyramid build)

Pick **one** primary raster path; log verdict in `.ai/artifacts/t090_1_1_source_spike.json`.

| ID | Candidate | Notes |
|----|-----------|-------|
| **G1-A** | **`MapDataExporter.ExportRasterization`** via existing `TBD_SatelliteExportPlugin.c` plumbing | 4096² stylized map @ T-090.1 — **correct product tier for Map view**. Upscale/warp to 12800² world extent before pyramid. |
| **G1-B** | **`defSatMap_BCR.edds`** / MapEntity “Satellite Background Image” from pak | Engine default stylized background — decode offline like SAP (`decode-edds.mjs`) |
| **G1-C** | **Offline synth (N9)** | DEM hillshade + land-cover LUT + `decode-topo.mjs` road stroke raster + water mask from `.2.5.2` classifier — manifest `tiles.map.source: synthesized-cartographic`, UI label **“Synthetic map”** |
| **G1-D** | Workbench GUI “Export Map Data → Satellite Background Image” one-shot TGA | Human-in-loop OK for first ship; automate in **T-090.3** |

**Honest-stop:** if G1-A/B/D all fail offline, ship **G1-C** — Map must **load**, never stay disabled. Document chosen path in spike JSON + verify log.

**Out of P0:** `.topo` alone is **vector** geometry — useful as synth input, not a direct raster.

---

## Pipeline deliverables

### 1. Staging ortho

Write **`packages/map-assets/everon/staging/map/everon-map-ortho.png`** (12800×12800 or documented upscale from 4096²) — north-up, same world bounds as satellite.

### 2. Pyramid

```bash
bash scripts/map-assets/build-tile-pyramid.sh \
  --input packages/map-assets/everon/staging/map/everon-map-ortho.png \
  --out packages/map-assets/everon/tiles/map \
  --minzoom 0 --maxzoom 6 --tilesize 256
```

Match satellite zoom range (0–6) unless spike proves otherwise. **No `--flip-v`** when source is north-up (same rule as T-090.1 orientation contact sheet).

### 3. One-button rebuild (operator requirement)

Add **`make map-cartographic-everon`** (mirror `map-water-everon` pattern):

```text
spike refresh (optional) → staging ortho → pyramid → manifest source patch → verify
```

Terrain-parameterized via env `TERRAIN=everon` where practical; Arland row may stub `TOPO_TERRAINS` + manifest only.

### 4. Verify script

Extend or add **`verify-tile-pyramid.mjs`** support for **`VIEW=map`** (or sibling script) — complete z0–max grid, manifest `tiles.map.*` agreement, WebP magic. Wire into Makefile target **`map-cartographic-verify`**.

### 5. Manifest

Update `tiles.map`:

```json
{
  "path": "tiles/map",
  "urlTemplate": "/map-assets/everon/tiles/map/{z}/{x}/{y}.webp",
  "source": "workbench-cartographic" | "synthesized-cartographic",
  "encoding": "webp-lossy"
}
```

Do **not** add unified delivery for Map v1 — pyramid LOD only (unified map bundle is optional follow-on).

---

## Frontend deliverables

| File | Change |
|------|--------|
| `state/basemapView.ts` | **Remove** T-127 `map`→`satellite` coercion; honor persisted `'map'` |
| `layout/MissionSettingsDialog.tsx` | Enable **Map** button; remove `(soon)` / `disabled` |
| `layers/useTerrainBasemapLayer.ts` | When `basemapView === 'map'`, resolve `tiles.map.urlTemplate` (pyramid LOD — reuse `computeLod` + `tileUrl`; **no** unified branch) |
| `layers/useBaseMapLayer.ts` | Pass `basemapView` through if not already |
| `TacticalMap.tsx` | Degraded toast when map pyramid 404 (V6) — fall back to grid or satellite per [`t090_basemap_dual_view.md`](t090_basemap_dual_view.md) V6 |

**Layer ids:** prefix `basemap-map-*` (distinct from `basemap-satellite-*`) so Deck caches do not cross-contaminate on switch.

**Perf:** instant swap (no cross-fade); pan/zoom ≥55 fps with pyramid cull (same cap `MAX_VISIBLE_BASEMAP_TILES`).

---

## Out of scope

| Item | Owner |
|------|-------|
| Satellite ortho / SAP / water composite edits | frozen @ `.2.5.2` |
| Unified map bundle (tbd-map-v1) | optional idea |
| Workbench `make map-export` full automation | **T-090.3** |
| T-090.5 road/building vector layers | **T-090.5** |
| Pixel-perfect hydrology on Map tab | **T-143** (`idea`) |
| MC “don’t place units in water” guard | **T-143** |

---

## Verification gates

| ID | Check | Command / method |
|----|-------|------------------|
| **M1** | Map pyramid complete z0–6 | `VIEW=map node scripts/map-assets/verify-tile-pyramid.mjs TERRAIN=everon` (or dedicated script) |
| **M2** | Manifest validates dual tiles | `make schema-validate` |
| **M3** | Landmark alignment Satellite ↔ Map @ (x,y) ≤ **50 m** | Manual H2 contact sheet (reuse T-090.1 anchors) |
| **M4** | Y-axis H2b on **map** pyramid | Manual / orientation crop |
| **M5** | `basemapView` persists `'map'` across reload | localStorage manual |
| **M6** | Map 404 → degraded path (toast + fallback) | Manual — delete one tile temporarily |
| **M7** | FE build + lint | `cd apps/website/frontend && npm run build && npm run lint` |
| **M8** | `make verify-terrain` | maxDeltaM unchanged |
| **M9** | Pan ≥55 fps on Map view | FpsCounter manual @ default zoom |

Log → **`.ai/artifacts/t090_1_1_verify_log.md`** with spike verdict + M1–M9 table.

---

## Ship contract

| Item | Value |
|------|-------|
| Commit prefix | **`T-090.1.1:`** |
| Tag | **`T-090.1.1`** |
| Post-ship | Cursor doc sync → **T-090.1.1.1** (land-cover) active · **T-090.1.2.9** (satellite roads) queued |

---

## Key files (touch list)

| Path | Role |
|------|------|
| `scripts/map-assets/build-map-cartographic.mjs` | **Create** — ortho build from spike winner |
| `scripts/map-assets/verify-tile-pyramid.mjs` | Extend for `VIEW=map` |
| `Makefile` | `map-cartographic-everon`, verify target |
| `packages/map-assets/everon/tiles/map/**` | LFS pyramid output |
| `packages/map-assets/everon/manifest.json` | `tiles.map.source` + encoding |
| `apps/website/frontend/src/features/tactical-map/**` | basemap switch |
| `.ai/artifacts/t090_1_1_source_spike.json` | P0 verdict |
| `.ai/artifacts/t090_1_1_verify_log.md` | M1–M9 |

---

## Related

- [`t090_basemap_dual_view.md`](t090_basemap_dual_view.md) — N8/N9/N10, V1–V7  
- [`t090_1_aligned_basemap.md`](t090_1_aligned_basemap.md) — Y-flip + Cartesian contract  
- [`t090_terrain_export_pipeline.md`](t090_terrain_export_pipeline.md) — future `make map-export`  
- Handoff: [`.ai/artifacts/t090_1_1_claude_code_handoff.md`](../../../.ai/artifacts/t090_1_1_claude_code_handoff.md)
