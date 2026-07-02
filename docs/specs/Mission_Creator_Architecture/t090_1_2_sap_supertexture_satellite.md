# T-090.1.2 — SAP supertexture satellite ortho (high detail)

**Ticket:** T-090 · **Slice:** T-090.1.2  
**Status:** **SHIPPED** — SAP supertexture ortho replaces interim rasterization tiles (follow-up: lossless pyramid @ z6)  
**Executor:** claude-code (+ operator time for decode/spike if needed)  
**Depends on:** **T-090.1** shipped @ `564419e` (Cartesian loader, pyramid, alignment proven)  
**Authority:** [`t090_091_map_terrain_program.md`](t090_091_map_terrain_program.md) · [`t090_1_aligned_basemap.md`](t090_1_aligned_basemap.md) · ops log [`.ai/artifacts/map_export_everon.json`](../../../.ai/artifacts/map_export_everon.json)

---

## In one sentence

Decode and stitch Everon **`Eden_*_supertexture.edds`** cells into a world-aligned, north-up ortho, rebuild the **Satellite** WebP pyramid, and swap it in — so zoomed-in Mission Creator shows **real ground texture detail**, not the interim `MapDataExporter.ExportRasterization` shaded-relief map.

---

## What T-090.1 already proved (do not redo)

| Done @ `564419e` | Keep |
|------------------|------|
| Cartesian basemap + grid layer order | ✓ |
| Pyramid LOD + `tileUrl` Y-flip + unit tests | ✓ |
| Manifest `tiles.satellite` contract | ✓ |
| K3 file gate + verify scripts | ✓ |
| H1/H2/H2b alignment (world `[0,0,12800,12800]`) | ✓ operator-confirmed |
| MCP + `build-tile-pyramid.sh` + LFS WebP | ✓ |

**This slice only replaces the raster source + regenerated tiles.** Frontend changes only if probe paths, manifest meta, or LOD constants need updating.

---

## What this slice is NOT

| Goal | Ticket |
|------|--------|
| **Roads / labels / cartographic “Map” view** | **T-090.1.1** (`.topo` / Export Map Data) |
| **Building/slot glyphs on the map** | **T-090.5** (+ T-090.3 export) |
| **Upscale / fake detail** | Forbidden — must be decoded real pixels |

---

## Problem

`MapDataExporter.ExportRasterization` @ 4096² produces a **stylized shaded-relief** map (land tint + hillshade + area colors). Pyramid LOD cannot add photographic detail — zoom magnifies smooth gradients. The true in-game ground appearance lives in **SAP super-textures**:

`worlds/Eden/Eden/.Data/Eden_*_supertexture.edds` (hundreds of cells; `.edds` inside `.pak` — spike noted 5266 super-textures repo-wide).

---

## Goal (110% bar)

1. **Decode** Enfusion `.edds` → lossless intermediate (PNG or raw) — proven on ≥1 Eden cell before full batch.
2. **Inventory** all Eden supertexture cells: grid index, world bounds per cell, resolution.
3. **Stitch** to one north-up ortho aligned to `[0,0,12800,12800]` (target ≥ **8192²** effective — ~2× interim; higher if cells native res allows).
4. **Rebuild** `packages/map-assets/everon/tiles/satellite/{z}/{x}/{y}.webp` (tile pyramid **z0–5**, XYZ on disk; Deck orthographic zoom maps separately) via existing `build-tile-pyramid.sh` (no `--flip-v` unless decode baseline proves upside-down).
5. **Replace** committed LFS tiles; update ops log + verify log with new source metadata.
6. **Acceptance:** operator zoom-in shows **meter-scale ground texture** (fields, forest floor, coastal detail) — visibly sharper than interim rasterization at same zoom.

---

## Phased execution

### P0 — Decode spike (gate: do not proceed without PASS)

| Step | Output |
|------|--------|
| Locate one `Eden_*_supertexture.edds` via pak symlink farm or Workbench `wb_resources` | path recorded |
| Implement or adopt `.edds` → PNG decoder (CLI or Node; document format findings) | `packages/map-assets/everon/staging/sap-spike/cell-0.png` |
| Assert non-empty RGB, plausible dimensions (not 1×1), visual sanity | `.ai/artifacts/t090_1_2_decode_spike.json` |

