# T-056 — Ctrl+C / Ctrl+V copy-paste (slots)

**Status:** shipped (T-056)
**Git tag on ship:** T-056
**Authority:** [MC ROADMAP](ROADMAP.md) · [eden/gap_analysis.md](eden/gap_analysis.md) (`ACTION-COPY-001 / ACTION-PASTE-001`) · [eden/interactions.md](eden/interactions.md) ACTION-COPY-001 / ACTION-PASTE-001 · [feature_inventory.md](feature_inventory.md)

---

## Goal

Wire **Ctrl+C copy / Ctrl+V paste** for placed slots in the Mission Creator. Today the only
way to create a unit is dragging a fresh asset off the palette — a configured slot (role,
tag, asset, stance) cannot be duplicated. Eden lets you copy a selection and paste it at the
cursor, preserving the group's relative layout.

| ID | Gap | Deliverable |
|----|-----|-------------|
| **ACTION-COPY-001** | `ACTION-COPY-001` `CopyUnit` | Ctrl/Cmd+C copies the current slot selection to an in-editor clipboard |
| **ACTION-COPY-001** | `ACTION-PASTE-001` `PasteUnit` | Ctrl/Cmd+V pastes the clipboard at the map cursor, preserving relative layout; pasted slots become the selection |

**Out of scope:** Cut (`ACTION-CUT-001` Ctrl+X), paste-at-original (`ACTION-PASTE-ORIG-001`
Ctrl+Shift+V), OS-clipboard / cross-tab copy, non-slot entities (vehicles / markers /
objectives aren't placeable yet — P0), backend changes, `useSelectTool` changes.

---

## Locked decisions (user confirmed)

| Decision | Choice |
|----------|--------|
| Paste position | **At the map cursor**, preserving relative layout: translate the clip so its **centroid** lands at the cursor. |
| Cursor off-map | If the cursor is `null` (mouse outside the map), fall back to a fixed **+20m / +20m nudge** from originals so copies don't perfectly overlap. |
| Scope | **Copy + Paste only, slots only** — exactly the ACTION-COPY-001 / ACTION-PASTE-001 gap rows. |
| Post-paste selection | Pasted slots become the new selection (`{ kind:'slot', ids: newIds }`) — matches Eden and the existing drop-to-place behavior. |
| Clipboard | **In-editor, in-memory** (a `useRef` on the page). Reusable across multiple pastes; overwritten on each copy. |
| Squad re-attach | Each pasted slot re-attaches to its **source squad** if it still exists, else `ensureDefaultSquad` (keeps the ORBAT export contract intact). |
| Layer | Pasted slots file into the **active Outliner layer** (`activeLayerId`), else `ensureDefaultLayer` — same as `addSlot`. |
| Undo | The whole paste is **one transaction** (one undo step). |
| Diff size | One batched ydoc action + one type + barrel export + two keydown branches. |

### Rationale

- Mirrors the proven batched-transaction actions already in `ydoc.ts` (`addSlot`,
  `moveEntities`, `removeEntities`) — `pasteSlots` is `addSlot` batched with relative offsets.
- The keydown handler in `MissionCreatorPage.tsx` already owns Undo/Redo/Space/Delete with
  the INPUT/SELECT/TEXTAREA/contentEditable guard; copy/paste are two more branches there, so
  Ctrl+C/V inside an Attributes text field stays native (the guard returns early).
- Cursor is read through a **ref** (not a keydown dependency): the cursor updates on every
  mouse move and must not re-bind the `window` keydown listener each time.

---

## Implementation specification

### a. `frontend/src/features/tactical-map/state/schema.ts`

Add a serializable clipboard-entry type near `Slot`:

```ts
/** A slot snapshot held on the editor clipboard (Ctrl+C). Plain/serializable — no ids;
 *  pasteSlots() mints fresh ids and re-resolves the squad/layer (T-056). */
export interface ClipboardSlot {
  role: string
  tag?: string
  assetId?: string
  stance: Slot['stance']
  position: { x: number; y: number; z: number; rotation: number }
  squadId: ID // source squad, re-attached on paste if it still exists
}
```

Export `ClipboardSlot` from the type block already exported by the barrel.

### b. `frontend/src/features/tactical-map/state/ydoc.ts`

Add `pasteSlots`, reusing `ensureDefaultSquad`, `ensureDefaultLayer`, `getTerrain`, and the
module `clamp` helper. One `transact()`:

```ts
/** Paste copied slots in ONE transaction (one undo step). Positions are translated so the
 *  clip's centroid lands at `anchorAt` (the map cursor); if `anchorAt` is null the clip is
 *  nudged +20m/+20m from its originals. Each new slot re-attaches to its source squad (or the
 *  default) and files into `opts.layerId` (or the default layer). Returns the new ids. */
const PASTE_NUDGE = 20
export function pasteSlots(
  md: MissionDoc,
  clip: ClipboardSlot[],
  opts?: { anchorAt?: { x: number; y: number } | null; layerId?: ID },
): ID[] {
  if (!clip.length) return []
  const terrain = getTerrain(md.meta.get('terrain') as TerrainId | undefined)
  const cx = clip.reduce((a, s) => a + s.position.x, 0) / clip.length
  const cy = clip.reduce((a, s) => a + s.position.y, 0) / clip.length
  const anchor = opts?.anchorAt
  const dx = anchor ? anchor.x - cx : PASTE_NUDGE
  const dy = anchor ? anchor.y - cy : PASTE_NUDGE
  const newIds: ID[] = []
  transact(md, () => {
    const { slots, squads, editorLayers } = md.entities
    for (const c of clip) {
      const targetSquad = squads.get(c.squadId) ? c.squadId : ensureDefaultSquad(md)
      const targetLayer = (opts?.layerId && editorLayers.get(opts.layerId))
        ? opts.layerId
        : ensureDefaultLayer(md)
      const squad = squads.get(targetSquad)!
      const slotIds = squad.get('slotIds') as ID[]
      const id = newId()
      const slot: Slot = {
        id,
        squadId: targetSquad,
        index: slotIds.length,
        role: c.role,
        ...(c.tag ? { tag: c.tag } : {}),
        ...(c.assetId ? { assetId: c.assetId } : {}),
        position: {
          x: clamp(c.position.x + dx, 0, terrain.width),
          y: clamp(c.position.y + dy, 0, terrain.height),
          z: c.position.z,
          rotation: c.position.rotation,
        },
        stance: c.stance,
        loadoutId: null,
      }
      slots.set(id, entityToYMap(slot as unknown as Record<string, unknown>))
      squad.set('slotIds', [...(squad.get('slotIds') as ID[]), id])
      const layer = editorLayers.get(targetLayer)
      if (layer) layer.set('entityIds', [...(layer.get('entityIds') as ID[]), id])
      newIds.push(id)
    }
  })
  return newIds
}
```

Import `ClipboardSlot` alongside the existing `Slot`/`EditorLayer` type import.

### c. `frontend/src/features/tactical-map/index.ts`

Export `pasteSlots` in the `ydoc` value block and `ClipboardSlot` in the `schema` type block.

### d. `frontend/src/features/mission-creator/MissionCreatorPage.tsx`

- Import `pasteSlots` and the `ClipboardSlot` type from `@/features/tactical-map`.
- Add refs:
  ```ts
  const clipboardRef = useRef<ClipboardSlot[]>([])
  const cursorRef = useRef(cursor)
  useEffect(() => { cursorRef.current = cursor }, [cursor])
  ```
- In the keydown handler, **after** the undo/redo branches and **before** the
  `useMapStore.getState()` Space/Delete block, add:
  ```ts
  if (mod && !e.altKey && !e.shiftKey && e.code === 'KeyC') {
    const { selection, slotsById } = useMapStore.getState()
    if (selection.kind !== 'slot' || !selection.ids.length) return
    const clip = selection.ids
      .map((sid) => slotsById[sid])
      .filter(Boolean)
      .map((s) => ({
        role: s.role,
        ...(s.tag ? { tag: s.tag } : {}),
        ...(s.assetId ? { assetId: s.assetId } : {}),
        stance: s.stance,
        position: { ...s.position },
        squadId: s.squadId,
      }))
    if (!clip.length) return
    clipboardRef.current = clip
    e.preventDefault()
    return
  }
  if (mod && !e.altKey && !e.shiftKey && e.code === 'KeyV') {
    if (!clipboardRef.current.length) return
    e.preventDefault()
    const { activeLayerId, setSelection } = useMapStore.getState()
    const anchorAt = cursorRef.current ? { x: cursorRef.current.x, y: cursorRef.current.y } : null
    const ids = pasteSlots(md, clipboardRef.current, { anchorAt, layerId: activeLayerId ?? undefined })
    if (ids.length) setSelection({ kind: 'slot', ids })
    return
  }
  ```
  Keydown effect deps stay `[md, undo]` (clipboard + cursor via refs; store via `getState()`).

No backend changes. No `useSelectTool` / compiler / store-schema change.

---

## Edge cases

- **Empty selection / non-slot kind** → Ctrl+C no-op (no `preventDefault`, so the browser
  copy is unaffected).
- **Focused form field** → the existing guard returns before any branch, so Ctrl+C/V edit the
  field (native text copy/paste), not the map.
- **Source squad deleted** between copy and paste → falls back to `ensureDefaultSquad` (slot
  still lands in the ORBAT graph).
- **Active layer deleted** between copy and paste → falls back to `ensureDefaultLayer`.
- **Paste lands off-terrain** → clamped to `[0,width] × [0,height]` (same clamp as
  `updateSlotPosition`); a wide clip near an edge can compress against the bound — acceptable.
- **Repeated Ctrl+V** reuses the same clipboard; each paste anchors at the **current** cursor.

---

## Files to change (checklist)

| File | Change |
|------|--------|
| `frontend/src/features/tactical-map/state/schema.ts` | Add + export `ClipboardSlot` |
| `frontend/src/features/tactical-map/state/ydoc.ts` | Add `pasteSlots` (one batched transact) |
| `frontend/src/features/tactical-map/index.ts` | Export `pasteSlots` + `ClipboardSlot` |
| `frontend/src/features/mission-creator/MissionCreatorPage.tsx` | clipboard/cursor refs + Ctrl+C / Ctrl+V keydown branches |

**No backend changes. No `useSelectTool.ts` / compiler change.**

---

## Verification

```bash
cd frontend && npm run build && npm run lint
```

### Manual test plan (`make web`, dev-login `mission_maker`, `/missions/:id/edit`)

1. Drop 2–3 units, set roles. Select them (marquee or Ctrl-click).
2. **Ctrl+C**, move the mouse over clear map, **Ctrl+V** → the group reappears centered at the
   cursor, same relative shape, and is the new selection; the Outliner lists the new slots.
3. **Ctrl+V** again at a new cursor spot → another copy; clipboard is reusable.
4. Move the mouse off-map, **Ctrl+V** → pastes at original +20m,+20m.
5. **Ctrl+Z** once removes the whole paste atomically.
6. Regression: Ctrl+C/V inside an Attributes text field does native text copy/paste; Delete,
   Space-center, additive select, drag-move all unchanged.

---

## Documentation sync (same commit — T-056)

Use [`docs/AGENT_COMMIT_CHECKLIST.md`](../../docs/AGENT_COMMIT_CHECKLIST.md).

| Doc | Change |
|-----|--------|
| **This file** | Status → **shipped** |
| [`CLAUDE.md`](../../CLAUDE.md) §Status | T-056 bullet + bump `latest feature work` line |
| [`ROADMAP.md`](ROADMAP.md) | Move T-056 → shipped (table + execution order line); add this doc to the hub; §Status "Next" leads with T-074 |
| [`eden/gap_analysis.md`](eden/gap_analysis.md) | T-056 → ✅ shipped T-056 |
| [`feature_inventory.md`](feature_inventory.md) | copy/paste (CopyUnit/PasteUnit) row → working (Trigger, Procedure, Evidence, acceptance) |
| [`agent_execution.md`](agent_execution.md) | Decisions log row **Copy/paste at cursor (T-056)** |
| [`docs/frontend/ROADMAP.md`](../../docs/frontend/ROADMAP.md) + [`frontend/docs/pages/mission-editor.md`](../../frontend/docs/pages/mission-editor.md) | Shipped/milestone row for copy-paste |
| [`docs/TAGS.md`](../../docs/TAGS.md) | T-056 row |

**Do not update:** archive stitch, Eden wiki artifacts, historical CLAUDE bullets.

---

## Git strategy

**One T-056 commit** on `main`: code + doc finalize + CLAUDE §Status. Co-Authored-By when
using AI. **Do not commit until the user asks.**

---

## Known scale limits — RESOLVED T-059

T-056 copy/paste is correct for typical mission sizes; **paste 10k+ used to freeze the browser**
(O(n²) `[...array, id]` spreads per slot in `pasteSlots`, 10k outliner leaves, 10k-id selection).
**T-059 (shipped)** fixed this: batch O(n) append in `pasteSlots`, post-paste selection cap
(`BULK_SELECT_CAP = 500`), and outliner leaf cap (`OUTLINER_LEAF_CAP = 500`) on both the Editor
Layers and ORBAT trees. Spec: [`t059_bulk_paste_operations.md`](t059_bulk_paste_operations.md).

---

## Related

- Prior: [t055_asset_browser_search.md](t055_asset_browser_search.md), [t053_additive_select.md](t053_additive_select.md)
- Next: … → ~~**T-066**~~ ✅ worker compile → **T-067.0** active — [`t067_spatial_chunks.md`](t067_spatial_chunks.md); Eden **T-068+**
