# Mission Editor (2D Canvas)

## Status

`in-progress`

## Summary

- **What:** Full-bleed Deck.gl tactical map editor for placing entities, ORBAT layers, and compiling mission versions.
- **Why:** Mission makers author `json_payload` for the mission library and event ORBAT materialization.
- **Route:** `/missions/:id/edit`
- **Live source:** `apps/website/frontend/src/features/mission-creator/` (wrapper) + `apps/website/frontend/src/features/tactical-map/` (engine); lazy route in `apps/website/frontend/src/router.tsx`
- **Stitch reference:** none
- **Min role:** `mission_maker` (owner or admin)
- **Blueprint ref:** [Mission Creator ROADMAP](../../../specs/Mission_Creator_Architecture/ROADMAP.md), [feature inventory](../../../specs/Mission_Creator_Architecture/feature_inventory.md)

## Element Inventory

| # | Element | Type | Text / Content | Purpose | Data source |
|---|---------|------|----------------|---------|-------------|
| 1 | Map viewport | canvas | Tactical grid + entities | Primary editor surface | Y.Doc + Deck.gl |
| 2 | Top strip | bar | Title, undo/redo, time/weather scrubber, settings, Export | Command chrome; Undo/Redo buttons + **keyboard shortcuts** (Cmd/Ctrl+Z, Cmd/Ctrl+Shift+Z, Ctrl+Y) (T-052) | Mission metadata |
| 3 | Left dock | panel | Editor Layers (+ ORBAT tree until **T-071** removes duplicate) | Outliner / drop target | `editorLayers` map |
| 4 | Right dock | panel | Asset Palette tabs + **Asset Browser search** | Drag assets to map; **search filters the Factions tree by name** (T-055) | **`GET /api/v1/registry`** → `useRegistry` + `buildCatalogTree` (T-068.3); 8 NATO characters @ current seed |
| 5 | Toolbelt | bar | Select, Ruler, LoS + X/Y/Z readout + **OBJ/SEL counts** (T-058) | Map tools; CUR/SEL coords; **OBJ** = total slots, **SEL** = selected count; **FpsCounter dev-only** (T-122 T12) | Tool state + `slotsById` + selection |
| 6 | Inspector | panel | Slot fields on double-click | `AttributesModal` opens from **map dbl-click**, **ORBAT** slot row, or **Editor Layers** slot row (T-054); **Transform X/Y/Z/rotation editable** (T-049); **Arsenal tab** — 4 gear dropdowns + Download loadout JSON for character slots (T-068.4) | Selected slot + `GET /registry` |
| 7 | Save Version | button | POST new semver | Immutable versions API | `useMissionEditor` |
| 8 | Export | button | Download mod envelope | Compiler | `compiler/exportSchema.ts` |

## Behavior

### Primary flow
1. Navigate from Mission Library **+ New Mission** dialog (T-048) or dossier **OPEN IN MISSION CREATOR** → `/missions/:id/edit`.
2. `useMissionDoc` boot: **v2** chunked restore from `tbd-mission-persist` (`idb` meta + 5k slot chunks — T-062.1), or **legacy** one-time y-indexeddb replay → migrate; `useMissionEditor` reconciles with the server on **cold** load (`GET /missions/:id` + conflict prompt when both local and server have content). **T-062.2:** warm same-tab return (`editorSession` + `sessionStorage`) skips the multi-MB GET when local content exists; dev `viteReloadGuard` blocks Vite HMR full reload on alt-tab. The mission **row** (`title`, `terrain`, time/weather) hydrates into `meta` on cold load — including new missions whose `json_payload` is `{}` (T-049). Terrain drives viewport bounds. **T-060:** bulk-sync window + `docStatus` loading overlay (restoring → download → apply → local on cold; restoring → local only on warm); LeftSidebar deferred until ready.
3. Author places entities via palette drop, moves selection on map, organizes layers in outliner.
4. **Save Version** → `POST /missions/:id/versions` with compiled `json_payload`.
5. **Export** downloads camelCase mod envelope without saving.

