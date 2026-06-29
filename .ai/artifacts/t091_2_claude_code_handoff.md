# T-091.2 — Claude Code handoff (copy-paste prompt)

**Date:** 2026-06-29 (UX + hillshade locked) · **Spec:** [`t091_2_z_axis_editor.md`](../../docs/specs/Mission_Creator_Architecture/t091_2_z_axis_editor.md) (read **entire** file — A4–A9, S1–S5, M1–M8)

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
  Import sampleElevation, isDemReady, isDemDegraded from tactical-map/dem (or ../dem leaf in ydoc)
  Do NOT reimplement dem/* decode/pngjs, vitest anchor tests, or Everon map-assets

═══ LOCKED DECISIONS (spec §Locked decisions) ═══
  Sample z: addSlot, pasteSlots (re-sample NOT clipboard z), moveEntities on commit
  Attributes: patch.z only → manual Z sticks; patch.x OR patch.y → re-sample z (terrain-follow)
  CUR z: TacticalMap emitCursor → sampleElevation(x,y) when isDemReady()
  Async CUR: z stays 0 until next pointermove if DEM loads while cursor stationary — OK v1
  Toolbelt Z: 3 decimal places (toFixed(3)); X/Y stay integer fmt
  Degraded: z=0; keep T-091.1 sonner toast+Retry (M7)
  Hillshade: useDemLayer.ts BitmapLayer; downsample ≤1024px edge; Horn/NW light ~40% opacity; default OFF
  Grid toggle: meta.environment.showGrid → TacticalMap showGrid (procedural grid, NOT T-090.1 tiles)
  ydoc import: ../dem or ../dem/DemController — NOT @/features/tactical-map barrel

═══ BUILD (files — spec §Files) ═══
  state/ydoc.ts — terrainZ in addSlot/pasteSlots/moveEntities; updateSlotPosition X/Y re-sample
  TacticalMap.tsx — CUR z; showHillshade layer; showGrid from props
  types.ts — showHillshade prop; fix stale CUR z comment
  dem/DemController.ts — getDemRasterForOverlay() internal only (not barrel)
  layers/useDemLayer.ts — NEW hillshade BitmapLayer (build once per terrain)
  layout/BottomToolbelt.tsx — Z display 3 dp
  layout/MissionSettingsDialog.tsx — Show hillshade + Show grid toggles
  state/schema.ts — environment.showHillshade, environment.showGrid
  MissionCreatorPage.tsx — showGrid + showHillshade from meta (remove hard-coded showGrid)
  compiler/compile.ts — verify only (editor.slots already carries position.z)

═══ NOT IN SCOPE ═══
  T-092.2 mod slots[].y, compiler worker DEM fetch, T-090.1 TileLayer, bulk re-sample, docs/**

DO NOT edit docs/**.

Verify (all exit 0):
  make map-assets-link
  cd apps/website/frontend && npm run build && npm run lint && npm test
  make verify-terrain-strict
  ! rg 'map-assets|fetch.*dem' apps/website/frontend/src/features/mission-creator/compiler/

Manual (Everon, dev-login, DEM loaded):
  M1 CUR Z: hill-north 9600,3200 ~221.652 vs valley-inland 5000,5000 ~80.871 (>5m delta)
  M2 drop slot at valley-inland → SEL Z ≈ 80.871
  M3 Attributes Z 123.456 → Save Version → editor.slots[].position.z in POST
  M4 drag → Z re-samples on release
  M5 hillshade toggle on/off
  M6 grid toggle on/off (procedural grid, not tiles)
  M7 break DEM path → toast + Retry + z=0
  M8 Attributes X-only edit on slope → Z re-samples
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
