**Status:** live

#### PERF-BULK-PASTE-001 — Bulk paste/delete at scale (T-059)

| Field | Value |
|-------|-------|
| **Domain** | PERF |
| **Goal** | Paste **10k** slots without browser hard-freeze; one undo step; pan ≥55 fps after |
| **Trigger** | Ctrl/Cmd+V with large clipboard; bulk delete |
| **Preconditions** | T-056 copy/paste working; T-058 OBJ readout for verification |
| **Procedure** | Batch `slotIds`/`entityIds` append in `pasteSlots`; cap selection ids >500; outliner virtualization (T-064 supersedes T-059 leaf cap) |
| **Postconditions** | OBJ correct; tab responsive; undo reverts entire paste |
| **Inputs** | `ClipboardSlot[]`, cursor anchor, active layer |
| **Outputs** | New slot ids (selection capped when bulk) |
| **Edge cases** | Paste 10k → selection cleared not 10k ids; large folders scroll via virtual outliner (T-064) |
| **Acceptance** | `- [x] Paste 10k no hard freeze` `- [x] OBJ correct` `- [x] Pan ≥55 fps after` `- [x] Undo one step` `- [x] 360k @ 100+ fps pan validated` |
| **Eden parity** | Eden:ACTION-PASTE-001 (bulk scale) |
| **Status** | working |
| **Ticket** | T-059 |
| **Evidence** | `state/ydoc.ts` (`pasteSlots`), `MissionCreatorPage.tsx`, `VirtualOutliner.tsx`, `flattenOutliner.ts` |

---

#### PERF-OUTLINER-001 — Virtualized outliner @ scale (T-064)

| Field | Value |
|-------|-------|
| **Domain** | PERF |
| **Goal** | Scroll through **100k–360k+** slot rows in ORBAT + Editor Layers without DOM explosion or tab freeze |
| **Trigger** | Expand a large squad/layer folder; scroll the left sidebar |
| **Preconditions** | T-060 deferred sidebar until `docStatus === 'ready'`; trees populated on first mount |
| **Procedure** | `@tanstack/react-virtual` in `VirtualOutliner.tsx`; segment-index flatten (`flattenOutliner.ts`); `virtualSlotIds` on folders with ≥50 slots (`VIRTUAL_SLOT_THRESHOLD`); callback-ref `scrollEl` (T-064.1) |
| **Postconditions** | Only ~viewport rows mounted; DnD/rename/select/dbl-click preserved |
| **Inputs** | ORBAT + Editor Layers tree models from Zustand |
| **Outputs** | Virtual row list in single `LeftSidebar` scroll container |
| **Edge cases** | Blank first frame if scroll element null — fixed T-064.1 via `scrollElement` state not RefObject |
| **Acceptance** | `- [x] Outliner visible on first paint @ ~367k` `- [x] Scroll 367k virtual rows` `- [x] No tab freeze` `- [x] DnD/reparent/root-drop/rename/delete` `- [x] Map pan/pick unchanged` |
| **Eden parity** | Eden outliner scroll (scale) |
| **Status** | working |
| **Ticket** | T-064 |
| **Evidence** | `VirtualOutliner.tsx`, `flattenOutliner.ts`, `TreeRow.tsx`, `LeftSidebar.tsx`, `EditorLayersSection.tsx`, `OrbatSection.tsx` |

---

#### PERF-CLUSTER-001 — Cluster / LOD @ extreme zoom (T-065)

| Field | Value |
|-------|-------|
| **Domain** | PERF |
| **Goal** | At extreme zoom-out on mega missions (>500 slots), show pan-stable cluster discs; at normal edit zoom keep full detail IconLayer @ ~160 fps |
| **Trigger** | Zoom to ≤ `ZOOM_CLUSTER_MAX` (-4) on mission with > `CLUSTER_SLOT_THRESHOLD` (500) slots |
| **Preconditions** | T-061 pan-stable icon cache; T-063 spatial pick; slotClusterIndex kept in sync via `slotIconCache` mutators |
| **Procedure** | `supercluster` in `slotClusterIndex.ts`; `getClusterMarkers` full-terrain pan-stable cache (T-065.2); `useClusterIconLayer` memoized on `markersVersion` + `iconCacheVersion`; `clusterMode` gates detail vs selected-only in `useIconLayer` |
| **Postconditions** | Default open zoom -2 = all rings; cluster drill-in via `pickClusterAt` → `flyTo` +1; selected slots visible in cluster band |
| **Inputs** | `slotIconCache` dense positions; Deck zoom; terrain bounds |
| **Outputs** | Cluster disc `IconLayer` (count-sized discs, no TextLayer) |
| **Edge cases** | ≤500 slots → never cluster; T-065.1 viewport-bbox pan stutter — fixed T-065.2; further cluster perf tuning deferred unless regression |
| **Acceptance** | `- [x] Detail @ -2 ~160 fps @ 367k` `- [x] Pan-stable cluster path (T-065.2)` `- [x] Git tag T-065` |
| **Eden parity** | Group icons when zoomed out (geo clusters v1 only) |
| **Status** | working |
| **Ticket** | T-065 |
| **Evidence** | `slotClusterIndex.ts`, `useClusterIconLayer.ts`, `constants.ts`, `TacticalMap.tsx`, `useSelectTool.ts`, `slotIconCache.ts` |

