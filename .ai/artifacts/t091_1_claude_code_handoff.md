# T-091.1 — Claude Code handoff (copy-paste prompt)

**Status:** **SHIPPED** @ `2c56c2e` (2026-06-29) — historical reference only. Active slice: **T-091.2**.

**Date:** 2026-06-29 · **Spec:** [`t091_1_dem_loader.md`](../../docs/specs/Mission_Creator_Architecture/t091_1_dem_loader.md) (acceptance S1–S10 — all PASS)

---

## Copy this into a **new** Claude Code chat

```
Read CLAUDE.md §Status, then implement ONLY from:
  docs/specs/Mission_Creator_Architecture/t091_1_dem_loader.md

Slice T-091.1 — frontend DEM loader + sampleElevation. Website only.

═══ PREFLIGHT (repo root — run first) ═══
  git lfs pull                              # if everon-dem-16bit.png missing locally
  make map-assets-link                      # symlink public/map-assets
  ./scripts/ticket brief T-091              # confirm active slice + spec path

═══ T-091.0 DONE @ 6d96339 — DO NOT REDO ═══
  No Workbench, MCP, TBD_TerrainExportPlugin.c, re-export PNG, anchor edits,
  manifest/dem-sample.mjs drift without updating both sides, or everon map-assets edits.
  Arland stub manifest (widthPx:0) is committed — do not change unless spec says so.

═══ LOCKED DECISIONS (confirmed) ═══
  OOB: clamp (x,y) to terrain bounds BEFORE worldToPixel — public API never throws
  PNG decode: pngjs production dependency { skipRescale: true }; .depth not .bitDepth
  Load failure: sonner toast WITH Retry (re-runs loadDemForTerrain)
  Stub terrain (Arland widthPx:0): degraded flat mode; toast WITH Retry (same as load failure)
  Degraded/not-ready: sampleElevation → 0 (not NaN)
  Math: port dem-sample.mjs faithfully; bilinear on uint16 then uint16ToMeters

═══ BUILD ═══
  dem/terrainManifest.ts, sampleElevation.ts, DemTexture.ts, DemController.ts, index.ts
  Float32Array meters cache after decode
  make map-assets-link (Makefile target exists; make web runs it)
  vitest.config.ts + pngjs dependency + sampleElevation.test.ts
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

Unit tests (real PNG @ 6d96339, ±0.01 m) — ALL 11 anchors required:
  bridgehead-sl 4839.2,6620.8 → 121.784
  bridgehead-tl0 4836.9,6626.5 → 123.328
  bridgehead-tl1 4831.2,6628.8 → 123.602
  coast-w 1000,6400 → 0.054
  valley-inland 5000,5000 → 80.871
  hill-north 9600,3200 → 221.652
  peak-central 6400,6400 → 157.882
  coast-sw 2000,2000 → -7.408
  seabed-e 11000,6400 → -84.860
  shelf-ne 8000,8000 → -18.314
  mid-s 3200,9600 → -47.743
  Plus S8 arland stub + S9 clamp + synthetic 2×2 + S10 not-ready→0
```

---

## Shipped assets (read-only)

| Artifact | Path |
|----------|------|
| DEM PNG | `packages/map-assets/everon/dem/everon-dem-16bit.png` (71,911,548 bytes) |
| Manifest | `packages/map-assets/everon/manifest.json` |
| Arland stub | `packages/map-assets/arland/manifest.json` (`widthPx/heightPx: 0`) |
| Reference | `packages/tbd-schema/scripts/lib/dem-sample.mjs` |
| sha256 | `585e1432ddf24dfb963f81510b4b570a41c68ec8ea85f56e755c3c5f95f4517b` |

```bash
git lfs pull   # if PNG missing
make map-assets-link
./scripts/ticket brief T-091
```