### States
- **Chromeless:** No platform Sidebar/TopNav (`fullBleed` + `chromeless` route handles).
- **Loading:** Four-phase overlay on **cold** load: **restoring** (T-062.1 ✅ v2 chunked / legacy migrate once) → download → apply → local flush. **Warm return** (T-062.2): restoring → local flush only. **v2 @ ~360k:** determinate restoring `done/total` ticks smoothly (no 0→300k jump on 2nd+ load). **Dev alt-tab:** overlay should not reappear after extended background (T-062.2). **Save:** T-060.1.4 FIXED.
- **Error (T-122 C3):** If local restore throws (corrupt/blocked IndexedDB), `docStatus: 'error'` — blocking overlay + toast; user Reloads; server version unaffected.
- **Dirty:** Local autosave to v2 `idb` on `LOCAL_ORIGIN` edits; Save Version posts **editor-only** payload (T-062.1.1 — no duplicate `orbat[]`); server derives ORBAT for events. Export keeps full superset. **T-122 T11:** autosave write failures toast.
- **Blocked:** T-090.1 aligned WebP tiles (no pyramid yet — **active**). **T-091 shipped** @ `dde589e` — DEM + loader + Z UX (hillshade, grid toggles, 3 dp coords). **Basemap today:** procedural grid + optional hillshade over DEM. Ruler/LoS viewshed after terrain program. **Spawn accuracy gate:** **T-092** (ready). **T-068** Phase 2 paused. Hub: [`t068_virtual_arsenal_program.md`](../../../specs/Mission_Creator_Architecture/t068_virtual_arsenal_program.md).

### Keyboard (host — `/missions/:id/edit`)
| Shortcut | Action |
|----------|--------|
| Space | Center camera on selection |
| Delete / Backspace | Remove selected slots (undoable) |
| Cmd/Ctrl+Z | Undo last edit |
| Cmd/Ctrl+Shift+Z or Ctrl+Y | Redo |
| Ctrl/Cmd + click (map icon) | Toggle slot in/out of selection; Ctrl/Cmd+empty map preserves selection (T-053) |
| Cmd/Ctrl+C / Cmd/Ctrl+V | Copy slot selection / paste at cursor (relative layout; off-map +20m nudge); pasted slots become the selection (T-056) |

Skipped when focus is in an input, select, textarea, or contentEditable field (T-052).

Double-click a slot on the **map**, in **ORBAT**, or in **Editor Layers** opens Attributes (T-054). Suppressed when multiple slots are selected.

Undo/redo applies to **session edits only** (drop, drag, delete, title/env changes via `LOCAL_ORIGIN`). Boot restore and server hydrate use `INIT_ORIGIN` (not on the undo stack).

## API Dependencies

| Endpoint | Method | When called | Response shape |
|----------|--------|-------------|----------------|
| `GET /missions/:id` | GET | **Cold** boot only (warm return skips — T-062.2) | `Mission` |
| `GET /missions/:id/versions/current` | GET | Hydrate | `MissionVersion` |
| `POST /missions/:id/versions` | POST | Save Version | `MissionVersion` (409 dup semver; **256 MB** route cap → **413** over it, T-060; other JSON routes stay 1 MB) |
| `GET /missions/:id/export` | GET | Export preview | camelCase envelope |
| `GET /registry` | GET | Factions palette on load | `{ data, etag, modpack_id, modpack_version }` — wired @ **T-068.3**; `mission_maker+` JWT |

## Milestones

