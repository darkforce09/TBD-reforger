# T-091.2 — Z-axis editor UX

**Ticket:** T-091 · **Slice:** T-091.2  
**Status:** **active** — unblocked @ T-091.1 `2c56c2e`  
**Executor:** claude-code  
**Authority:** [`t090_091_map_terrain_program.md`](t090_091_map_terrain_program.md)

---

## In one sentence

Wire T-091.1 `sampleElevation` into slot placement/move/paste, live CUR/SEL Z readout, optional hillshade + grid toggles, and ensure `editor.slots[].position.z` survives Save Version — mod `slots[].y` flatten stays **T-092.2**.

---

## DO NOT (T-091.1 shipped — consume, do not redo)

| Do **not** touch | Reason |
|------------------|--------|
| `tactical-map/dem/*` loader math / pngjs decode | T-091.1 @ `2c56c2e` — import `sampleElevation`, `isDemReady`, `isDemDegraded` |
| Everon PNG / manifest / anchors | T-091.0 |
| `compiler.worker.ts` DEM fetch | T-092.2 — worker reads `MapSnapshot` only |
| Mod `slots[].y` / export flatten | **T-092.2** |
| Aligned tile pyramid / `TileLayer` | **T-090.1** (no tiles in repo yet) |
| Bulk legacy mission re-sample | Optional **T-091.3** deferred |
| `docs/**` | Cursor doc sync |

---

## Prerequisites

| Gate | Evidence |
|------|----------|
| **T-091.1** | ✅ `loadDemForTerrain` + `sampleElevation()` @ `2c56c2e`; vitest 15/15; Everon DEM loads in editor |
| **T-091.0** | Anchor verify PASS @ `6d96339` |
| **Dev serve** | `make map-assets-link` — DEM PNG HTTP 200 |

---

## Problem (verified in repo @ `2c56c2e`)

| Location | Today |
|----------|--------|
| [`ydoc.ts`](../../../apps/website/frontend/src/features/tactical-map/state/ydoc.ts) `addSlot` | `z: 0` hard-coded (line ~145) |
| `pasteSlots` | Copies `c.position.z` from clipboard — does **not** re-sample at pasted x/y |
| `moveEntities` | Updates x/y only — **no** z re-sample on drag release |
| [`TacticalMap.tsx`](../../../apps/website/frontend/src/features/tactical-map/TacticalMap.tsx) `emitCursor` | `onCursorMove({ x, y, z: 0 })` (line ~150) |
| [`BottomToolbelt.tsx`](../../../apps/website/frontend/src/features/mission-creator/layout/BottomToolbelt.tsx) | Reads `cursorWorld?.z` / `selectedSlot.position.z` but values stay 0; `fmt()` uses **`Math.round`** (integer) — wrong for 0.001 m contract |
| [`useDemLayer.ts`](../../../apps/website/frontend/src/features/tactical-map/layers/useDemLayer.ts) | **Does not exist** |
| `MissionSettingsDialog` | No hillshade/grid toggles |
| `compile.ts` | `editor.slots` = full `Object.values(slotsById)` — **already includes** `position.z` when store has it (S2 is a ydoc wiring check, not a compiler rewrite) |

---

## Goal

| Touchpoint | Change |
|------------|--------|
| `ydoc.ts` `addSlot` | `z: sampleElevation(x, y)` when `isDemReady()`, else `0` |
| `ydoc.ts` `pasteSlots` | After clamped x/y, `z: sampleElevation(x, y)` (not clipboard z) |
| `ydoc.ts` `moveEntities` | After x/y delta, `z: sampleElevation(newX, newY)` per moved slot |
| `ydoc.ts` `updateSlotPosition` | **Unchanged** — manual Z via Attributes preserved until next move/paste/add |
| `TacticalMap.tsx` `emitCursor` | `z: sampleElevation(x, y)` when `isDemReady()`, else `0` |
| `BottomToolbelt.tsx` | CUR/SEL Z: **3 decimal places** (`toFixed(3)`, tabular-nums); X/Y stay integer meters |
| `MissionSettingsDialog.tsx` | Toggles: **Show hillshade**, **Show grid** (procedural grid — not T-090.1 tiles) |
| `useDemLayer.ts` | New — hillshade overlay from DEM meters cache (see §Hillshade) |
| `TacticalMap.tsx` layers | Insert hillshade layer when enabled + DEM ready; `showGrid` driven by settings (not hard-coded prop) |
| `schema.ts` / `meta.environment` | Optional `showHillshade?: boolean`, `showGrid?: boolean` persisted on mission |
| `compile.ts` | **Verify only** — no structural change expected if ydoc writes z |

