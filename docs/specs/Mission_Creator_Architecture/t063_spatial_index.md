# T-063 — Spatial index for click / marquee pick @ 360k

**Status:** **shipped + verified** — FE build/lint clean; manual @ ~367k: click/marquee significantly faster vs Deck GPU pick (2026-06-24).  
**Git tag on ship:** **T-063** (`078960e`)  
**Authority:** [MC ROADMAP](ROADMAP.md) §Map performance · [agent_execution.md](agent_execution.md) §ACTIVE SLICE · [t061_drag_move_hotfix.md](t061_drag_move_hotfix.md) · [t057_map_performance_hotfix.md](t057_map_performance_hotfix.md)

**Prerequisites:** T-062.1.1 shipped (`4baf5fa`). Repro mission: `70a36667-612f-40c5-ad56-3fb8e0613a17` (~367k slots).

---

## In one sentence

**Click and box-select used to scan all 360k icons; T-063 adds an rbush lookup tree so picking only checks icons near the cursor or inside the box.**

---

## Problem (pre-T-063)

Pan/zoom (T-057) and drag-move (T-061) were fast @ 367k. **Picking was not.**

Every click, double-click, drag-start, and marquee release ran Deck.gl `pickObject` / `pickObjects` over the full `slot-icons` layer (~367k GPU pick pass). Large marquees could freeze for seconds.

---

## Goal

Replace Deck GPU picking with an **rbush** R-tree in world meters, kept in sync with slot positions. Query the tree instead of Deck; set `slot-icons` **`pickable: false`**.

---

## Shipped implementation

### Dependencies

| Package | Role |
|---------|------|
| `rbush@^4.0.1` | R-tree spatial index |
| `@types/rbush@^4.0.0` | TypeScript types (package does not ship `.d.ts` — devDep) |

### Backend of the index

| File | Role |
|------|------|
| **`slotSpatialIndex.ts`** | Singleton rbush: `rebuild` (bulk `tree.load`), `insert`, `remove`, `updatePositions` (remove+reinsert), `clear`, `pickNearest(px, viewport, radiusPx=4)`, `pickRect(world bbox)`. Degenerate point boxes; per-id `Item` reference for rbush remove-by-reference. Local `Viewport` interface — no Deck import. |
| **`slotIconCache.ts`** | Index wired **inside each mutator**: `rebuildFromSlots→rebuild(dense)`, `append→insert(added)`, `remove→remove`, `patchPositions`/`setPositions→updatePositions`, `clearSlotIconCache→clear`. `exclude`/`restore` unchanged. |

### Pick / selection paths

| File | Change |
|------|--------|
| **`useSelectTool.ts`** | Dropped `deckRef`; `pointerDown` → `pickNearest`; marquee release → `pickRect` in **world** coords; **pending-left pointerUp** handles click-select (plain replace, Ctrl/Cmd toggle, empty deselect / Ctrl preserve — moved from Deck `onClick`). |
| **`TacticalMap.tsx`** | Removed Deck `onClick` + unused pick imports; `onDoubleClick` → `pickNearest`; `deckRef` kept as `<DeckGL ref>` only. |
| **`useIconLayer.ts`** | `slot-icons`: `pickable: false` |

**No `useMapStore.ts` changes** — cache mutators carry the index.

---

## Interaction contract (unchanged)

- Left-drag icon = move; left-drag empty = marquee; middle/right = pan
- 4px threshold; Ctrl/Cmd toggle (T-053); marquee replaces selection
- Dbl-click → Attributes when `selection.ids.length <= 1`
- Pan/cursor (T-057) and drag render path (T-061) untouched

---

## Acceptance (T-063)

| Check | Result |
|-------|--------|
| `npm run build` + `npm run lint` | **Clean** |
| Click select @ ~367k | **Pass** — significantly faster vs pre-T-063 |
| Marquee @ ~367k | **Pass** — no multi-second freeze |
| Dbl-click Attributes | **Pass** |
| Ctrl/Cmd toggle + empty preserve | **Pass** |
| Drag-start 4px threshold | **Pass** |
| Pan / drag-move | **Unchanged** (100+ / ~60 fps) |

---

## Out of scope

- T-064 virtualized outliner, T-065 LOD, T-066 worker, T-067 spatial chunks
- Backend / Y.Doc / compiler changes

---

## After T-063

- **T-065** ✅ cluster/LOD. **T-066** ✅ worker compile — [`t066_worker_compile.md`](t066_worker_compile.md). **T-067** ✅ — [`t067_spatial_chunks.md`](t067_spatial_chunks.md). **Eden T-068+**
- Eden **T-068+** after scale milestones

---

## Documentation sync (Cursor — this commit)

`agent_execution.md`, `ROADMAP.md`, `CLAUDE.md`, `feature_inventory.md` (PERF-PICK-001 + SEL-MAP), `mission-editor.md`, `TAGS.md`, `docs/frontend/ROADMAP.md`, `docs/AGENT_COMMIT_CHECKLIST.md`, spec footers (`t057`–`t062`, `t070`).

---

## Claude Code prompt archive (T-063 — completed)

Historical — do not re-run unless regressing pick perf. See git diff / agent transcript for the copy-paste prompt used to implement T-063.