**FAIL:** stop; document blockers; do not stitch fake tiles.

### P1 — Cell catalog

Script under `scripts/map-assets/` enumerates Eden supertexture assets → JSON manifest:

`packages/map-assets/everon/staging/sap/cell-catalog.json` — `{ id, eddsPath, gridX, gridY, widthPx, heightPx, worldMinX, worldMinZ, … }`.

Use MCP `asset_search` / pak listing + engine wiki / in-game bounds if available.

### P2 — Stitch

`scripts/map-assets/stitch-sap-ortho.mjs` (or `.sh`) — composite catalog cells → single ortho PNG/TGA in staging.

Write `TBD_SatExport_meta.json` fields (same contract as T-090.1): dimensions, m/px, worldBounds, `source: sap-supertexture-stitch`, `captureMethodId: 6`.

### P3 — Pyramid + ship

```bash
scripts/map-assets/build-tile-pyramid.sh \
  --input packages/map-assets/everon/staging/sap/everon-sap-ortho.png \
  --out packages/map-assets/everon/tiles/satellite \
  --minzoom 0 --maxzoom 5
node scripts/map-assets/verify-tile-pyramid.mjs TERRAIN=everon
make ci-local-frontend
```

Update `.ai/artifacts/map_export_everon.json` → `tileFindings.satellite.method` = sap-supertexture-stitch.

### P4 — Manual acceptance (operator)

| ID | Pass |
|----|------|
| D1 | Zoom to ~100 m scale — ground texture readable (not smooth color ramps) |
| D2 | North up; SE peninsula / coast matches Reforger |
| D3 | H1/H2/H2b still pass on new ortho |
| D4 | Pan/zoom ≥55 fps (pyramid LOD unchanged) |

Log: `.ai/artifacts/t090_1_2_verify_log.md`

---

## Files (expected)

| Action | Path |
|--------|------|
| Create | `scripts/map-assets/decode-edds.mjs` (or `tools/edds/`) |
| Create | `scripts/map-assets/catalog-sap-cells.mjs` |
| Create | `scripts/map-assets/stitch-sap-ortho.mjs` |
| Create | `scripts/map-assets/verify-sap-ortho.mjs` |
| Replace | `packages/map-assets/everon/tiles/satellite/**` |
| Maybe edit | `packages/map-assets/everon/manifest.json` (`tiles.satellite.source`) |
| Artifacts | `.ai/artifacts/t090_1_2_decode_spike.json`, `.ai/artifacts/t090_1_2_verify_log.md` |

**Do not touch** unless regression: `useTerrainBasemapLayer.ts`, `tileUrl.ts` (already correct).

---

## Verification (automated — all PASS to ship)

```bash
test -f .ai/artifacts/t090_1_2_decode_spike.json   # P0 recorded
node scripts/map-assets/verify-sap-ortho.mjs TERRAIN=everon
node scripts/map-assets/verify-tile-pyramid.mjs TERRAIN=everon
make ci-local-frontend
make verify-terrain
node scripts/map-assets/verify-spike-ops-log.mjs TERRAIN=everon
```

**Ship:** tag **`T-090.1.2`** · commit prefix **`T-090.1.2:`** · `active_slice` → **T-090.1.1** (Cursor sync).

---

## Follow-ups (110% bar — post-ship)

Tracked in [`t090_1_2_satellite_backlog.md`](t090_1_2_satellite_backlog.md):

| Slice | Issue |
|-------|-------|
| **T-090.1.2.2** | SAP cell seam lines @ 256 m grid |
| **T-090.1.2.3** | Pan ~40 fps + tile pop-in |
| **T-090.1.2.5** | No readable water (ocean + inland) |
| **T-090.1.2.4** | Engine render ortho R&D (idea, deferred) |

---

## Out of scope

- Map cartographic tiles (T-090.1.1)
- Full `.edds` tooling for non-Eden terrains (Arland = follow-on)
- World object vector layers (T-090.5)

---

## Related

- Interim ship: T-090.1 @ `564419e` (rasterization — `mod-maprasterization-export`)
- Spike S3 source paths: `.ai/artifacts/map_export_everon.json` → `tileFindings.satellite`