---

#### PERF-WORKER-001 — Compiler worker offload (T-066 + T-066.1)

| Field | Value |
|-------|-------|
| **Domain** | PERF |
| **Goal** | Run `compileMissionWithProgress` + `buildVersionBlob` off main thread @ 367k+; stretch ≤10 s @ 1M |
| **Trigger** | Save Version or Export on large mission |
| **Preconditions** | T-060 compile progress + T-060.1.2 blob streaming; **T-066.1:** `pickMapSnapshot(getState())` before Comlink RPC |
| **Procedure** | `compiler.worker.ts` + `compilerClient.ts` (Comlink); `useMissionEditor` Save + Export; `terminateCompiler` on unmount |
| **Postconditions** | Same `MissionPayload` / POST contract; Save progress phases unchanged |
| **Inputs** | `MapSnapshot` via `pickMapSnapshot(useMapStore.getState())` — **never raw getState()** |
| **Outputs** | `MissionPayload` or version POST `Blob` |
| **Edge cases** | Raw `getState()` → DataCloneError 25 (T-066.1 fixed); Export `JSON.stringify` still on main thread |
| **Acceptance** | `- [x] Save 201 @ ~367k` `- [x] pickMapSnapshot hotfix` `- [x] Git tag T-066` |
| **Eden parity** | n/a (infra) |
| **Status** | shipped |
| **Ticket** | T-066 |
| **Evidence** | `compiler.worker.ts`, `compilerClient.ts`, `compile.ts`, `useMissionEditor.ts`, `pickMapSnapshot` in `useMapStore.ts`; spec [`t066_worker_compile.md`](/documentation_v2/tickets/specs/t066_worker_compile.md) |

---

#### PERF-CHUNK-001 — Spatial chunks / bulk-paste scale (T-067)

