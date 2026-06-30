# T-052 — Undo/redo keyboard shortcuts

**Status:** shipped (T-052)  
**Git tag on ship:** T-052  
**Authority:** [MC ROADMAP](ROADMAP.md) · [eden/gap_analysis.md](eden/gap_analysis.md) (`TOP-UNDO-001 / KEY-UNDO-001`) · [feature_inventory.md](feature_inventory.md) TOP-UNDO-001 / TOP-REDO-001 / KEY-UNDO-001

---

## Goal

Wire **keyboard undo/redo** in the Mission Creator. Toolbar Undo/Redo buttons already work via
`Y.UndoManager`; Eden expects **Ctrl+Z / Ctrl+Y** (and macOS **Cmd+Z / Cmd+Shift+Z**).

| ID | Gap | Deliverable |
|----|-----|-------------|
| **TOP-UNDO-001** | `TOP-UNDO-001` / `KEY-UNDO-001` | Host-level Ctrl/Cmd+Z and Ctrl/Cmd+Y (or Shift+Z) call existing `UndoController` |
| **TOOLBAR-UNDO-001** | Eden toolbar parity | Keyboard matches toolbar behavior |

**Out of scope:** Copy/paste (T-056), new undo stack, undo while typing in modal fields, backend changes.

---

## Locked decisions (user confirmed)

| Decision | Choice |
|----------|--------|
| Undo stack | **Reuse** `useMissionDoc` → `createUndoManager` — no second stack |
| macOS | **Cmd+Z** undo; **Cmd+Shift+Z** redo |
| Windows/Linux | **Ctrl+Z** undo; **Ctrl+Y** redo **and** **Ctrl+Shift+Z** redo (either works) |
| Focus guard | **Skip** when `INPUT`, `SELECT`, `TEXTAREA`, or `contentEditable` — same pattern as Space/Delete in `MissionCreatorPage` |
| When disabled | Call `undo()` / `redo()` only when `canUndo()` / `canRedo()`; still `preventDefault` when shortcut matched and handled |
| Toolbar | **Do not break** existing Undo/Redo buttons in `TopCommandStrip` |
| Diff size | **Minimal** — `MissionCreatorPage` keydown + `useMissionDoc` StrictMode lifecycle fix |
| StrictMode lifecycle | **`instanceKey` bump** on effect teardown (one-shot `recreatedRef`) so dev `<StrictMode>` gets a fresh `md`+`UndoController` after setup→cleanup→setup |

### Rationale — why not a global hook

- Space and Delete already live on `MissionCreatorPage` with the focus guard — keep keyboard host wiring in one place.
- `UndoController` is already passed to `TopCommandStrip`; the page has access via `editor.undo` from `useMissionEditor`.

---

## Root cause audit (pre-ship)

| Surface | Pre-T-052 | Shipped (T-052) |
|---------|-----------|-----------------|
| `TopCommandStrip.tsx` | Undo/Redo buttons call `undo.undo()` / `undo.redo()` | Working when stack has items |
| `MissionCreatorPage.tsx` keydown | Space/Delete only | **Cmd/Ctrl+Z/Y** wired |
| `useMissionDoc.ts` | `useMemo` doc + undo; StrictMode cleanup calls `undo.destroy()` without remounting `useMemo` | **`instanceKey` + one-shot bump** → fresh `UndoController` after StrictMode teardown |
| `feature_inventory.md` KEY-UNDO-001 | `partial` — buttons only | **working** — keyboard + lifecycle fix |

**Evidence:** [`undo.ts`](../../../apps/website/frontend/src/features/tactical-map/state/undo.ts) — `trackedOrigins: LOCAL_ORIGIN`; `addSlot` / `moveEntities` use `transact()`. Dev app runs `<StrictMode>` in [`main.tsx`](../../../apps/website/frontend/src/main.tsx).

**UX note:** Undo only covers **session edits** after load (`LOCAL_ORIGIN`). Units already in IndexedDB / hydrated from the server are not undoable — expected.

---

## Implementation specification

### 1. Extend page keydown handler

**File:** [`frontend/src/features/mission-creator/MissionCreatorPage.tsx`](../../../apps/website/frontend/src/features/mission-creator/MissionCreatorPage.tsx)

Inside the existing `useEffect` keydown listener (after the focus guard):

```ts
const mod = e.metaKey || e.ctrlKey
if (mod && e.code === 'KeyZ' && !e.altKey) {
  e.preventDefault()
  if (e.shiftKey) {
    if (undo.canRedo()) undo.redo()
  } else {
    if (undo.canUndo()) undo.undo()
  }
  return
}
if (mod && (e.code === 'KeyY' || (e.code === 'KeyZ' && e.shiftKey)) && !e.altKey) {
  // Ctrl+Y redo (Windows/Linux); Cmd+Shift+Z already handled above on macOS
  if (!e.shiftKey && e.code === 'KeyY') {
    e.preventDefault()
    if (undo.canRedo()) undo.redo()
  }
  return
}
```

**Simpler equivalent (recommended):**

- `mod && KeyZ && !shift` → undo if `canUndo()`
- `mod && ((KeyZ && shift) || KeyY)` → redo if `canRedo()`
- Always `preventDefault()` when the shortcut matches (even if stack empty — stops browser "undo typing" bleed on the page chrome).

Add `undo` to the `useEffect` dependency array.

### 2. StrictMode-safe Y.Doc lifecycle

**File:** [`frontend/src/features/mission-creator/hooks/useMissionDoc.ts`](../../../apps/website/frontend/src/features/mission-creator/hooks/useMissionDoc.ts)

