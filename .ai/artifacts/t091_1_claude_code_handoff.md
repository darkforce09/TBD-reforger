# T-091.1 — Claude Code handoff (copy-paste prompt)

**Date:** 2026-06-29 · **Branch:** `ticket/T-091` (or implement on `main` per repo convention)  
**Slice spec:** [`docs/specs/Mission_Creator_Architecture/t091_1_dem_loader.md`](../../docs/specs/Mission_Creator_Architecture/t091_1_dem_loader.md)

---

## Copy this into a **new** Claude Code chat

```
Read CLAUDE.md §Status first, then ONLY:
  docs/specs/Mission_Creator_Architecture/t091_1_dem_loader.md

Implement slice T-091.1 — DEM loader + sampleElevation (frontend only).

T-091.0 is SHIPPED @ 6d96339 (tag T-091.0). DO NOT redo any of it:
  - Do NOT open Workbench, run MCP terrain export, or touch TBD_TerrainExportPlugin.c
  - Do NOT re-export, replace, or regenerate packages/map-assets/everon/dem/everon-dem-16bit.png
  - Do NOT re-probe anchors or edit packages/map-assets/everon/anchors/verification.json
  - Do NOT run raw-u16-to-dem-png.mjs or change the committed DEM/manifest
  The 6400² PNG + 11-anchor verify (maxDeltaM 0.204 m) is done. Use it as-is.

Your scope (website frontend only):
  1. Symlink apps/website/frontend/public/map-assets → ../../../../packages/map-assets
     (or Makefile map-assets-link target)
  2. New modules under apps/website/frontend/src/features/tactical-map/dem/:
     terrainManifest.ts, DemTexture.ts, sampleElevation.ts, DemController.ts
  3. Port sampling math from packages/tbd-schema/scripts/lib/dem-sample.mjs
     (worldToPixel, bilinear, uint16ToMeters — must match verify script)
  4. PNG decode: if using pngjs, { skipRescale: true } and .depth not .bitDepth
  5. Add vitest + sampleElevation.test.ts — 3 anchors ±0.01 m:
     coast-sw 2000/2000 → -7.408 m
     valley-inland 5000/5000 → 80.871 m
     hill-north 9600/3200 → 221.652 m
  6. Wire DemController init in TacticalMap.tsx on terrain mount
  7. Degraded mode: DEM 404 → sampleElevation returns 0 + sonner toast

Out of scope (T-091.2 — do NOT implement now):
  Z on addSlot/move/paste, toolbelt CUR/SEL Z, hillshade, ydoc z writes

DO NOT edit docs/** — return verify output to human/Cursor.

Verify before done:
  cd apps/website/frontend && npm run build && npm run lint
  npm test -- sampleElevation
  make verify-terrain-strict
```

---

## Already in repo (do not recreate)

| Artifact | Path |
|----------|------|
| DEM PNG (LFS) | `packages/map-assets/everon/dem/everon-dem-16bit.png` |
| Manifest | `packages/map-assets/everon/manifest.json` — 6400², `mod-getsurfacey-resample` |
| Anchors | `packages/map-assets/everon/anchors/verification.json` |
| Plugin (shipped, leave alone) | `apps/mod/tbd-framework/Scripts/WorkbenchGame/TBD_TerrainExportPlugin.c` |
| Reference sampler | `packages/tbd-schema/scripts/lib/dem-sample.mjs` |

**DEM sha256:** `585e1432ddf24dfb963f81510b4b570a41c68ec8ea85f56e755c3c5f95f4517b`

---

## Commands

```bash
./scripts/ticket brief T-091
git checkout main   # or ticket/T-091 if using branch mode
./scripts/ticket run   # optional batch runner
```