| Field | Value |
|-------|-------|
| **Domain** | PERF |
| **Goal** | Bulk paste O(k) not O(n) snapshot; chunk scaffolding for 1M+ lazy RAM / GPU cull |
| **Trigger** | Ctrl+V bulk paste; (future) viewport crosses chunk @ 1M |
| **Preconditions** | T-062 incremental bindings; T-066 worker save unchanged |
| **Procedure** | **Shipped:** `slot-add-bulk` in `incPatchPlan` → `_patchAddSlotsBulk`; dormant `chunkBuckets` in `slotIconCache`. **Render:** `getBaseIcons()` (CPU cull reverted T-067.0.1). **Follow-on (`idea`):** **T-111** lazy RAM; **T-112** GPU `DataFilterExtension` |
| **Postconditions** | Paste ≤10k avoids full snapshot; pan ~160 fps @ 367k zoom -2 |
| **Inputs** | Added slot ids from Y.Doc txn; (future) viewport bounds |
| **Outputs** | O(k) store + cache updates on bulk paste |
| **Edge cases** | Structural squad/layer paste → full snapshot fallback (unchanged) |
| **Acceptance** | `- [x]` pan ~160 fps @ 367k `- [x]` build/lint `- [x]` 6k paste `- [x]` pick/drag/cluster `- [~]` Save 201 (no T-067 save-path change; repro mission needs local DB seed) |
| **Eden parity** | n/a (infra) |
| **Status** | **shipped** (bulk paste + scaffolding; CPU cull deferred) |
| **Ticket** | T-067 |
| **Evidence** | `incPatchPlan.ts`, `useMapStore._patchAddSlotsBulk`, `spatialChunks.ts`, `slotIconCache.ts` chunk buckets, `useIconLayer.ts` → `getBaseIcons()`; spec [`t067_spatial_chunks.md`](/documentation_v2/tickets/specs/t067_spatial_chunks.md); follow-ons [`T-111`/`T-112`](../../TICKET_BRAINSTORM.md#scale) |

---

#### PERF-LOAD-001 — Fast initial load / hydrate gate (T-060 + T-060.1 + T-060.1.1)

| Field | Value |
|-------|-------|
| **Domain** | PERF |
| **Goal** | Open **10k–1M** missions with **determinate progress bar**; coalesce boot snapshots; **≤10 s ideal @ 1M** (stretch → **T-066** worker) |
| **Trigger** | Navigate to `/missions/:id/edit` |
| **Preconditions** | Y.Doc persisted in IndexedDB (possibly 100k–360k+ slots) |
| **Procedure** | Bulk-sync window; `docStatus` gate; **four-phase** overlay: **restoring** (v2 chunked `loadSlotsWithProgress` T-062.1, or legacy y-indexeddb poll T-060.1.1) → download → apply → local flush; `docToSnapshotWithProgress` + `hydrateMissionDocWithProgress` + `onDownloadProgress`; `endBulkSync` async after hydrate; LeftSidebar deferred until ready |
| **Postconditions** | Map interactive; OBJ correct; pan ≥55 fps |
| **Inputs** | v2 `tbd-mission-persist` (`idb`) and/or legacy y-indexeddb; optional server `json_payload` |
| **Outputs** | `docStatus: ready`; `loadProgress` with determinate % (v2 restoring has `done/total`) |
| **Edge cases** | Legacy v1: one-time blocking replay + migrate. v2: smooth restoring @ ~360k. Server-adopted mission not cached to v2 until first `LOCAL_ORIGIN` edit. Warm return skips GET (T-062.2) |
| **Acceptance** | `- [x] Overlay + bulk-sync (T-060)` `- [x] Determinate % + chunked snapshot/hydrate (T-060.1)` `- [x] Hydrate inside bulk window (T-060.1)` `- [x] Restoring phase (T-060.1.1 legacy)` `- [x] v2 chunked IDB restore (T-062.1)` `- [ ] Pan regression clean (manual)` |
| **Eden parity** | — |
| **Status** | **shipped (T-060 + T-062.1)** — v2 determinate restore @ ~360k |
| **Ticket** | T-060 |
| **Evidence** | `useMissionDoc.ts`, `persistence/*`, `bindings.ts`, `ydoc.ts`, `MissionCreatorPage.tsx`, `t062_1_idb_streaming_load.md` |

---

#### PERF-SAVE-001 — Fast Save Version + progress (T-060 + T-060.1 + T-060.1.2 + T-060.1.3 + T-060.1.4)

| Field | Value |
|-------|-------|
| **Domain** | PERF |
| **Goal** | **Save Version** with **progress bar**; compile + POST without hard-freeze at **50k+**; **API accepts payloads >> 1 MB** (256 MB route limit — T-060); **≤10 s ideal @ 1M** |
| **Trigger** | User clicks Save → Save Version in TopCommandStrip |
| **Preconditions** | Valid mission UUID; dirty or explicit save |
| **Procedure** | `compileMissionWithProgress` → **`preparing`** (✅ chunked `buildVersionBlob`) → **`uploading`** (✅ Blob POST; **E3b:** auto direct `:8080` in dev when body >1 MB) → 256 MB cap, `timeout: 600_000` |
| **Postconditions** | Version 201; dirty cleared |
| **Inputs** | `useMapStore` snapshot |
| **Outputs** | POST body; progress UI |
| **Edge cases** | Mid-upload `ERR_NETWORK` @ ~4% / ~135 MB → **FIXED T-060.1.4**; payload >256 MB → pre-gate 413; **T-062.1.1:** Save omits duplicate `orbat[]` (editor-only POST); Go derives ORBAT for events |
| **Acceptance** | `- [x] E1/E2/E3b` `- [x] SZ + Save dialog (T-060.1.3)` `- [x] browser Save @ ~367k → 201` `- [x] T-062.1.1 IT: editor-only → event ORBAT` `- [x] Manual Save @ ~367k: ~94.8 MB estimated (~33% smaller vs ~141 MB)` |
| **Status** | **shipped** — T-060..T-060.1.4 + **T-062.1.1** orbat dedup |
| **Ticket** | T-060 |
| **Evidence** | `useMissionEditor.ts`, `compiler/compile.ts`, `lib/missionSize.ts`, `internal/services/mission_payload.go`, `internal/handlers/missions_orbat_integration_test.go`, `t062_1_1_batch_save.md` |

---

#### PERF-DRAG-001 — Drag-move preview @ 360k (T-061)

| Field | Value |
|-------|-------|
| **Domain** | PERF |
| **Goal** | **Drag-move** of selected slots acceptable @ ~360k (motion, pickup, release); preview transient (Y.Doc commit on pointer-up) |
| **Trigger** | Left-drag a selected slot icon (or selection) on the map |
| **Preconditions** | T-057 pan/cursor path shipped; mission has many slots (stress @ 360k) |
| **Procedure** | T-061.0: dual IconLayer + split drag state + rAF delta. T-061.0.1: `slotIconCache` O(k) + bindings slot fast path (`fastSlotPatchIds`) |
| **Postconditions** | Icons follow pointer smoothly; commit on release; undo reverts |
| **Inputs** | Pointer gesture in `useSelectTool` |
| **Outputs** | Transient store preview; Y.Doc update on release |
| **Edge cases** | 1 vs N selected; asset palette drop lag → **resolved T-062** (slot-add path). Release: possible single dropped frame (two cache bumps) — **deferred** |
| **Acceptance** | `- [x] Drag motion 1 slot @ 360k ≥55 fps` `- [x] Drag motion ~10 selected @ 360k ≥55 fps` `- [x] Pickup/release good enough (not perfect)` `- [x] build + lint clean` `- [x] Full regression sweep documented` |
| **Eden parity** | Eden:XFORM-MOVE-001 |
| **Status** | **shipped (good enough)** — spec [`t061_drag_move_hotfix.md`](/documentation_v2/tickets/specs/t061_drag_move_hotfix.md) |
| **Ticket** | T-061 |
| **Evidence** | `slotIconCache.ts`, `useMapStore.ts`, `bindings.ts`, `useIconLayer.ts`, `useSelectTool.ts`, `selectors.ts`, `TacticalMap.tsx` |

---

#### PERF-PICK-001 — Spatial index for pick/marquee @ 360k (T-063)

| Field | Value |
|-------|-------|
| **Domain** | PERF |
| **Goal** | **Click, dbl-click, drag-start, marquee** pick without scanning all ~367k icons |
| **Trigger** | LMB click, dbl-click, drag-start, marquee release on map |
| **Preconditions** | Mission loaded with many slots; pan/drag paths already fast (T-057, T-061) |
| **Procedure** | rbush R-tree in `slotSpatialIndex.ts`; maintained alongside `slotIconCache`; `pickNearest` / `pickRect` replace `deck.pickObject` / `pickObjects`; `slot-icons` `pickable: false` |
| **Postconditions** | Selection unchanged vs Eden contract; no GPU pick pass on IconLayer |
| **Inputs** | Screen px + viewport (point picks); world bbox (marquee) |
| **Outputs** | `selection.ids[]`; Attributes via dbl-click |
| **Edge cases** | Overlapping icons → nearest to click; Ctrl/Cmd toggle (T-053); drag exclude no tree change |
| **Acceptance** | `- [x] Click @ 367k instant` `- [x] Marquee no multi-s freeze` `- [x] Dbl-click Attributes` `- [x] Pan/drag unchanged` `- [x] build + lint clean` |
| **Status** | **shipped** — spec [`t063_spatial_index.md`](/documentation_v2/tickets/specs/t063_spatial_index.md) |
| **Ticket** | T-063 |
| **Evidence** | `slotSpatialIndex.ts`, `slotIconCache.ts`, `useSelectTool.ts`, `TacticalMap.tsx`, `useIconLayer.ts` |

---

#### PERF-BIND-001 — Incremental bindings @ 360k (T-062)

| Field | Value |
|-------|-------|
| **Domain** | PERF |
| **Goal** | Patch Zustand O(k) on everyday Y.Doc edits @ ~360k instead of full `docToSnapshot(n)` |
| **Trigger** | Asset drop, Delete, title/env edit, outliner layer rename/reparent/move |
| **Preconditions** | T-061 drag path shipped; mission @ scale (stress @ 360k) |
| **Procedure** | **T-062.0:** `incPatchPlan.classifyTransaction` → `PatchPlan` (`slot-fields`, `slot-add`, `slot-remove`, `meta`, `editor-layers`) → store patch methods + `slotIconCache.append`/`remove`. **T-062.0.1:** batched `removeEntities('slots')`; `slotCount`/`slotsRevision`; `REMOVE_PATCH_CAP` 10_000 |
| **Postconditions** | Store mirror updated without `e.slots.toJSON()` over all slots for classified txns |
| **Inputs** | Y.Doc observer events |
| **Outputs** | Incremental Zustand patches; `iconCacheVersion` bumps |
| **Edge cases** | Bulk paste / `addEditorLayer` / empty-doc bootstrap / `removeEditorLayer` → full snapshot fallback. Undo large multi-delete → full snapshot (verified OK @ 6k undo). IDB 0→300k jump → **fixed T-062.1** (v2 chunked restore). `_patchSlots` drag release still O(n) spread — deferred mega opt |
| **Acceptance** | `- [x] Asset drop instant @ 360k` `- [x] Delete 150/4000 no crash` `- [x] Drag not regressed` `- [x] Undo 6000 delete OK` `- [x] build + lint clean` |
| **Eden parity** | Eden:XFORM-PLACE-001 (drop), Eden:DELETE-001 |
| **Status** | **shipped** — spec [`t062_incremental_bindings.md`](/documentation_v2/tickets/specs/t062_incremental_bindings.md) |
| **Ticket** | T-062 |
| **Evidence** | `incPatchPlan.ts`, `bindings.ts`, `useMapStore.ts`, `slotIconCache.ts`, `ydoc.ts`, `BottomToolbelt.tsx`, `EditorLayersSection.tsx`, `OrbatSection.tsx` |

#### PERF-SESSION-001 — Editor session / alt-tab (T-062.2)

| Field | Value |
|-------|-------|
| **Domain** | PERF |
| **Goal** | Alt-tab back to editor without automatic full load overlay; warm same-tab return skips multi-MB server GET |
| **Trigger** | Extended background tab (dev: Vite WS disconnect); same-tab reload with warm `sessionStorage` marker |
| **Procedure** | Dev: `viteReloadGuard` blocks `vite:beforeFullReload` on `/missions/:id/edit`. `editorSession.ts` marks ready → on reboot, `onSynced` skips GET when warm + `hasLocalContent(md)`. `yieldToUi` + restore poll visibility-aware |
| **Edge cases** | Warm path trusts local v2 store — remote server changes undetected until cold load. New tab = cold. `dirty` UI flag resets on reload (data in IDB). Undo stack session-only |
| **Acceptance** | `- [x] Alt-tab 30+ min → no overlay (Firefox dev @ ~360k)` `- [x] Edits preserved` `- [x] Cold load unchanged` |
| **Status** | **shipped** — spec [`t062_2_editor_session_persistence.md`](/documentation_v2/tickets/specs/t062_2_editor_session_persistence.md) |
| **Ticket** | T-062.2 |
| **Evidence** | `viteReloadGuard.ts`, `editorSession.ts`, `useMissionEditor.ts`, `useMissionDoc.ts`, `yieldToUi.ts` |

#### PERF-IDB-001 — Chunked IDB slot restore (T-062.1)

| Field | Value |
|-------|-------|
| **Domain** | PERF |
| **Goal** | Restore ~360k slots from local persistence with **determinate** restoring progress (no 0→300k jump on 2nd+ load) |
| **Trigger** | Navigate to `/missions/:id/edit` when v2 `tbd-mission-persist` exists (or legacy v1 → migrate once) |
| **Preconditions** | T-062.2 warm path; T-060 bulk-sync window |
| **Procedure** | v2: `loadMissionMetaIntoDoc` → `loadSlotsWithProgress` (5k/chunk, `INIT_ORIGIN`, `yieldToUi`) — **no** y-indexeddb. Legacy: y-indexeddb replay → `migrateLegacyToV2` → delete `tbd-mission-${id}`. Writes: debounced meta + slot save on `LOCAL_ORIGIN`; flush on tab hide/pagehide |
| **Postconditions** | Y.Doc populated; overlay shows smooth `done/total` during v2 restoring |
| **Edge cases** | Server-adopted mission not v2-cached until first edit. SPA navigate-away within ~2s debounce may drop last edits. `docAlive` / `isCancelled` guards prevent corrupt writes on teardown |
| **Acceptance** | `- [x] Migration once` `- [x] 2nd+ load smooth progress @ ~360k` `- [x] Legacy DB deleted` `- [x] build/lint/tsc clean` |
| **Status** | **shipped** — spec [`t062_1_idb_streaming_load.md`](/documentation_v2/tickets/specs/t062_1_idb_streaming_load.md) |
| **Ticket** | T-062.1 |
| **Evidence** | `persistence/*`, `useMissionDoc.ts`, `useMissionEditor.ts`, `ydoc.ts` (`entityToYMap`) |

---