### M1 — [x] Route + chromeless shell
### M2 — [x] Y.Doc state, map viewport, Eden docked panels
### M3 — [x] Map drag, multi-select, outliner ops, compiler + Save/Export
### M3.5 — [x] T-049 terrain wired to viewport; row title/terrain/env hydrate; editable Transform X/Y/Z/rotation; selection-aware toolbelt
### M3.6 — [x] T-050 cursor readout shows X/Y/Z
### M3.6a — [x] T-091.1 DEM loader @ `2c56c2e`
### M3.6b — [x] T-091.2 Z UX @ `dde589e` — sampled CUR/SEL Z, hillshade + grid toggles, Mission Settings persistence
### M3.7 — [x] T-052 undo/redo keyboard shortcuts (Cmd/Ctrl+Z, Cmd/Ctrl+Shift+Z, Ctrl+Y)
### M3.8 — [x] T-053 Ctrl/Cmd+LMB additive toggle select on map
### M3.9 — [x] T-054 Attributes entry — map dbl-click + ORBAT tree dbl-click
### M3.10 — [x] T-055 Asset browser search (filters Factions tree; folder-name match shows subtree)
### M3.11 — [x] T-056 Ctrl+C/V copy-paste at cursor (slots; relative layout, off-map +20m nudge; one undo step)
### M3.12 — [x] T-057 Map perf hotfix (≥55 fps pan/zoom @ 200+ slots: cursor→store rAF, no Deck `onHover` pick, pan rAF-coalesce, `React.memo` panels)
### M3.13 — [x] T-058 Toolbelt OBJ/SEL entity counts (total placed slots + selected count; scale telemetry)
### M3.14 — [x] T-059 Bulk paste/delete at scale (batch O(n) append; selection cap 500; outliner virtualization via T-064; validated **360k @ 100+ fps** pan)
### M3.15 — [x] T-060 scale load/save (shipped `b1fd25a` — load partial pass @ ~360k; Save ~142 MB → 201)
### M3.16 — [x] T-061 drag-move @ 360k (good enough — motion ~60 fps; boundaries via `slotIconCache` — spec: [t061_drag_move_hotfix.md](../../../specs/Mission_Creator_Architecture/t061_drag_move_hotfix.md))
### M3.17 — [x] T-062 incremental bindings @ 360k (classifier + bulk delete ≤10k — spec: [t062_incremental_bindings.md](../../../specs/Mission_Creator_Architecture/t062_incremental_bindings.md))
### M3.18 — [x] T-062.2 editor session / alt-tab resilience (Vite reload guard + warm session — spec: [t062_2_editor_session_persistence.md](../../../specs/Mission_Creator_Architecture/t062_2_editor_session_persistence.md))
### M3.19 — [x] T-062.1 chunked IDB slot restore (v2 `tbd-mission-persist`; determinate restoring @ ~360k — spec: [t062_1_idb_streaming_load.md](../../../specs/Mission_Creator_Architecture/t062_1_idb_streaming_load.md))
### M3.20 — [x] T-062.1.1 Save orbat dedup (editor-only POST; Go derives ORBAT — spec: [t062_1_1_batch_save.md](../../../specs/Mission_Creator_Architecture/t062_1_1_batch_save.md))
### M4.21 — [x] T-063 spatial index (rbush pick/marquee @ ~367k — spec: [t063_spatial_index.md](../../../specs/Mission_Creator_Architecture/t063_spatial_index.md))
### M4.22 — [x] T-064 virtualized outliner @ ~367k (incl. T-064.1 scroll-ref hotfix — spec: [t064_virtualized_outliner.md](../../../specs/Mission_Creator_Architecture/t064_virtualized_outliner.md))
### M5.23 — [x] T-066 worker compile offload + `pickMapSnapshot` (Save 201 @ ~367k — spec: [t066_worker_compile.md](../../../specs/Mission_Creator_Architecture/t066_worker_compile.md))
### M5.24 — [x] T-067 spatial chunks — bulk-paste `slot-add-bulk` + chunk scaffolding; CPU viewport cull deferred (T-067.0.1 revert to `getBaseIcons()` @ ~160 fps pan @ 367k) — spec: [t067_spatial_chunks.md](../../../specs/Mission_Creator_Architecture/t067_spatial_chunks.md)
### M5.26 — [ ] T-068 Virtual Arsenal — **Phase 1 shipped** @ 2026-06-27 (T-068.0.1–T-068.6); **active T-068.7+** Phase 2 — hub: [t068_virtual_arsenal_program.md](../../../specs/Mission_Creator_Architecture/t068_virtual_arsenal_program.md)
### M5.26a — [x] T-068.3 Factions palette → live registry (`useRegistry`, `buildCatalogTree`, `resource_name` on drop) @ `da78452`
### M5.26b — [x] T-068.4 Arsenal dumb loadout UI (4 gear dropdowns + `loadout-export.json` download) @ `a85f16b`
### M5.26c — [x] T-068.5 Mod equip loadout JSON (`TBD_LoadoutEquipComponent` @ 6400) @ `21ec91e` — mod; see [`t068_5_mod_equip_loadout.md`](../../../specs/Mission_Creator_Architecture/t068_5_mod_equip_loadout.md)
### M5.26d — [x] T-068.5.1 Visual wear on test NPC (`EquipCloth`/`EquipWeapon` + worn-verify) @ `b233b11` — **not** human player
### M5.26e — [x] T-068.6 Phase 1 E2E gate PASS @ 2026-06-27 — [`t068_6_phase1_e2e_gate.md`](../../../specs/Mission_Creator_Architecture/t068_6_phase1_e2e_gate.md)
### M5.25 — [ ] **T-111** lazy chunk residency @ 1M+ / **T-112** GPU viewport cull (`idea` — [`TICKET_BRAINSTORM.md`](../../../TICKET_BRAINSTORM.md#scale))

