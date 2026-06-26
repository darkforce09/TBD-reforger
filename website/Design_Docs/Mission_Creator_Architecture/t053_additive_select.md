# T-053 — Ctrl/Cmd+LMB additive (toggle) select

**Status:** shipped (T-053)  
**Git tag on ship:** T-053  
**Authority:** [MC ROADMAP](ROADMAP.md) · [eden/gap_analysis.md](eden/gap_analysis.md) (`SEL-MOD-001`) · [feature_inventory.md](feature_inventory.md) SEL-MOD-001

---

## Goal

Wire **modifier-click additive selection** in the Mission Creator. Multi-select already
exists via marquee box-drag (`SEL-MAP-003`), but a single LMB click on a unit **replaces**
the whole selection (`SEL-MAP-001`) — so building or trimming a multi-selection requires a
fresh marquee each time. Eden lets you **Ctrl-click** units to add/remove them one at a time.

| ID | Gap | Deliverable |
|----|-----|-------------|
| **SEL-MOD-001** | `SEL-MOD-001` | Ctrl/Cmd+LMB on a slot toggles it in/out of the current selection |

**Out of scope:** Shift range-select, Ctrl+A select-all (`KEY-SELALL-001`), copy/paste
(T-056), Ctrl+drag add-and-move, any `useSelectTool` gesture change, backend changes.

---

## Locked decisions (user confirmed)

| Decision | Choice |
|----------|--------|
| Modifier | **Ctrl or Cmd (meta) only.** Shift stays **unbound** (reserved for future range-select) |
| Semantics | **Toggle** — modifier+click adds an unselected slot, removes an already-selected one |
| Empty selection | Removing the last id → selection becomes `{ kind:'none', ids:[] }` |
| Modifier + empty-click | **Preserve** the current selection (no-op). Only a *plain* empty click deselects (`SEL-MAP-002` unchanged) |
| Where | **Deck `onClick` in `TacticalMap.tsx`** — plain clicks already fall through `useSelectTool` to Deck's `onClick`; no gesture-machine change |
| Diff size | **Minimal** — one file, augment the existing `onClick` callback |

### Rationale

- `useSelectTool.ts` deliberately owns only **drags**; sub-threshold clicks fall through
  to Deck's canvas `onClick`. The deck-level `onClick` second arg is a `MjolnirGestureEvent`
  whose `srcEvent` is the underlying `MouseEvent` carrying `ctrlKey`/`metaKey`. So the
  modifier is available exactly where single-select already lives — no second code path.
- Selection state is read with `useMapStore.getState().selection` (the same imperative
  read `useSelectTool.ts` already uses), so no new store subscription or schema change
  (`Selection.ids[]` already holds an array).

---

## Implementation specification

**File:** [`frontend/src/features/tactical-map/TacticalMap.tsx`](../../frontend/src/features/tactical-map/TacticalMap.tsx)

Augment the existing `onClick` (`useCallback`, ~line 56):

1. Add the event param and derive the modifier:
   ```ts
   const onClick = useCallback(
     (info: PickingInfo, event?: { srcEvent?: { ctrlKey?: boolean; metaKey?: boolean } }) => {
       const src = event?.srcEvent
       const additive = !!(src && (src.ctrlKey || src.metaKey))
   ```
   (Inline structural type avoids a `mjolnir.js` import; the real arg is a
   `MjolnirGestureEvent`.)

2. **Icon click + additive** → toggle (place **before** the single-select + dbl-click block):
   ```ts
   if (additive) {
     const sel = useMapStore.getState().selection
     const cur = sel.kind === 'slot' ? sel.ids : []
     const next = cur.includes(id) ? cur.filter((x) => x !== id) : [...cur, id]
     setSelection(next.length ? { kind: 'slot', ids: next } : { kind: 'none', ids: [] })
     lastClick.current = null   // additive click never arms the dbl-click timer
     return
   }
   ```

3. **Empty click + additive** → preserve: in the empty branch, `return` early when
   `additive` (skip the `setSelection({ kind:'none' })`).

The non-additive paths (`SEL-MAP-001` single replace, `SEL-MAP-004` dbl-click,
`SEL-MAP-002` empty deselect) are untouched.

---

## Edge cases

- A Ctrl-built selection has `ids.length > 1`, so `SEL-MAP-004` dbl-click-to-attributes
  stays suppressed (its `selection.ids.length <= 1` precondition) — no regression.
- Ctrl+drag on a slot still routes through `useSelectTool` move mode (unchanged); only the
  sub-threshold *click* is additive.
- Marquee box-select still **replaces** the selection (Eden parity) — additive marquee is
  out of scope.

---

## Files to change (checklist)

| File | Change |
|------|--------|
| `frontend/src/features/tactical-map/TacticalMap.tsx` | `onClick` reads Ctrl/Cmd from the deck event; toggle on icon; preserve on empty |

**No backend changes. No store/schema change. No `useSelectTool.ts` change.**

---

## Verification

```bash
cd frontend && npm run build && npm run lint
```

### Manual test plan (`make web`, dev-login `mission_maker`, `/missions/:id/edit`)

1. Click unit A → only A selected (regression: plain click still replaces).
2. Ctrl-click B, then C → A+B+C all highlighted (map icons + outliner rows sync).
3. Ctrl-click B again → B drops out; A+C remain.
4. Ctrl-click A then C (last) off → selection empties to none.
5. Ctrl-click empty map → selection preserved; plain click empty → cleared.
6. Marquee, drag-move, Delete, Space-center, dbl-click attributes all unchanged.

---

## Documentation sync (same commit — T-053)

Use [`docs/AGENT_COMMIT_CHECKLIST.md`](../../docs/AGENT_COMMIT_CHECKLIST.md).

| Doc | Change |
|-----|--------|
| **This file** | Status → **shipped** |
| [`CLAUDE.md`](../../CLAUDE.md) §Status | T-053 bullet + bump `latest feature work` line |
| [`feature_inventory.md`](feature_inventory.md) | SEL-MOD-001 → **working** (Trigger, Procedure, Evidence, acceptance); SEL-SYNC-001 stays partial |
| [`agent_execution.md`](agent_execution.md) | Decisions log row **Additive select (T-053)** |
| [`ROADMAP.md`](ROADMAP.md) | Move T-053 → shipped; §Status "Next" leads with T-055 asset search |
| [`eden/gap_analysis.md`](eden/gap_analysis.md) | SEL-MOD-001 + T-053 → ✅ shipped T-053 |

**Do not update:** archive stitch, Eden wiki artifacts, historical CLAUDE bullets.

---

## Git strategy

**One T-053 commit** on `main`: code + doc finalize + CLAUDE §Status. Co-Authored-By when
using AI. **Do not commit until the user asks.**

---

## Related

- Prior: [t052_undo_shortcuts.md](t052_undo_shortcuts.md)
- Next Eden backlog: see [`docs/TICKET_LEAD.md`](../../docs/TICKET_LEAD.md) (T-068+).
