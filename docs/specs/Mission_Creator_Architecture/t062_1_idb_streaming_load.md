# T-062.1 — Chunked IDB slot restore (fix 0→300k load jump)

**Status:** **shipped** — build + lint + tsc clean; manual verify @ ~360k: migration once, then determinate restoring progress with smooth done/total (no 0→300k jump on 2nd+ load)  
**Git tag on ship:** **T-062.1** (`4ad27fe`)  
**Authority:** [MC ROADMAP](ROADMAP.md) §Map performance · [agent_execution.md](agent_execution.md) §ACTIVE SLICE · [t060_1_scale_load_save_completion.md](t060_1_scale_load_save_completion.md) §Load interpretation · [t062_2_editor_session_persistence.md](t062_2_editor_session_persistence.md)

**Prerequisites:** T-062.2 shipped (`693e227`). Repro mission: `70a36667-612f-40c5-ad56-3fb8e0613a17` (~360k slots).

---

## Problem (pre-T-062.1)

On return visits, `useMissionDoc` created `IndexeddbPersistence('tbd-mission-${id}', md.doc)`. y-indexeddb replayed the entire persisted Y.Doc in **one synchronous `Y.applyUpdate`** before `synced`:

| Symptom @ ~360k | Cause |
|-----------------|-------|
| Count stuck at **0** for ~30–60s | `slots.size` cannot tick during the block |
| Jump **0 → ~300k** in one frame | Replay completes atomically |
| Indeterminate restoring bar | T-060.1.1 poll had no `total` |

T-060.1.1 fixed the **stuck blank 0%** overlay (label + pulse). T-062.1 fixes **incremental restore** and **determinate progress**.

**Not the problem:** server hydrate (`hydrateMissionDocWithProgress`) and local flush (`docToSnapshotWithProgress`) — already chunked @ 5k.

---

## Goal

**v2 persistence:** slots durably stored in a **chunked `idb` store**; restore incrementally (5k/chunk, `yieldToUi`, `INIT_ORIGIN`). Overlay shows smooth **`done / total`** on **2nd+ load** after one-time v1→v2 migration.

**Product call:** total wall time ~30s–1min @ 360k still acceptable; win = **perceived progress** + no dead-zone UX. Verified good enough in manual pass.

---

## Locked architecture

**Do not keep slots in y-indexeddb on v2** — `IndexeddbPersistence` syncs the whole Y.Doc; slot edits re-bloat IDB and restore is all-at-once.

| Path | Durability | Boot |
|------|------------|------|
| **v1 legacy** | `IndexeddbPersistence` `tbd-mission-${id}` | Blocking replay → migrate once → delete legacy DB |
| **v2** | `idb` `tbd-mission-persist` (meta + slot chunks) | Chunked + yielding restore |
| **fresh** | none until first `LOCAL_ORIGIN` edit | Seed defaults only |

**Runtime:** Y.Doc + `Y.UndoManager` unchanged. Load/migration writes use `INIT_ORIGIN` (not undo-tracked). User edits use `LOCAL_ORIGIN` → debounced v2 persist.

### v2 idb layout

Single DB **`tbd-mission-persist`**, two object stores:

| Store | Key | Contents |
|-------|-----|----------|
| `meta` | `meta-${missionId}` | `{ schemaVersion: 2, meta, factions, squads, editorLayers, objectives, vehicles, markers, loadouts, items }` |
| `slots` | `slots-${missionId}` | `{ schemaVersion: 2, slotCount, chunks: { index, slots: Slot[] }[] }` — **5000 slots/chunk** (`PERSIST_CHUNK_SIZE`) |

**v2 signal:** `hasV2Persist(missionId)` — slot-store record present.

---

## Shipped implementation

### NEW `frontend/src/features/mission-creator/persistence/`

| File | Role |
|------|------|
| `missionPersistSchema.ts` | `openPersistDb`, `hasV2Persist`, `detectLegacyV1`, `deleteV2`, key helpers, `PERSIST_CHUNK_SIZE = 5000` |
| `slotChunkStore.ts` | `loadSlotsWithProgress`, `saveSlotsFromDoc`, debounced + serialized `saveSlotsFromDocDebounced` / `flushSlots` |
| `missionMetaStore.ts` | `loadMissionMetaIntoDoc`, `saveMissionMetaFromDoc`, debounced + `flushMeta` |
| `migrateLegacyToV2.ts` | One-time idempotent snapshot ("Migrating local save…") |

