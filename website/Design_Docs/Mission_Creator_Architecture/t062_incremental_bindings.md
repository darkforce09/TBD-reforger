# T-062 — Incremental bindings @ 360k (interactive edits + bulk delete)

**Status:** **shipped** — T-062.0 + T-062.0.1 implemented; build + lint clean; manual verify @ ~360k (delete 4k, undo 6k, asset drop, drag)  
**Git tag on ship:** **T-062** (single commit: T-062.0 classifier + T-062.0.1 bulk delete)  
**Authority:** [MC ROADMAP](ROADMAP.md) §Map performance · [agent_execution.md](agent_execution.md) §ACTIVE SLICE · [t061_drag_move_hotfix.md](t061_drag_move_hotfix.md)

**Prerequisites:** T-061 shipped (`35d6336`). Repro mission: `70a36667-612f-40c5-ad56-3fb8e0613a17` (~360k slots).

---

## Goal

Stop everyday Y.Doc edits from flushing the entire Zustand mirror via `docToSnapshot(md)` → `e.slots.toJSON()` over ~360k maps. T-061 fixed **drag-move** only; T-062 generalizes to **asset drop**, **delete**, **meta/env**, and **outliner layer** edits, plus **bulk delete** up to `REMOVE_PATCH_CAP`.

**Original symptom (pre-T-062 @ ~360k):**

| Operation | Before | After T-062 |
|-----------|--------|-------------|
| Asset palette drop | Multi-second hitch | Instant (slot-add path) |
| Delete 1–10 | ~10 fps collapse | Incremental slot-remove |
| Delete ~150 | Heavy lag (O(n·k) ydoc + O(n) spread) | Usable (T-062.0.1) |
| Delete ~4000 | Tab crash (`FAST_PATCH_CAP` → full snapshot) | Incremental (T-062.0.1) |
| Drag-move | Good (T-061) | Unchanged |
| Title / environment | Full snapshot | meta path |
| Outliner rename / move-to-layer | Full snapshot | editor-layers path |

**Manual verify (2026-06, repro mission):** delete **4000** OK; asset drop OK; drag OK; **undo 6000** delete OK (uses full-snapshot fallback on undo txn — acceptable UX).

---

## Shipped implementation

### T-062.0 — Transaction classifier + O(k) store patches

| File | Change |
|------|--------|
| **NEW** `state/incPatchPlan.ts` | `classifyTransaction(md, events, txn)` → `PatchPlan \| null`; firing-order-independent (whole `txn.changed`); `FAST_PATCH_CAP = 512` for slot-fields + slot-add |
| `state/bindings.ts` | Observer → `classifyTransaction` → `applyPlan`; `lastFastTxn` dedupe; fallback `queueMicrotask(flush)` unchanged |
| `state/useMapStore.ts` | `_patchMeta`, `_patchEditorLayers`, `_patchAddSlot`, `_patchRemoveSlots` |
| `state/slotIconCache.ts` | `append(slots, selection)`, `remove(ids)` — dense-only; never touch `excluded` during drag |

**PatchPlan kinds:** `slot-fields` (T-061 drag/attributes), `slot-add` (k=1), `slot-remove`, `meta`, `editor-layers`.

### T-062.0.1 — Bulk delete + consumer decoupling

| File | Change |
|------|--------|
| `state/ydoc.ts` | Batched `removeEntities('slots')` when `ids.length > 1`: `Set(ids)` → one `slotIds` filter per affected squad → one `entityIds` filter per layer → `slots.delete` each id. O(n·k) → O(n + k). |
| `state/useMapStore.ts` | `slotCount`, `slotsRevision`; in-place `_patchAddSlot` / `_patchRemoveSlots` (no O(n) spread); `_applySnapshot` sets `slotCount` once at boot |
| `state/incPatchPlan.ts` | `REMOVE_PATCH_CAP = 10_000` for slot-remove only |
| `BottomToolbelt.tsx` | OBJ reads `slotCount` directly |
| `EditorLayersSection.tsx`, `OrbatSection.tsx` | `slotsRevision` in `useMemo` deps (in-place dict mutation) |

**`_patchSlots` unchanged** — still spreads `slotsById` on field edits / drag release so Attributes + outliner role/tag labels refresh; drag O(n) spread deferred per ROADMAP §Deferred mega optimizations.

---

## Classifier fallbacks (full `docToSnapshot` — by design)

| Transaction | Why null |
|-------------|----------|
| Bulk paste (`pasteSlots`, multi-add) | `added.length > 1` |
| `addEditorLayer` (new folder) | `layersStructural` only |
| Empty-doc bootstrap `addSlot` | `forbiddenStructural` (faction/squad/layer create) |
| `removeEditorLayer` cascade | `layersStructural` + many slot deletes |
| hydrate / load / `clearAll` | bulk window or whole-doc |
| Undo large multi-delete | Re-adds k slots → `added.length !== 1` → full snapshot (verified OK @ 6k undo) |
| Mixed / ambiguous txn | Safety first |

---

## Out of scope (T-062.1 / T-062.1.1 — not T-062.0)

- ~~y-indexeddb **streaming** / fixing load **0→300k** IDB jump~~ → **shipped T-062.1** ([`t062_1_idb_streaming_load.md`](t062_1_idb_streaming_load.md))
- ~~Backend orbat dedup on Save~~ → **shipped T-062.1.1** ([`t062_1_1_batch_save.md`](t062_1_1_batch_save.md))
- Incremental **undo multi-add** (slot-add cap stays k=1)
- `_patchSlots` / drag-release O(n) spread elimination (T-061.1 / mega opts)
- ~~T-063 spatial index~~ ✅ — [`t063_spatial_index.md`](t063_spatial_index.md); ~~T-064 virtualized outliner~~ ✅ — [`t064_virtualized_outliner.md`](t064_virtualized_outliner.md)

---

## Acceptance (T-062 — shipped)

| Check @ ~360k | Result |
|---------------|--------|
| Asset drop | **Pass** — instant |
| Delete 10 / 150 / ~4000 | **Pass** — no crash |
| Drag-move | **Pass** — not regressed |
| Undo 6000 delete | **Pass** — correct state (full snapshot on undo txn) |
| Pan idle 100+ fps | **Pass** (unchanged) |
| build + lint | **Clean** |

---

## After T-062

- ~~**T-062.2** editor session~~ ✅ — spec [`t062_2_editor_session_persistence.md`](t062_2_editor_session_persistence.md)
- ~~**T-062.1** chunked IDB load~~ ✅ — spec [`t062_1_idb_streaming_load.md`](t062_1_idb_streaming_load.md)
- ~~**T-062.1.1** Save orbat dedup~~ ✅ — spec [`t062_1_1_batch_save.md`](t062_1_1_batch_save.md)
- ~~**T-063** spatial index~~ ✅ — spec [`t063_spatial_index.md`](t063_spatial_index.md)
- **T-064** ✅ virtualized outliner. **T-065** ✅ cluster/LOD. **T-066** ✅ worker compile. **T-067** ✅ — [`t067_spatial_chunks.md`](t067_spatial_chunks.md). **Eden T-068+**
- **Eden T-068+** after scale milestones

---

## Documentation sync (Cursor — done on ship)

`agent_execution.md`, `ROADMAP.md`, `feature_inventory.md` PERF-BIND-001, `mission-editor.md`, `TAGS.md`, `CLAUDE.md`, spec footers, `docs/frontend/ROADMAP.md`, `docs/AGENT_COMMIT_CHECKLIST.md`.
