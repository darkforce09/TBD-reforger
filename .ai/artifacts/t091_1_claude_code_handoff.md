# T-091.1 — Claude Code handoff (copy-paste prompt)

**Date:** 2026-06-29 · **Spec:** [`t091_1_dem_loader.md`](../../docs/specs/Mission_Creator_Architecture/t091_1_dem_loader.md) (read **entire** file — acceptance S1–S10)

---

## Copy this into a **new** Claude Code chat

```
Read CLAUDE.md §Status, then implement ONLY from:
  docs/specs/Mission_Creator_Architecture/t091_1_dem_loader.md

Slice T-091.1 — frontend DEM loader + sampleElevation. Website only.

═══ T-091.0 DONE @ 6d96339 — DO NOT REDO ═══
  No Workbench, MCP, TBD_TerrainExportPlugin.c, re-export PNG, anchor edits,
  manifest/dem-sample.mjs drift without updating both sides, or map-assets changes.

═══ BUILD ═══
  dem/terrainManifest.ts, sampleElevation.ts, DemTexture.ts, DemController.ts, index.ts
  Port math from packages/tbd-schema/scripts/lib/dem-sample.mjs (must match verify gate)
  sampleElevation(editorX, editorY) — editor y = world z in verify script
  Float32Array meters cache after decode; bilinear on uint16 then uint16ToMeters
  PNG: skipRescale:true, .depth not .bitDepth
  make map-assets-link (or symlink public/map-assets)
  vitest.config.ts + pngjs devDep + sampleElevation.test.ts
  TacticalMap.tsx useEffect → loadDemForTerrain(terrainId)
  Export sampleElevation/isDemReady/isDemDegraded from tactical-map barrel

═══ NOT IN SCOPE (T-091.2) ═══
  Z on place/move, toolbelt Z, hillshade, ydoc z, compiler worker DEM fetch

DO NOT edit docs/**.

Verify (all exit 0):
  make map-assets-link
  cd apps/website/frontend && npm install && npm run build && npm run lint && npm test
  make verify-terrain-strict
  ! rg 'map-assets|dem/|sampleElevation' apps/website/frontend/src/features/mission-creator/compiler/

Unit test expected (real PNG, ±0.01 m):
  coast-sw      2000, 2000  → -7.408
  valley-inland 5000, 5000  → 80.871
  hill-north    9600, 3200  → 221.652
  seabed-e     11000, 6400  → -84.860  (recommended 4th case)
```

---

## Shipped assets (read-only)

| Artifact | Path |
|----------|------|
| DEM PNG | `packages/map-assets/everon/dem/everon-dem-16bit.png` |
| Manifest | `packages/map-assets/everon/manifest.json` |
| Reference | `packages/tbd-schema/scripts/lib/dem-sample.mjs` |
| sha256 | `585e1432ddf24dfb963f81510b4b570a41c68ec8ea85f56e755c3c5f95f4517b` |

```bash
git lfs pull   # if PNG missing
make map-assets-link
./scripts/ticket brief T-091
```
