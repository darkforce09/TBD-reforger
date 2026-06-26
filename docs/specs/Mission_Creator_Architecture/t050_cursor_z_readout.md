# T-050 — Cursor Z readout (CUR mode shows X/Y/Z)

**Status:** shipped (T-050)  
**Git tag on ship:** T-050  
**Authority:** [MC ROADMAP](ROADMAP.md) §Recommended program order · [eden/ui_anatomy.md](eden/ui_anatomy.md) Status Bar X/Y/Z · follows [t049_terrain_title_position.md](t049_terrain_title_position.md)

---

## Goal

A one-line follow-up to T-049: the bottom toolbelt's **CUR** (cursor) mode now shows **X/Y/Z**
instead of X/Y with a dimmed `—` for Z.

| ID | Gap | Deliverable |
|----|-----|-------------|
| **MAP-CURSOR-001** | Status bar Z | Cursor readout shows `Z` (= 0 on the flat map) while hovering on-map |

**Out of scope:** DEM / elevation sampling (Z stays a literal 0 until Phase 2), any `WorldPoint`
type export, `onAssetDrop` changes, SEL-mode behavior.

---

## Locked decisions (user confirmed)

| Decision | Choice |
|----------|--------|
| CUR readout | Cursor mode shows **X/Y/Z** (not X/Y only) |
| Flat-map Z | **Z = 0** while the cursor is on the map — a real value, not a placeholder. **Do not** show `—` on-map. |
| Off-map hover | Null cursor → **all axes show `—`** (unchanged) |
| SEL mode | Unchanged — single selected slot shows entity X/Y/Z from `slot.position` |
| Diff size | **Minimal** — no new `WorldPoint` export, do not touch `onAssetDrop` |

### Rationale — why Z = 0 (not `—`)

- The Reforger world is a flat local grid in meters (Ultra Plan §4.1); with no DEM yet, the ground
  plane elevation **is** 0 — so `0` is the correct reading, and a `—` falsely implies "unknown".
- Deck.gl's `OrthographicView` unproject returns `coordinate[2]` (z) — `?? 0` covers the flat case
  and will carry real elevation for free once a DEM layer feeds z (**T-091** DEM).

---

## Root cause audit

T-049 shipped the selection-aware toolbelt with `cursor Z stays —` as a deliberate placeholder
(`BottomToolbelt.tsx` — `z = showSel ? slot.position.z : undefined`, plus a `text-outline` dim on
the CUR Z span). The engine's `onCursorMove` payload only carried `{ x, y }`, so the cursor's z was
never plumbed through even though `info.coordinate[2]` is available.

---

## Implementation specification

### 1. Engine payload — `onCursorMove` carries z

**File:** [`frontend/src/features/tactical-map/types.ts`](../../frontend/src/features/tactical-map/types.ts)

`onCursorMove?: (world: { x: number; y: number; z: number } | null) => void` — JSDoc: z is 0 on the
flat map until Phase 2 DEM.

### 2. Emit z on hover

**File:** [`frontend/src/features/tactical-map/TacticalMap.tsx`](../../frontend/src/features/tactical-map/TacticalMap.tsx)

`onHover` → `{ x: info.coordinate[0], y: info.coordinate[1], z: info.coordinate[2] ?? 0 }` (null branch unchanged).

### 3. Cursor state type

**File:** [`frontend/src/features/mission-creator/MissionCreatorPage.tsx`](../../frontend/src/features/mission-creator/MissionCreatorPage.tsx)

`useState<{ x: number; y: number; z: number } | null>(null)`.

### 4. Toolbelt readout

**File:** [`frontend/src/features/mission-creator/layout/BottomToolbelt.tsx`](../../frontend/src/features/mission-creator/layout/BottomToolbelt.tsx)

- prop `cursorWorld: { x: number; y: number; z: number } | null`
- `z = showSel ? selectedSlot.position.z : cursorWorld?.z`
- remove the CUR-mode `text-outline` dim on the Z span (Z renders like X/Y in both modes)
- comment updated to "cursor X/Y/Z (Z = 0 on the flat map until Phase 2 DEM)"

---

## Files to change (checklist)

| File | Change |
|------|--------|
| `frontend/src/features/tactical-map/types.ts` | `onCursorMove` payload `+ z` |
| `frontend/src/features/tactical-map/TacticalMap.tsx` | `onHover` emits `z: info.coordinate[2] ?? 0` |
| `frontend/src/features/mission-creator/MissionCreatorPage.tsx` | cursor state `+ z` |
| `frontend/src/features/mission-creator/layout/BottomToolbelt.tsx` | CUR Z from cursor; un-dim |

**No backend changes.**

---

## Verification

```bash
cd frontend && npm run build && npm run lint
```

### Manual test plan

1. Open `/missions/:id/edit`, hover over the map → toolbelt shows **CUR X/Y/Z** with **Z = 0**.
2. Move the cursor off the map → all three axes show `—`.
3. Select a single slot → **SEL** mode shows entity X/Y/Z from `slot.position` (unchanged).

---

## Documentation sync (same commit — T-050)

Use [`docs/AGENT_COMMIT_CHECKLIST.md`](../../docs/AGENT_COMMIT_CHECKLIST.md).

| Doc | Change |
|-----|--------|
| **This file** | Status → **shipped** |
| [`CLAUDE.md`](../../CLAUDE.md) §Status | T-050 bullet + bump latest-feature line |
| [`docs/TAGS.md`](../../docs/TAGS.md) | T-050 row |
| [`frontend/docs/pages/mission-editor.md`](../../frontend/docs/pages/mission-editor.md) | Element #5 (toolbelt X/Y/Z incl. cursor) + M3.6 milestone |
| [`feature_inventory.md`](feature_inventory.md) | `MAP-CURSOR-001` — X/Y/Z, Outputs fix, acceptance |
| [`agent_execution.md`](agent_execution.md) | Decisions log — CUR readout X/Y/Z |
| [`ROADMAP.md`](ROADMAP.md) | DONE T-050; T-050 shipped note; shipped-list cursor X/Y/Z |
| [`docs/frontend/ROADMAP.md`](../../docs/frontend/ROADMAP.md) | Recently shipped T-050 |
| [`eden/ui_anatomy.md`](eden/ui_anatomy.md) | Status Bar X/Y/Z mapping row |
| [`t049_terrain_title_position.md`](t049_terrain_title_position.md) | Amendment: renumber Future T-050 title PATCH → **T-051**; mark "cursor Z stays `—`" superseded |

**Do not update:** archive stitch, Eden wiki artifacts.

---

## Related

- Prior: [t049_terrain_title_position.md](t049_terrain_title_position.md)
- Next: [**t052_undo_shortcuts.md**](t052_undo_shortcuts.md) — keyboard undo/redo (T-052, shipped)
- Deferred: **T-051** — optional `PATCH` title sync on Save Version / debounced strip edit
- **T-091** DEM feeds real cursor + slot Z — blocked on hosted heightmaps