- `missionKey` + `instanceKey` state; `useMemo` deps `[missionKey, instanceKey]`.
- Effect cleanup: destroy bindings/persistence/doc/undo, then bump `instanceKey` once per mount (one-shot `recreatedRef`) so StrictMode's setup→cleanup→setup gets a **live** `UndoController`.
- Without this, keyboard shortcuts and toolbar buttons stay grayed in dev (`canUndo()` always false).

### 3. Optional — toolbar tooltips

**File:** [`frontend/src/features/mission-creator/layout/TopCommandStrip.tsx`](../../../apps/website/frontend/src/features/mission-creator/layout/TopCommandStrip.tsx)

If trivial: append ` (⌘Z)` / ` (Ctrl+Z)` to Undo button `title` via `navigator.platform` or a small `isMac` helper. **Optional — skip if it adds noise.**

---

## Files to change (checklist)

| File | Change |
|------|--------|
| `frontend/src/features/mission-creator/MissionCreatorPage.tsx` | modifier+Z/Y → `undo.undo()` / `undo.redo()`; TEXTAREA in focus guard |
| `frontend/src/features/mission-creator/hooks/useMissionDoc.ts` | `instanceKey` StrictMode lifecycle fix |
| `TopCommandStrip.tsx` | optional tooltip hints (skipped) |

**No backend changes.**

---

## Verification

```bash
cd frontend && npm run build && npm run lint
```

### Manual test plan

1. Open `/missions/:id/edit` (dev-login `mission_maker`).
2. Drag a slot → **Ctrl+Z** (or **Cmd+Z**) → move reverts; toolbar Undo disabled state matches.
3. **Ctrl+Y** or **Ctrl+Shift+Z** (or **Cmd+Shift+Z**) → move returns.
4. Focus a number field in Attributes modal → **Ctrl+Z** must **not** revert map edits (browser/native field behavior).
5. With empty undo stack → **Ctrl+Z** does nothing harmful; no console errors.
6. **StrictMode / dev:** after page load, drop or drag a unit → toolbar Undo **enables** (not permanently grayed).

---

## Documentation sync (same commit — T-052)

Use [`docs/AGENT_COMMIT_CHECKLIST.md`](../../website/AGENT_COMMIT_CHECKLIST.md).

| Doc | Change |
|-----|--------|
| **This file** | Status → **shipped** |
| [`CLAUDE.md`](../../../CLAUDE.md) §Status | T-052 bullet + bump `latest feature work` line |
| [`docs/TAGS.md`](../../website/TAGS.md) | T-052 row (planned → shipped) |
| [`docs/website/frontend/pages/mission-editor.md`](../../../website/frontend/pages/mission-editor.md) | Element #2 note keyboard undo/redo; **Behavior → Keyboard** subsection; M3.7 milestone `[x]` |
| [`feature_inventory.md`](feature_inventory.md) | TOP-UNDO-001 / TOP-REDO-001 edge cases + acceptance; KEY-UNDO-001 → **working** |
| [`agent_execution.md`](agent_execution.md) | Decisions log row **Undo keyboard (T-052)** |
| [`ROADMAP.md`](ROADMAP.md) | Move **PLANNED T-052** → **DONE T-052**; line ~80 note keyboard undo |
| [`docs/website/frontend/ROADMAP.md`](../../website/frontend/ROADMAP.md) | Recently shipped T-052 |
| [`eden/gap_analysis.md`](eden/gap_analysis.md) | TOP-UNDO-001 → ✅ shipped T-052; TOOLBAR-UNDO-001 parity → match or partial→match |
| [`t050_cursor_z_readout.md`](t050_cursor_z_readout.md) | Related: prior shipped slice (T-050) |

**Do not update:** archive stitch, Eden wiki artifacts, historical T-049 CLAUDE bullet.

### Target copy — mission-editor.md Keyboard subsection

Add under **Behavior**:

```markdown
### Keyboard (host — `/missions/:id/edit`)
| Shortcut | Action |
|----------|--------|
| Space | Center camera on selection |
| Delete / Backspace | Remove selected slots (undoable) |
| Cmd/Ctrl+Z | Undo last edit |
| Cmd/Ctrl+Shift+Z or Ctrl+Y | Redo |

Skipped when focus is in an input, select, or contentEditable field.
```

---

## Git strategy

**One T-052 commit** on `main`: code + doc finalize + CLAUDE §Status. Co-Authored-By when using AI.

T-050 must already be on `main` before T-052 (it is).

---

## Claude Code handoff prompt

```
Read CLAUDE.md and docs/AGENT_COMMIT_CHECKLIST.md first.

Implement T-052 per docs/specs/Mission_Creator_Architecture/t052_undo_shortcuts.md.
Docs are PRE-STAGED in the spec §Documentation sync — FINALIZE on ship (status → shipped).

LOCKED: see spec Locked decisions — reuse UndoController, macOS Cmd+Z/Shift+Z, Windows Ctrl+Z/Y/Shift+Z, focus guard, minimal diff.

Verify: npm run build && npm run lint + manual test plan in spec.
Commit on main as T-052 with Co-Authored-By when I ask. Do not commit until I say.
```

---

## Related

- Prior: [t050_cursor_z_readout.md](t050_cursor_z_readout.md)
- Next Eden backlog: see [`docs/TICKET_LEAD.md`](../../TICKET_LEAD.md) (T-068+).
- Deferred: **T-051** — optional `PATCH` title sync ([t049 amendment](t049_terrain_title_position.md))
