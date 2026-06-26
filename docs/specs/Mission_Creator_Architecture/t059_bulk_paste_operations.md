# T-059 — Bulk paste/delete operations at scale

**Status:** shipped (T-059) — batch append + selection/outliner caps
**Git tag on ship:** T-059
**Authority:** [MC ROADMAP](ROADMAP.md) §Map performance · [agent_execution.md](agent_execution.md) §ACTIVE SLICE

**Prerequisites:** **T-057** shipped (pan/zoom ≥55 fps @ 200+; validated **100+ fps @ 10k**). **T-058** shipped (OBJ/SEL toolbelt telemetry).

**Observed blocker (2026-06):** Pan/zoom at **10k objects = 100+ FPS**. **Paste 10k = browser hard-freeze.** Copy/paste at 100k+ would crash without this slice.

---

## Goal

Make **bulk copy/paste and delete** usable at **10k+** objects without freezing the tab — prerequisite for the **1M–10M** north star (Arma-scale maps).

**Acceptance:**
- Paste **10k** slots: tab stays **responsive** (no multi-second hard freeze); completes in **≤5s** or shows chunked progress (`Pasting… N/M`).
- **One undo step** reverts the entire paste.
- **OBJ** readout correct after paste; pan/zoom still **≥55 fps** (T-057 regression guard).
- Copy 10k + paste once still works (Eden T-056 relative-layout contract preserved).
- `cd frontend && npm run build && npm run lint` clean.

**Out of scope (later tags):** fast load + save (T-060), typed-array IconLayer (T-061), incremental bindings (~~T-062~~ ✅ shipped), virtualization (T-064).

---

## Root cause (confirmed in code)

[`ydoc.ts`](../../frontend/src/features/tactical-map/state/ydoc.ts) `pasteSlots` — per slot in one `transact`:

```ts
squad.set('slotIds', [...(squad.get('slotIds') as ID[]), id])   // O(n²) over 10k
layer.set('entityIds', [...(layer.get('entityIds') as ID[]), id]) // O(n²) over 10k
```

Then [`bindings.ts`](../../frontend/src/features/tactical-map/state/bindings.ts): `observeDeep` → `docToSnapshot()` → full `slots.toJSON()` of **all** slots → `_applySnapshot` → [`selectSlotIcons`](../../frontend/src/features/tactical-map/state/selectors.ts) maps every slot → [`EditorLayersSection`](../../frontend/src/features/mission-creator/layout/LeftOutliner/EditorLayersSection.tsx) `buildTree` renders **every** `entityId` as a tree leaf → `MissionCreatorPage` sets `selection.ids` to **all 10k new ids**.

---

## Locked decisions

| Decision | Choice |
|----------|--------|
| Batch append | In `pasteSlots` / `addSlot`: accumulate `slotIds` and `entityIds` with `.push()` on local arrays; **one** `set` per array at end of paste (O(n) not O(n²)) |
| Post-paste selection | If pasted count **> 500**: **do not** put all new ids in `selection.ids`. Clear selection (`none`) or keep prior — **recommend `none`**; OBJ updates, user box-selects if needed |
| Outliner cap | If layer `entityIds.length > 500` (same threshold): `buildTree` shows folder label `"Layer (N entities)"` **without** N slot leaf nodes |
| Chunked paste | If single transact still blocks >~100ms after batch fix: paste in chunks of **1000** slots per `requestAnimationFrame`, show toolbelt progress `Pasting… 3000/10000`. Prefer **one Y.Doc transaction** wrapping all chunks for one undo step |
| Bulk snapshot | Optional `_bulkMode` in `bindings.ts`: suppress `docToSnapshot` flush until bulk op ends → single flush (**T-060 code** bulk window; **T-060.1** adds determinate progress + hydrate timing fix) |
| Copy | No change required for T-059 unless profiling shows copy 10k is slow (ClipboardSlot[] snapshot is acceptable) |
| Delete 10k | Audit `removeEntities` — apply same batch patterns if spread loops exist |

---

## Implementation specification

### a. `state/ydoc.ts` — `pasteSlots` (and `addSlot` if same pattern)

```ts
// Inside transact, before loop:
const squadSlotIds = [...(squad.get('slotIds') as ID[])]
const layerEntityIds = layer ? [...(layer.get('entityIds') as ID[])] : null

for (const c of clip) {
  // ... create slot, slots.set(id, ...)
  squadSlotIds.push(id)
  layerEntityIds?.push(id)
  newIds.push(id)
}

squad.set('slotIds', squadSlotIds)
if (layer && layerEntityIds) layer.set('entityIds', layerEntityIds)
```

### b. `MissionCreatorPage.tsx` — post-paste selection cap

```ts
const BULK_SELECT_CAP = 500
const ids = pasteSlots(...)
if (ids.length) {
  setSel(ids.length <= BULK_SELECT_CAP ? { kind: 'slot', ids } : { kind: 'none', ids: [] })
}
```

### c. `EditorLayersSection.tsx` — `buildTree` leaf cap

