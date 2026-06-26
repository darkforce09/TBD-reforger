# T-054 — Attributes modal entry points (map + ORBAT)

**Status:** shipped (T-054)  
**Git tag on ship:** T-054  
**Authority:** [MC ROADMAP](ROADMAP.md) · [eden/gap_analysis.md](eden/gap_analysis.md) (`SEL-ORBAT-DBL-001 / SEL-MAP-004`) · [feature_inventory.md](feature_inventory.md) SEL-ORBAT-DBL-001 / SEL-MAP-004

---

## Goal

Make double-click open the **Attributes** modal consistently from every slot entry point,
and put the map's double-click on the same native-`dblclick` footing the outliner trees
already use.

| ID | Gap | Deliverable |
|----|-----|-------------|
| **SEL-ORBAT-DBL-001** | `SEL-ORBAT-DBL-001` | Double-click a slot row in the **ORBAT** tree opens its Attributes |
| (harden) | `SEL-MAP-004` | Map double-click switches from a hand-rolled 350ms click timer to a native `dblclick` + `pickObject` |

**Out of scope:** ORBAT *authoring* (T-071), any AttributesModal field change, the Editor
Layers tree (already wired), copy/paste (T-056), backend changes.

---

## Locked decisions

| Decision | Choice |
|----------|--------|
| Map detection | **Native `onDoubleClick`** on the gesture-host container `<div>` (Deck has no `onDblClick`). Pick the slot under the cursor with `deckRef.pickObject({ layerIds:['slot-icons'] })` — the same pick `useSelectTool.onPointerDown` already does |
| Remove | The `lastClick` `useRef` + the `350ms` manual double-click block in `onClick`, and its resets |
| ORBAT | Mirror `EditorLayersSection`: `OrbatSection` gains `onActivateSlot` and passes `onActivate` to its `TreeView`; `LeftSidebar` threads the existing `onActivateSlot` prop through |
| Suppression | Unchanged — `MissionCreatorPage.onEntityActivate` keeps the `selection.ids.length <= 1` guard, so a Ctrl-built multi (T-053) still won't open Attributes on dbl-click |
| `doubleClickZoom` | Stays `false` (DOM `dblclick` is independent of Deck's zoom controller option) |
| Diff size | **Minimal** — three files; `TreeView` and `MissionCreatorPage` unchanged |

### Rationale

- The shared `TreeView` already fires `onActivate` on a non-folder row's native
  `onDoubleClick` (`TreeView.tsx` L190). Editor Layers consumes it; ORBAT simply never
  passed the prop. So T-054 is pure wiring — no TreeView change.
- The map's manual timer duplicated click bookkeeping and was perturbed by the T-053
  Ctrl/Cmd-toggle path (which reset `lastClick`). A native container `dblclick` + a single
  `pickObject` is the same idiom as the trees and as `useSelectTool`'s pick, so the editor
  has **one** double-click contract.

---

## Implementation specification

### 1. Map — `frontend/src/features/tactical-map/TacticalMap.tsx`

- Add `onDoubleClick` on the container `<div>` (next to the existing pointer + drop handlers):
  ```ts
  const onDoubleClick = useCallback(
    (e: React.MouseEvent) => {
      const el = containerRef.current
      const deck = deckRef.current
      if (!el || !deck) return
      const r = el.getBoundingClientRect()
      const info = deck.pickObject({
        x: e.clientX - r.left,
        y: e.clientY - r.top,
        radius: 4,
        layerIds: ['slot-icons'],
      })
      const id = (info?.object as { id: ID } | undefined)?.id
      if (id) onEntityActivate?.(id)
    },
    [onEntityActivate],
  )
  ```
- Remove from `onClick`: the `lastClick` `useRef`, the `prev/now/350ms` activate block, and
  the two `lastClick.current = null` resets. `onClick` keeps only the additive toggle
  (T-053), single-select replace (`SEL-MAP-001`), and empty-click deselect (`SEL-MAP-002`).

### 2. ORBAT — `OrbatSection.tsx`

```ts
interface OrbatSectionProps { onActivateSlot?: (id: ID) => void }
...
const onActivate = (id: string) => { if (slotsById[id]) onActivateSlot?.(id) }
...
<TreeView nodes={nodes} selectedIds={selectedIds} onSelect={onSelect} onActivate={onActivate} />
```

### 3. Sidebar — `LeftSidebar.tsx`

Pass the existing `onActivateSlot` through to `<OrbatSection onActivateSlot={onActivateSlot} />`
(it already threads it to `<EditorLayersSection>`).

---

## Files to change (checklist)

| File | Change |
|------|--------|
| `frontend/src/features/tactical-map/TacticalMap.tsx` | native `onDoubleClick` + `pickObject`; remove `lastClick` timer |
| `frontend/src/features/mission-creator/layout/LeftOutliner/OrbatSection.tsx` | `onActivateSlot` prop → `onActivate` on TreeView |
| `frontend/src/features/mission-creator/layout/LeftOutliner/LeftSidebar.tsx` | thread `onActivateSlot` to OrbatSection |

**No backend change. No store/schema change. No TreeView / MissionCreatorPage change.**

---

## Verification

```bash
cd frontend && npm run build && npm run lint
```

### Manual test plan (`make web`, dev-login `mission_maker`, `/missions/:id/edit`)

1. Place a couple of units. Double-click one **on the map** → Attributes opens for it.
2. Double-click a slot row in the **ORBAT** tree → Attributes opens.
3. Double-click a slot row in **Editor Layers** → still opens (regression).
4. **T-053 unchanged:** Ctrl/Cmd+click toggles in/out; plain click replaces; marquee +
   drag-move work; a Ctrl-built multi-select does *not* open Attributes on dbl-click.

---

## Documentation sync (same commit — T-054)

Use [`docs/AGENT_COMMIT_CHECKLIST.md`](../../docs/AGENT_COMMIT_CHECKLIST.md).

| Doc | Change |
|-----|--------|
| **This file** | Status → **shipped** |
| [`CLAUDE.md`](../../CLAUDE.md) §Status | T-054 bullet + bump `latest feature work` line |
| [`feature_inventory.md`](feature_inventory.md) | SEL-ORBAT-DBL-001 → **working**; SEL-MAP-004 Procedure/Evidence → native `dblclick` + `pickObject` |
| [`agent_execution.md`](agent_execution.md) | Decisions log row **Attributes entry points (T-054)** |
| [`ROADMAP.md`](ROADMAP.md) | T-054 → shipped; refresh "Next" |
| [`eden/gap_analysis.md`](eden/gap_analysis.md) | T-054 → ✅ shipped T-054 |

**Do not update:** archive stitch, Eden wiki artifacts, historical CLAUDE bullets.

---

## Git strategy

**One T-054 commit** on `main`: code + doc finalize + CLAUDE §Status. Co-Authored-By when
using AI. **Do not commit until the user asks.**

---

## Related

- Prior: [t053_additive_select.md](t053_additive_select.md)
- Next Eden backlog: see [`docs/TICKET_LEAD.md`](../../docs/TICKET_LEAD.md) (T-068+).
