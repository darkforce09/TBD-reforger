# T-061 — Drag-move performance @ 360k (motion + boundary hotfix)

**Status:** **shipped (good enough)** — T-061.0 + T-061.0.1 implemented; build + lint clean; manual verify @ ~360k acceptable for Eden-blocking work  
**Git tag on ship:** **T-061** (single commit: T-061.0 + T-061.0.1)  
**Authority:** [MC ROADMAP](ROADMAP.md) §Map performance · [agent_execution.md](agent_execution.md) §ACTIVE SLICE · [t057_map_performance_hotfix.md](t057_map_performance_hotfix.md)

**Prerequisites:** T-057–T-060 shipped (`b1fd25a`). Repro mission: `70a36667-612f-40c5-ad56-3fb8e0613a17` (~360k slots).

---

## Goal

Restore acceptable FPS during **left-drag move** of selected slot icons at **~360k** entity scale.

**Original symptom (pre-T-061):**

| Scenario @ ~360k | FPS |
|----------------|-----|
| Idle / pan (middle-right drag) | **100+** |
| Select (any count) | **Good** |
| **Drag-move selection (sustained)** | **5–10** — not acceptable |
| Drag pickup / release | noticeable hitches |

**After T-061 (manual verify 2026-06 — good enough for ship):**

| Scenario @ ~360k | FPS / feel | Status |
|----------------|------------|--------|
| **Drag during motion** (1 or ~10 selected) | **~60** (rAF cap) | **Pass** |
| **Drag pickup** (cross 4px threshold) | materially improved; minor hitch possible | **Good enough** |
| **Drag release** (pointer-up) | no ~10 fps collapse; may be one dropped frame | **Good enough** |
| Pan | **100+** | unchanged |

**Product call:** current drag perf is **good enough** for Eden-blocking scale work. **Mega optimizations** (typed-array buffers, collapsing release repacks, worker compile, terrain base) are **deferred** — see [ROADMAP.md](ROADMAP.md) §Deferred mega optimizations.

---

## Shipped implementation

### T-061.0 — Motion hotfix

Dual IconLayer + split drag state + rAF-coalesced delta. Fixed sustained 5–10 fps during drag.

| File | Change |
|------|--------|
| `useMapStore.ts` | `dragPreviewIds` + `dragPreviewDelta`; removed monolithic `drag`/`setDrag` |
| `selectors.ts` | `selectSlotIconsBase` + `selectDragOverlayIcons` |
| `useIconLayer.ts` | Base layer + `useDragIconLayer()` overlay |
| `useSelectTool.ts` | rAF-coalesced drag delta; commit on pointer-up |
| `TacticalMap.tsx` | Overlay in layers array |
| `index.ts` | Barrel export update |

### T-061.0.1 — Boundary hotfix

`slotIconCache` O(k) exclude/restore + incremental slot observer in `bindings.ts`.

| File | Change |
|------|--------|
| **NEW** `state/slotIconCache.ts` | Dense `SlotIcon[]` + `Map<ID,index>`; `rebuildFromSlots` (O(n) snapshot only); `exclude`/`restore` (O(k) swap-and-pop); `patchPositions`/`setPositions`; `setSelectionFlags`; `getBaseIcons`/`getVersion` |
| `useMapStore.ts` | `iconCacheVersion`; `_applySnapshot` → rebuild cache; `_patchSlots`; `setSelection` → cache flags; `_syncIconCache`; `reset` clears cache |
| `bindings.ts` | Observer `(events, txn)`; `fastSlotPatchIds` eligibility from whole `txn.changed`; read only k changed slots (not `e.slots.toJSON()`); fallback to full `docToSnapshot` for bulk/structural ops |
| `useIconLayer.ts` | Base layer subscribes `iconCacheVersion` + `getBaseIcons()` only; overlay unchanged |
| `useSelectTool.ts` | Drag start: `exclude` + `_syncIconCache`; pointer-up: optimistic `patchPositions` → `onEntitiesMove` → `restore` → `clearDragPreview` → `_syncIconCache` |

**How boundaries were fixed:**

- **Pickup:** was `Object.values(slotsById)` + ~360k `SlotIcon` allocations. Now `exclude(ids)` is O(k) swap-pop on persistent objects.
- **Release:** was `e.slots.toJSON()` over ~360k. Now bindings fast path reads only k moved slots → `_patchSlots`; optimistic cache patch avoids flicker.

**Known residual (deferred optimization):** at release with an actual move there can be **two** `iconCacheVersion` bumps (restore sync, then microtask `_patchSlots`) → two one-time Deck attribute repacks. Should stay ≥55 fps; if a single dropped frame remains @ 360k, collapse to one bump — **not blocking T-061 ship**.

**Correctness guards:** eligibility uses whole transaction (`txn.changed`); structural ops fall back to full snapshot; optimistic relative patch + bindings absolute patch are idempotent.

---

## Acceptance (T-061 — good enough)

| Check | Result |
|-------|--------|
| Drag motion @ ~360k ≥55 fps sustained | **Pass** (~60) |
| Drag pickup materially improved | **Good enough** |
| Drag release no ~10 fps collapse | **Good enough** |
| Pan 100+ fps | **Pass** |
| build + lint | **Clean** |
| Regression sweep | Manual pass recommended (undo, marquee, Attributes, copy/paste, asset drop) |

---

## After T-061

- ~~**T-062** incremental bindings~~ ✅ — spec [`t062_incremental_bindings.md`](t062_incremental_bindings.md)
- ~~**T-062.2** editor session~~ ✅ — spec [`t062_2_editor_session_persistence.md`](t062_2_editor_session_persistence.md)
- **T-065** ✅ cluster/LOD. **T-066** ✅ worker compile — [`t066_worker_compile.md`](t066_worker_compile.md). **T-067** ✅ — [`t067_spatial_chunks.md`](t067_spatial_chunks.md). **Eden T-068+**
- **Stretch:** ~~**T-062.1** IDB streaming~~ ✅
- **Deferred mega optimizations** — [ROADMAP.md](ROADMAP.md) §Deferred mega optimizations (T-061.1 typed-array, release repack collapse, T-066 worker, T-110 terrain, 1M–10M stretch)

---

## Historical spec (T-061.0 root cause)

T-057 rAF-coalesced pan. Pre-T-061 drag still called `setDrag` every `pointermove` → O(n) `selectSlotIcons` + full IconLayer position refresh every frame.

---

## Documentation sync (Cursor — done on ship)

All docs updated: `agent_execution.md`, `ROADMAP.md`, `feature_inventory.md` PERF-DRAG-001, `mission-editor.md`, `TAGS.md`, `CLAUDE.md`, footers.

---

## Claude Code prompt archive (T-061.0.1 — completed)

Historical — do not re-run unless regressing boundaries. See git history / agent transcript for the copy-paste prompt used to implement T-061.0.1.
