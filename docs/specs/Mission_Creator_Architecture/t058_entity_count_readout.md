# T-058 — Toolbelt entity count readout (OBJ total + SEL selected)

**Status:** shipped (T-058)
**Git tag on ship:** T-058
**Authority:** [MC ROADMAP](ROADMAP.md) §Map performance · [agent_execution.md](agent_execution.md) §ACTIVE SLICE · [feature_inventory.md](feature_inventory.md)

**Prerequisite:** **T-057 shipped** (`7adc345`) — pan/zoom perf hotfix; toolbelt cursor already store-backed so OBJ/SEL updates must **not** reintroduce page-level re-renders.

---

## Goal

Show **how many slot entities are on the map** and **how many are selected** in the bottom toolbelt — scale-test telemetry for the **T-059..T-067** performance program (1M–10M north star).

**Acceptance:**
- Bottom toolbelt displays **OBJ** `{total}` = count of entries in `slotsById` (all placed map slots).
- **SEL** `{n}` = `selection.ids.length` when `selection.kind === 'slot'`, else `0`.
- JetBrains Mono / `tabular-nums`, same glass toolbelt row as CUR/SEL X/Y/Z (T-049/T-050/T-057).
- Updates when slots are added/removed/pasted/deleted or selection changes; **does not** update on cursor move alone (T-057 cursor channel is separate).
- Marquee multi-select, Ctrl-toggle, copy-paste, delete — SEL count tracks correctly.
- `cd frontend && npm run build && npm run lint` clean.

**Scope:** slots only (`slotsById`). Vehicles/markers join the count in later **T-069**/**T-070** slices — do not invent a multi-entity aggregator yet.

---

## Visual layout (locked)

Extend the existing `BottomToolbelt` HudBar — **right of the X/Y/Z block**, separated by a vertical divider (same as between tools and coords):

```
[ Select | Ruler | LoS ]  |  CUR  X 4157  Y 3646  Z 0  |  OBJ 3842  SEL 12
```

| Label | Meaning | Source |
|-------|---------|--------|
| **OBJ** | Total placed slots on the map | `selectSlotCount(slotsById)` |
| **SEL** | Currently selected slots | `selection.kind === 'slot' ? selection.ids.length : 0` |

Use `text-code-md font-mono tabular-nums text-on-surface-variant` with values in `text-on-surface` — match the CUR/SEL coord styling. No comma separators (raw integer for paste-doubling math).

**Note:** The coord prefix **SEL** (single-slot X/Y/Z mode) and the count label **SEL** (selection count) share the same abbreviation by design — they appear in different columns and never conflict visually.

---

## Locked decisions

| Decision | Choice |
|----------|--------|
| Entity type | **Slots** in `slotsById` only — the only map-placed entity type today |
| Count source | **T-062:** `useMapStore((s) => s.slotCount)` — O(1) maintained by bindings patches. **T-058 original:** memoized `selectSlotCount(slotsById)` in `state/selectors.ts` |
| Selection | Reuse existing `selection` slice — no new store field |
| Placement | `BottomToolbelt.tsx` only — do **not** duplicate on `FpsCounter` |
| Performance | Subscribe inside already-memoized `BottomToolbelt`; do **not** lift counts to `MissionCreatorPage` state |
| Large numbers | Plain integer display (supports 100000+ without layout break — no fixed-width padding required) |

---

## Implementation specification

### a. `frontend/src/features/tactical-map/state/selectors.ts`

Add after the `memo1` helper, before `selectSlotIcons`:

```ts
/** O(n) slot count; memoized on slotsById ref (bindings replace the dict object per snapshot). */
export const selectSlotCount = memo1((slotsById: Record<ID, Slot>): number => {
  let n = 0
  for (const _ in slotsById) n++
  return n
})
```

### b. `frontend/src/features/tactical-map/index.ts`

Re-export: `selectSlotCount` alongside `selectSlotIcons`.

### c. `frontend/src/features/mission-creator/layout/BottomToolbelt.tsx`

- Import `selectSlotCount` from `@/features/tactical-map`.
- Subscribe:
  ```ts
  const slotsById = useMapStore((s) => s.slotsById)
  const totalSlots = selectSlotCount(slotsById)
  const selectedCount = useMapStore((s) =>
    s.selection.kind === 'slot' ? s.selection.ids.length : 0,
  )
  ```
- After the X/Y/Z `<div>`, add divider + OBJ/SEL block (see Visual layout).
- `title="Placed slots on map / current selection"` on the OBJ/SEL container for accessibility.

### d. What is intentionally unchanged

- T-057 cursor path (`useMapStore.cursor`, rAF-throttled) — untouched.
- Deck layers, picking, pan coalescing — untouched.
- Y.Doc schema, compiler, backend — untouched.
- `FpsCounter` stays separate (bottom-right).

---

## Verification

1. `cd frontend && npm run build && npm run lint` — clean.
2. `make web`; dev-login as `mission_maker`; open `/missions/:id/edit`.
3. **OBJ = 0, SEL = 0** on empty mission.
4. Drop one slot → **OBJ 1**; click it → **SEL 1**; click empty map → **SEL 0**.
5. Marquee-select 5 slots → **SEL 5**; Delete → **OBJ** decreases, **SEL 0**.
6. Ctrl+C/V paste doubling: OBJ doubles each paste cycle; SEL matches pasted selection after paste.
7. Pan/zoom with 200+ slots — **FpsCounter stays green** (T-057 regression guard); OBJ/SEL still correct.

---

## Documentation sync (same commit — T-058)

| Doc | Update |
|-----|--------|
| [`CLAUDE.md`](../../CLAUDE.md) §Status | T-058 bullet; bump `latest feature work`; Next → T-059 bulk paste |
| [`ROADMAP.md`](ROADMAP.md) | DONE — T-058 section; scale table T-058 row → ✅; Next → T-059 |
| [`feature_inventory.md`](feature_inventory.md) | New **BOTTOM-OBJCOUNT-001** row → working |
| [`agent_execution.md`](agent_execution.md) | Decisions log row; ACTIVE SLICE → T-059 bulk paste |
| [`ux_spec.md`](ux_spec.md) | Interaction table row for OBJ/SEL readout |
| [`docs/TAGS.md`](../../docs/TAGS.md) | T-058 row → shipped |
| [`docs/frontend/ROADMAP.md`](../../docs/frontend/ROADMAP.md) | Recently shipped row + recommended next → **T-060.1** acceptance |
| [`frontend/docs/pages/mission-editor.md`](../../frontend/docs/pages/mission-editor.md) | Element inventory row 5 + M3.13 milestone |

**One T-058 commit** on `main`: code + doc finalize + CLAUDE §Status. Co-Authored-By when applicable.

---

## After T-058

**T-059 (bulk paste/delete):** fix O(n²) spreads, cap selection/outliner after bulk paste, optional chunked paste — **10k paste without hard freeze**. Use **OBJ** to confirm totals. Spec: [`t059_bulk_paste_operations.md`](t059_bulk_paste_operations.md).

**T-060.1.4** Fix mid-upload @ ~135 MB — ✅ (`b1fd25a`). **T-061** ✅. **T-062** ✅. **T-062.2** ✅. **T-062.1** ✅. **T-062.1.1** ✅. **T-063** ✅. **T-064** ✅ virtualized outliner. **T-065** ✅ cluster/LOD. **T-066** ✅ worker compile. **T-067** ✅ spatial chunks — [`t067_spatial_chunks.md`](t067_spatial_chunks.md). **Eden T-068+**. **Eden T-068+.**