### Wiring

| File | Change |
|------|--------|
| `ydoc.ts` | Exported `entityToYMap` for persistence reuse |
| `useMissionDoc.ts` | `restoringPhase(done, total?)` — determinate when `total` known; boot branches v2 / legacy / fresh in cancellable async IIFE; legacy-only rAF/interval poll; `cancelled` through chunk loops (StrictMode) |
| `useMissionEditor.ts` | Debounced v2 persist on `LOCAL_ORIGIN` once `docStatus === 'ready'`; flush on `visibilitychange→hidden`, `pagehide`, unmount; `docAlive` / `isDocCancelled` guard |

### Boot flow (execution order)

1. `beginBulkSync` + `bindStoreToDoc`
2. **v2:** `loadMissionMetaIntoDoc` → `loadSlotsWithProgress` → `seedDefaults` — **no** `IndexeddbPersistence`
3. **legacy:** `IndexeddbPersistence` + poll → `synced` → `seedDefaults` → `migrateLegacyToV2` → destroy persistence + `indexedDB.deleteDatabase(legacy)`
4. **fresh:** `seedDefaults` only
5. `onSynced` reconcile (cold GET / warm skip T-062.2) → `endBulkSync` → `docStatus: ready`

---

## Deliberate behavior (documented)

| Case | Behavior |
|------|----------|
| **Server-adopted mission, no user edit** | Not cached to v2 until first `LOCAL_ORIGIN` edit — cold load re-fetches server. Migration covers existing edited-mission population. |
| **SPA navigate-away within ~2s debounce** | Doc destroyed before flush; `isCancelled` aborts save — no corruption; edits in that window may be lost. Tab hide / close / F5 covered while doc alive. |
| **Undo** | Session edits only (`LOCAL_ORIGIN`); restore/migration uses `INIT_ORIGIN` |
| **Warm return (T-062.2)** | Unchanged — skips GET when session marker + local content |

---

## Acceptance (T-062.1 — shipped)

| Check @ ~360k | Result |
|---------------|--------|
| First load after upgrade | Migration ("Migrating local save…"); may block once |
| 2nd+ load | Restoring count ticks smoothly; **no 0→300k jump** |
| Progress bar | Determinate % during v2 restoring |
| Cold load | restoring → download → apply → local flush |
| Warm return | Skips GET; chunked restore |
| F5 after edit | Edits persist in `tbd-mission-persist` |
| Legacy DB | `tbd-mission-${id}` deleted post-migration |
| build + lint + tsc | **Clean** |

---

## Out of scope (T-062.1)

- ~~T-062.1.1 Save orbat dedup~~ → **shipped** ([`t062_1_1_batch_save.md`](t062_1_1_batch_save.md))
- ~~T-063 spatial index~~ ✅ — spec [`t063_spatial_index.md`](t063_spatial_index.md)
- **T-064** ✅ virtualized outliner. **T-065** ✅ cluster/LOD. **T-066** ✅ worker compile. **T-067** ✅ — [`t067_spatial_chunks.md`](t067_spatial_chunks.md). **Eden T-068+**
- ≤10s @ 1M (T-066 worker stretch)
- Post-hydrate v2 write without user edit

---

## After T-062.1

- ~~**T-062.1.1** Save orbat dedup~~ ✅ — spec [`t062_1_1_batch_save.md`](t062_1_1_batch_save.md)
- ~~**T-063** spatial index~~ ✅ — spec [`t063_spatial_index.md`](t063_spatial_index.md)
- **T-064** ✅ virtualized outliner. **T-065** ✅ cluster/LOD. **T-066** ✅ worker compile. **T-067** ✅ — [`t067_spatial_chunks.md`](t067_spatial_chunks.md). **Eden T-068+**

---

## Documentation sync (Cursor — done)

`agent_execution.md`, `ROADMAP.md`, `CLAUDE.md`, `mission-editor.md`, `feature_inventory.md` (PERF-IDB-001, DATA-IDB-001, PERF-LOAD-001), `TAGS.md`, `docs/frontend/ROADMAP.md`, `docs/AGENT_COMMIT_CHECKLIST.md`, `docs/backend/architecture.md`, `engineering_plan.md`, spec footers (`t060_1`, `t060_fast_initial_load`, `t062`, `t062_2`, `t056`–`t059`, `t061`, `t070`).
