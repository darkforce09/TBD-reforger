# T-091.2 — Z-axis editor UX

**Ticket:** T-091 · **Slice:** T-091.2  
**Status:** Spec ready — blocked on **T-091.1**  
**Executor:** claude-code  
**Authority:** [`t090_091_map_terrain_program.md`](t090_091_map_terrain_program.md)

---

## In one sentence

Auto-sample DEM Z on place/move/paste, preserve manual Attributes Z, show real CUR/SEL Z in toolbelt, add hillshade + basemap toggles, and emit `position.z` in version payload (mod flatten deferred to **T-092.2**).

---

## Prerequisites

| Gate | Evidence |
|------|----------|
| **T-091.1** | `sampleElevation()` returns non-zero over varied terrain |
| **T-091.0** | Anchor verify PASS |

---

## Problem

[`ydoc.ts`](../../../apps/website/frontend/src/features/tactical-map/state/ydoc.ts) sets `z: 0` on every slot. Toolbelt CUR Z is flat. No hillshade. Save payload lacks meaningful elevation for mod spawn path.

---

## Goal

| Touchpoint | Change |
|------------|--------|
| `addSlot` | `z: round(sampleElevation(x,y), 3)` |
| `moveEntities` / paste | Re-sample Z on commit |
| `updateSlotPosition` | Manual Z preserved; no overwrite until move commit |
| `BottomToolbelt.tsx` | CUR/SEL show sampled Z |
| `MissionSettingsDialog.tsx` | Toggles: hillshade, basemap |
| `useDemLayer.ts` | Hillshade overlay from DemTexture |
| `compile.ts` | Include `position.z` in `editor.slots` (version POST) — **not** mod `slots[]` yet |

---

## Out of scope

- Mod `slots[].y` flatten (**T-092.2**)
- Viewshed / ruler (**post T-091**)
- Bulk re-sample legacy missions (**optional T-091.3 deferred**)
- Compiler worker DEM fetch — pass pre-built cache or main-thread only

---

## Locked decisions

| Decision | Choice |
|----------|--------|
| Manual Z wins | Attributes edit until next move/paste re-sample |
| Display | 3 decimal places (0.001 m) |
| Incremental bindings | If z-only hot path needed, extend `incPatchPlan` slot-fields |
| Degraded | DEM fail → z=0 + banner; editor still usable |

---

## Verification gate (mandatory)

**Unblocks T-092** only when ALL PASS.

### Automated

```bash
cd apps/website/frontend && npm run build && npm run lint
make test-it   # if compile tests touched
```

### Manual (browser @ Everon with DEM loaded)

| ID | Step | Pass condition |
|----|------|----------------|
| M1 | Cursor over hill vs valley | CUR Z differs by >5 m |
| M2 | Drop slot on slope | SEL Z ≠ 0; matches approximate terrain |
| M3 | Edit Attributes Z to 123.456 | Save Version → JSON shows 123.456 |
| M4 | Drag slot to new XY | Z re-samples on release |
| M5 | Toggle hillshade | Overlay visible/hidden |
| M6 | Toggle basemap | Tiles visible/hidden (T-090.1) |
| M7 | Break DEM URL | Banner + z=0 behavior |

### Acceptance criteria (A4–A6 from program)

| ID | Check | Pass condition |
|----|-------|----------------|
| A4 | Cursor Z | M1 pass |
| A5 | New slot Z | M2 pass |
| A6 | Manual Z | M3 pass |
| A9 | Degraded | M7 pass |
| S1 | Build/lint | exit 0 |
| S2 | Version payload | `editor.slots[].position.z` populated on test mission |

---

## Related

- [`t092_spawn_transform_program.md`](t092_spawn_transform_program.md)
- [`t071_orbat_manager_program.md`](t071_orbat_manager_program.md) — blocked until T-092