When `layer.entityIds.length > OUTLINER_LEAF_CAP` (500): render folder node only with subtitle/count; **no** per-slot children in TreeView.

### d. Optional: chunked paste + progress

- Export `pasteSlotsChunked(md, clip, opts, onProgress)` or loop in page with slice + rAF.
- Toolbelt transient `pasteProgress: { done, total } | null` in store (like cursor — only toolbelt subscribes).

### e. Optional: `bindings.ts` bulk coalesce

```ts
let bulkDepth = 0
export function runBulk(md, fn) { bulkDepth++; try { fn() } finally { bulkDepth--; if (!bulkDepth) flush() } }
```

---

## Verification

1. `npm run build && npm run lint` — clean.
2. Mission with **10k** slots (paste-double or dev helper): **OBJ 10000**.
3. Select all → Ctrl+C → Ctrl+V: tab **does not freeze**; OBJ → 20000 (or 10000 if replace scenario); progress bar if chunked.
4. Undo once → back to prior count.
5. Pan middle-drag: **≥55 fps** after bulk paste.
6. Outliner: layer folder shows count, not 10k expandable leaves.

**Live validation (2026-06 — post ship):**
- Repeat **6k-object paste** loops: smooth, no major freeze.
- **360k objects** on map: **100+ fps** pan/zoom (FpsCounter).
- Chunked paste (spec d/e) confirmed **not needed**.

---

## Documentation sync (same commit — T-059)

| Doc | Update |
|-----|--------|
| [`CLAUDE.md`](../../CLAUDE.md) §Status | T-059 bullet; validation notes; Next → **T-060.1.1** |
| [`ROADMAP.md`](ROADMAP.md) | Scale table T-059 ✅; Next → **T-060.1.1** |
| [`t056_copy_paste.md`](t056_copy_paste.md) | Note: bulk scale limits addressed T-059 |
| [`feature_inventory.md`](feature_inventory.md) | ACTION-PASTE bulk row or amend KEY-COPY-001 |
| [`agent_execution.md`](agent_execution.md) | Decisions log; ACTIVE SLICE → **T-060.1.1** |
| [`docs/TAGS.md`](../../docs/TAGS.md) | T-059 shipped row |
| [`mission-editor.md`](../../frontend/docs/pages/mission-editor.md) | M3.14 milestone; PERF-002 bulk paste |

---

## Shipped (T-059)

**Landed (the two confirmed freeze causes):**

- **a. Batch append in `pasteSlots`** — the per-slot `[...spread, id]` appends to `slotIds` /
  `entityIds` (O(n²)) are gone. The loop accumulates into local arrays — a `Map<squadId, ID[]>`
  (seeded once per squad from its live `slotIds`, so per-slot squad re-targeting still works) plus
  one array per target layer — and writes each Y.Map **once** after the loop. `index` is derived
  from the accumulating array length (no per-iteration Y.Map re-read). `addSlot` left unchanged
  (single slot, not O(n²)); `removeEntities` left unchanged (not the paste hotpath).
- **b. Post-paste selection cap** — `MissionCreatorPage` Ctrl+V branch: `BULK_SELECT_CAP = 500`.
  A paste of ≤500 selects its result (T-056 behavior); above it, selection is cleared to `none`
  (OBJ still updates from `slotsById`; the user box-selects if needed). No 10k ids in `selection.ids`.
- **c. Outliner leaf cap (both trees)** — `OUTLINER_LEAF_CAP = 500` (exported from
  `EditorLayersSection`). `buildTree`: a layer with `entityIds.length > 500` renders as a folder
  with a `"(N units)"` count label and **no** slot leaves (child folders still recurse).
  `OrbatSection.buildOrbat`: a squad with `slotIds.length > 500` renders `"(N slots)"` with no
  slot leaves. **Spec deviation (noted):** item 3 named only EditorLayersSection, but the ORBAT
  tree renders the same slots and would still freeze, so the cap covers both — required for the
  no-freeze acceptance.

**Not implemented (conditional items d/e — not needed):** chunked paste + `Pasting… N/M` progress
and the `bindings._bulkMode` coalesce were specced as fallbacks **only if** a single batched
transact still blocked. The batch fix (a) removes the O(n²); a single transact of 10k slots plus
the existing one-flush-per-transaction coalescing in `bindings.ts` is expected to complete within
the ≤5s acceptance without a hard freeze. If a manual 10k paste is observed to still stall, revisit
items d/e (design preserved above).

## After T-059

**T-060.1.4:** Fix mid-upload @ ~135 MB — ✅ (`b1fd25a`). **T-061** ✅. **T-062** ✅. **T-062.2** ✅. **T-062.1** ✅. **T-062.1.1** ✅. **T-063** ✅. **T-064** ✅ virtualized outliner. **T-065** ✅ cluster/LOD. **T-066** ✅ worker compile. **T-067** ✅ spatial chunks — [`t067_spatial_chunks.md`](t067_spatial_chunks.md). **Eden T-068+**.