---

## Out of scope

- Mod `slots[].y` flatten (**T-092.2**)
- Viewshed / ruler elevation tools (**post T-091**)
- **T-090.1** aligned WebP tile basemap (M6 tile imagery deferred — grid toggle only)
- Compiler worker DEM fetch
- Re-open T-091.1 loader / pngjs / vitest anchor tests (unless regression fix)

---

## Locked decisions (confirmed for implementation)

| Decision | Choice | Evidence |
|----------|--------|----------|
| **Elevation API** | Import **`sampleElevation`** from `tactical-map/dem` (barrel) — do not duplicate math | T-091.1 public contract |
| **When to sample** | `addSlot`, `pasteSlots`, `moveEntities` commit; **not** on drag preview (preview stays xy-only) | Existing drag uses `dragPreviewDelta` + commit on release |
| **Manual Z wins** | `updateSlotPosition` `{ z }` sticks until next move/paste/add re-sample | Spec + existing Attributes `NumberField` |
| **Display precision** | **3** decimal places (0.001 m) — match `manifest.precision.storageDecimals` | Program + T-091.1 rounding |
| **Degraded DEM** | `sampleElevation` → **0**; existing T-091.1 **sonner toast + Retry** — no new banner | T-091.1 DemController; M7 = break URL → toast + z=0 |
| **Grid toggle (M6)** | Toggle **`showGrid`** procedural grid ([`useBaseMapLayer`](../../../apps/website/frontend/src/features/tactical-map/layers/useBaseMapLayer.ts)) — **not** T-090.1 tiles | `TacticalMap` already has `showGrid` prop; today hard-coded `showGrid` in `MissionCreatorPage` |
| **Hillshade default** | **Off** until user enables (avoid 6400² overlay cost on first paint) — persist in `meta.environment.showHillshade` | Performance @ scale missions |
| **Grid default** | **On** (`showGrid: true`) — matches current `MissionCreatorPage` | Verified line 175 |
| **Hillshade source** | Build from T-091.1 **Float32 meters cache** in DemController; expose read-only accessor for `useDemLayer` — GPU `Texture` optional if BitmapLayer hillshade is too heavy | T-091.1 ships CPU cache only; engineering_plan GPU path is aspirational |
| **incPatchPlan** | Position z changes via existing **`slot-fields`** path — no new patch kind unless profiling proves otherwise | [`incPatchPlan.ts`](../../../apps/website/frontend/src/features/tactical-map/state/incPatchPlan.ts) line ~182 |

---

## Public API (consume — do not rename)

From [`dem/index.ts`](../../../apps/website/frontend/src/features/tactical-map/dem/index.ts):

```typescript
sampleElevation(x: number, y: number): number  // 0 when not ready / degraded
isDemReady(): boolean
isDemDegraded(): boolean
```

Optional helper in `ydoc.ts` (thin wrapper):

```typescript
function terrainZ(x: number, y: number): number {
  return isDemReady() ? sampleElevation(x, y) : 0
}
```

---

## Hillshade (minimum bar)

- New [`useDemLayer.ts`](../../../apps/website/frontend/src/features/tactical-map/layers/useDemLayer.ts).
- Visible when: `meta.environment.showHillshade === true` **and** `isDemReady()`.
- Data: read meters cache from DemController (add e.g. `getDemRasterForOverlay(): { cache, width, height, terrain } | null` — **internal**, not barrel).
- Render: Deck **`BitmapLayer`** or custom layer with precomputed hillshade RGBA from slope of meters cache (6400² — build **once** on DEM ready, cache in module ref).
- Layer order in `TacticalMap`: `[grid?, hillshade?, …icons]` per engineering_plan §4.3.
- Toggle off → layer omitted (M5 pass).

---

## Meta / settings persistence

Extend [`MissionMeta.environment`](../../../apps/website/frontend/src/features/tactical-map/state/schema.ts):

```typescript
showGrid?: boolean      // default true when undefined
showHillshade?: boolean // default false when undefined
```