## Test Plan

1. Mission maker opens `/missions/:id/edit` → map + docks render full-bleed.
2. Drop asset from palette → slot appears on map and in active layer.
3. Save Version with new semver → 201; duplicate semver → 409 with error surfaced.
4. Export downloads JSON without POST.
5. Drop or drag a unit → Undo enables → Cmd/Ctrl+Z reverts; redo restores.
6. Double-click a **character** slot → Attributes → **Arsenal** → four gear dropdowns populate from `GET /registry`; **Download loadout JSON** saves file passing `loadout-export.schema.json` jq gates (T-068.4 @ `a85f16b`).

## Open Questions / Blockers

- **[PERF-001] ~~Map pan/zoom FPS collapse~~** — **Resolved T-057** (100+ fps @ 10k validated); **T-058** OBJ/SEL entity-count telemetry shipped.
- **[PERF-002] ~~Bulk paste 10k freeze~~** — **Resolved T-059** (validated **360k @ 100+ fps** pan; 6k paste loops smooth).
- **[PERF-003] Initial load** — **Resolved T-062.1** (v2 chunked restore; legacy migrate once). Spec: [t062_1_idb_streaming_load.md](../../../specs/Mission_Creator_Architecture/t062_1_idb_streaming_load.md). Pan **100+ fps** @ 360k when idle.
- **[PERF-004] Save Version** — **Resolved T-060.1.4 / shipped T-060.** Verified: curl 140 MB → 201; browser Save @ ~367k/~142 MB → 201 (2026-06-23).
- **[PERF-005] Drag-move @ 360k** — **Resolved T-061 (good enough).** Motion ~60 fps sustained; pickup/release materially improved via `slotIconCache` + bindings slot fast path. Mega optimizations deferred ([MC ROADMAP §Deferred mega optimizations](../../../specs/Mission_Creator_Architecture/ROADMAP.md)). Spec: [t061_drag_move_hotfix.md](../../../specs/Mission_Creator_Architecture/t061_drag_move_hotfix.md).
- **[PERF-006] Incremental bindings @ 360k** — **Resolved T-062.** Spec: [t062_incremental_bindings.md](../../../specs/Mission_Creator_Architecture/t062_incremental_bindings.md).
- **[PERF-007] Alt-tab / session reload @ 360k** — **Resolved T-062.2.** Extended alt-tab (Firefox dev) no longer re-triggers full load overlay; warm session skips server GET on same-tab return. Spec: [t062_2_editor_session_persistence.md](../../../specs/Mission_Creator_Architecture/t062_2_editor_session_persistence.md).
- **[PERF-008] Outliner @ 360k** — **Resolved T-064.** Virtualized ORBAT + Editor Layers; outliner visible on first paint @ ~367k; scrollable 367k rows; T-064.1 scroll-ref hotfix. Spec: [t064_virtualized_outliner.md](../../../specs/Mission_Creator_Architecture/t064_virtualized_outliner.md).
- **[PERF-009] Spatial chunks / bulk paste @ 367k+** — **Partially resolved T-067.** Bulk paste `slot-add-bulk` shipped; pan ~160 fps @ 367k (CPU cull deferred). Lazy RAM + GPU cull @ 1M+ deferred. Spec: [t067_spatial_chunks.md](../../../specs/Mission_Creator_Architecture/t067_spatial_chunks.md).
- **Active slice:** **T-090.1** — aligned basemap ([`t090_1_aligned_basemap.md`](../../../specs/Mission_Creator_Architecture/t090_1_aligned_basemap.md)). **T-091 shipped** @ `dde589e` (DEM + Z + hillshade complete).
- **Next queued:** **T-092** spawn transform → **T-071** ORBAT → **T-068 Phase 2** → T-069 markers, T-070 vehicles — see [Mission Creator ROADMAP](../../../specs/Mission_Creator_Architecture/ROADMAP.md) and [TICKET_REGISTRY.md](../../../TICKET_REGISTRY.md).
