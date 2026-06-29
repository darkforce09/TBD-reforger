# T-091.2 — Claude Code handoff (copy-paste prompt)

**Date:** 2026-06-29 · **Spec:** [`t091_2_z_axis_editor.md`](../../docs/specs/Mission_Creator_Architecture/t091_2_z_axis_editor.md) (read **entire** file — A4–A9, S1–S5, M1–M7)

---

## Copy this into a **new** Claude Code chat

```
Read CLAUDE.md §Status, then implement ONLY from:
  docs/specs/Mission_Creator_Architecture/t091_2_z_axis_editor.md

Slice T-091.2 — Z-axis editor UX. Website only.

═══ PREFLIGHT (repo root — run first) ═══
  git lfs pull                              # if everon-dem-16bit.png missing
  make map-assets-link
  ./scripts/ticket brief T-091              # confirm active slice T-091.2

═══ T-091.1 DONE @ 2c56c2e — CONSUME, DO NOT REDO ═══
  Import sampleElevation, isDemReady, isDemDegraded from tactical-map/dem
  Do NOT reimplement dem/*, pngjs, vitest anchor tests, or Everon map-assets

═══ LOCKED DECISIONS ═══
  Sample z: addSlot, pasteSlots (re-sample NOT clipboard z), moveEntities on commit
  Manual Z: updateSlotPosition preserved until next move/paste/add
  CUR z: TacticalMap emitCursor → sampleElevation(x,y) when isDemReady()
  Toolbelt Z: 3 decimal places (toFixed(3)); X/Y stay integer fmt
  Degraded: z=0; keep T-091.1 sonner toast+Retry (M7)
  Hillshade: new useDemLayer.ts from DemController meters cache; default OFF
  Grid toggle: meta.environment.showGrid → TacticalMap showGrid (procedural grid)
  NOT T-090.1 tiles — M6 is grid only

═══ BUILD ═══
  ydoc.ts — terrainZ via sampleElevation in addSlot/pasteSlots/moveEntities
  TacticalMap.tsx — CUR z sampling; hillshade layer; showGrid from props
  dem/DemController.ts — optional getDemRasterForOverlay() for hillshade (internal)
  layers/useDemLayer.ts — NEW hillshade BitmapLayer (build once on DEM ready)
  BottomToolbelt.tsx — Z display 3 dp
  MissionSettingsDialog.tsx — Show hillshade + Show grid toggles
  schema.ts — environment.showHillshade, environment.showGrid
  MissionCreatorPage.tsx — showGrid from meta (remove hard-coded showGrid)
  compile.ts — verify only (editor.slots already carries position.z)

═══ NOT IN SCOPE ═══
  T-092.2 mod slots[].y, compiler worker DEM fetch, T-090.1 TileLayer, bulk re-sample

DO NOT edit docs/**.

Verify (all exit 0):
  make map-assets-link
  cd apps/website/frontend && npm run build && npm run lint && npm test
  make verify-terrain-strict
  ! rg 'map-assets|fetch.*dem' apps/website/frontend/src/features/mission-creator/compiler/

Manual (Everon, dev-login):
  M1 CUR Z: hill-north 9600,3200 ~221.652 vs valley-inland 5000,5000 ~80.871 (>5m delta)
  M2 drop slot → SEL Z ≠ 0
  M3 Attributes Z 123.456 → Save Version → editor.slots[].position.z in POST
  M4 drag → Z re-samples
  M5 hillshade toggle
  M6 grid toggle (not tiles)
  M7 break DEM path → toast + z=0
```

---

## Shipped dependency (read-only)

| Artifact | Path |
|----------|------|
| DEM API | `apps/website/frontend/src/features/tactical-map/dem/` @ `2c56c2e` |
| Anchor coords | `docs/specs/Mission_Creator_Architecture/t091_1_dem_loader.md` §Unit test table |

```bash
make map-assets-link
./scripts/ticket brief T-091
```