Wire [`MissionSettingsDialog.tsx`](../../../apps/website/frontend/src/features/mission-creator/layout/MissionSettingsDialog.tsx) toggles → `updateEnvironment(md, { showGrid, showHillshade })`.

`MissionCreatorPage` passes `showGrid={meta.environment.showGrid !== false}` to `TacticalMap` (replace hard-coded `showGrid`).

---

## Files

| File | Action |
|------|--------|
| `state/ydoc.ts` | Sample z in `addSlot`, `pasteSlots`, `moveEntities` |
| `TacticalMap.tsx` | CUR z via `sampleElevation`; hillshade layer; `showGrid` from props |
| `dem/DemController.ts` | Optional internal overlay accessor (meters cache + dims) |
| `layers/useDemLayer.ts` | **New** — hillshade Deck layer |
| `layout/BottomToolbelt.tsx` | Z format 3 dp |
| `layout/MissionSettingsDialog.tsx` | Hillshade + grid toggles |
| `state/schema.ts` | `environment.showGrid`, `environment.showHillshade` |
| `MissionCreatorPage.tsx` | `showGrid` from meta (not literal prop) |
| `compiler/compile.ts` | Verify only — expect `position.z` in payload |

**Tests (recommended, not blocking if manual M1–M7 pass):**

| File | Action |
|------|--------|
| `ydoc.z-sample.test.ts` or extend dem tests | Mock `sampleElevation` → assert `addSlot` z |

---

## Verification gate (mandatory)

### Pre-flight (repo root)

```bash
git lfs pull                              # if PNG missing
make map-assets-link
./scripts/ticket brief T-091              # confirm active slice T-091.2
```

### Automated (exit 0)

```bash
make map-assets-link
cd apps/website/frontend && npm run build && npm run lint && npm test
make verify-terrain-strict              # unchanged T-091.0 gate
! rg 'map-assets|fetch.*dem' apps/website/frontend/src/features/mission-creator/compiler/
```

`make test-it` — only if compile/integration tests are touched.

### Manual (browser @ Everon, DEM loaded, dev-login)

Use known anchor coordinates from T-091.1 verify table for M1:

| ID | Step | Pass condition |
|----|------|----------------|
| **M1** | Hover **hill-north** `(9600, 3200)` vs **valley-inland** `(5000, 5000)` | CUR Z differs by **>5 m** (~221.652 vs ~80.871) |
| **M2** | Drop slot on slope (e.g. valley-inland) | SEL Z ≠ 0; ≈ sampled elevation |
| **M3** | Attributes → set Z **123.456** → Save Version | `editor.slots[].position.z === 123.456` in POST body |
| **M4** | Drag slot to new XY | Z re-samples on release (matches new terrain) |
| **M5** | Mission Settings → toggle **Show hillshade** | Overlay visible / hidden |
| **M6** | Mission Settings → toggle **Show grid** | Procedural grid visible / hidden (**not** T-090.1 tiles) |
| **M7** | Break DEM URL (rename `dem` → `dem_off`, reload) | Toast + Retry; CUR/slot Z → **0** |

### Acceptance criteria

| ID | Check | Pass condition | How |
|----|-------|----------------|-----|
| **A4** | Cursor Z | M1 | Manual |
| **A5** | New slot Z | M2 | Manual |
| **A6** | Manual Z | M3 | Manual + DevTools POST |
| **A9** | Degraded | M7 | Manual |
| **S1** | Build/lint/test | exit 0 | CI commands |
| **S2** | Version payload | `editor.slots[].position.z` populated | M3 or test mission Save |
| **S3** | No worker DEM | compiler grep gate | `rg` above |
| **S4** | Hillshade toggle | M5 | Manual |
| **S5** | Grid toggle | M6 | Manual |

**Unblocks T-092** when ALL of A4, A5, A6, A9, S1, S2, S3 pass.

---

## Related

- [`t091_1_dem_loader.md`](t091_1_dem_loader.md) — shipped loader @ `2c56c2e`
- [`t092_spawn_transform_program.md`](t092_spawn_transform_program.md) — blocked until this slice ships
- [`t090_1_aligned_basemap.md`](t090_1_aligned_basemap.md) — real tile basemap (after T-091.2)

---

## Claude Code handoff

[`.ai/artifacts/t091_2_claude_code_handoff.md`](../../../.ai/artifacts/t091_2_claude_code_handoff.md)
