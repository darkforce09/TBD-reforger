# Mission Creator — ROADMAP

**Start here.** Single planning view for the 2D mission editor — what is **done**, what **must work**, and links to all supporting documentation.

**Route:** `/missions/:id/edit` · **Code:** [`frontend/src/features/mission-creator/`](../../../apps/website/frontend/src/features/mission-creator) + [`tactical-map/`](../../../apps/website/frontend/src/features/tactical-map)

**Open work:** [`docs/TICKET_LEAD.md`](../../TICKET_LEAD.md) (auto-generated queue, dependencies, ready/queued tickets). Eden parity item detail: [`eden/gap_analysis.md`](eden/gap_analysis.md) (`eden_id` columns).

<!-- ticket-sync:next:start -->
### Recommended next work (auto-generated)

- **T-071** — ORBAT Manager modal (queued)
- **T-072** — Ctrl multi-place (queued)
- **T-073** — Shift + map rotation (queued)
- **T-074** — Faction submode / catalog filter (queued)
- **T-075** — Spacebar flyTo vs widget (queued)
- **T-090** — Map visualization program (ready)
- **T-114** — Slot roster enforcement + production slot picker (queued)
- **T-115** — Capture win condition (queued)
- **T-116** — Results POST to backend (queued)
- **T-117** — Mission upload + validation UI (queued)
<!-- ticket-sync:next:end -->

---

## Current strategy (locked — 2026-06-28)

**Map-verify gate:** Ship **T-090 / T-091 / T-092** (aligned tiles, DEM, mod-native compile + spawn Y/yaw) **before** T-071 ORBAT baseline and T-068 Phase 2 loadout. Hub: [`t090_091_map_terrain_program.md`](t090_091_map_terrain_program.md).

| Active now | Blocked until map gate |
|------------|-------------------------|
| **T-090.3.0** Workbench export spike (active) → **T-090.1** basemap | **T-071** ORBAT Manager |
| **T-090.2–.9** typed world objects + **forests (.8)** + **interaction (.9)** → export → Z audit → Deck-zoom render | **T-068.7+** loadout Phase 2 |
| **T-092** mod mission compile (ready) | |
| **Shipped** | **T-090.0** + **T-090.0.1** + **T-090.0.2** (map-object schemas + goldens + verify) · **T-091** @ `dde589e` |
| **Deferred idea** | **T-129** building floor selector (not T-090) |

**T-068 Phase 1 shipped** — registry + dumb loadout + **test NPC** equip only.

### Execution order (recommended)

1. **T-090 → T-091 → T-092** — map + spawn parity ([`t092_spawn_transform_program.md`](t092_spawn_transform_program.md))
2. **T-071.0–.2** — web ORBAT ([`t071_orbat_manager_program.md`](t071_orbat_manager_program.md))
3. **T-068.13** — production mod LOBBY slot picker
4. **T-068.7–.14** — loadout Phase 2 on **human player**
5. **T-069+** — markers, vehicles, … after full **T-068** ship

See [`docs/TICKET_LEAD.md`](../../TICKET_LEAD.md) for registry queue.

### Map performance (contract + scale program)

**Contract (engineering plan §4.4):** 60 fps pan/zoom with **200+** pickable slot icons on the flat grid. **Observed regression (2026-06):** ~100–200 slots + pan → ~9 fps. **T-057** is an **interrupt hotfix** — **shipped** (fps acceptance is a manual in-browser check via `FpsCounter`).

**Root causes → T-057 fix (shipped):**

| Layer | Issue | T-057 fix (done) |
|-------|--------|-----------|
| React shell | `onHover` → `setCursor` re-renders entire `MissionCreatorPage` every pointer move | ✅ Cursor moved to transient `useMapStore.cursor` (rAF-throttled); only `BottomToolbelt` subscribes. `React.memo` on the panels |
| Deck picking | `IconLayer` `pickable: true` + `onHover` runs a pick pass over all icons for cursor coords | ✅ Removed `onHover`; cursor unprojected from the mouse on `onPointerMove`. Picking only on click/dbl-click/marquee/drag-start |
| Pan | `useOrthographicView` `setViewState` every pan frame re-renders `TacticalMap` + children | ✅ `useSelectTool` rAF-coalesces pan to one `setViewState`/frame (layers already memoized) |
| Gestures | `pickObject` on pointerdown + hover during pan | ✅ Hover picking removed (T-057); **T-063:** rbush `pickNearest`/`pickRect` replaces Deck GPU pick; `slot-icons` not pickable |

**1M–10M editable entities** is the **north star** (Arma 3 reference ~8M map objects); reach it **step-by-step** (not one commit). **Validated (2026-06):** pan/zoom **100+ fps @ 360k** (T-057 + T-059); repeat **6k paste** loops smooth. **Bulk paste — fixed (T-059).** **T-060 shipped** (`b1fd25a`): load partial pass @ ~360k; Save @ ~367k/~142 MB → **201**. **T-061 shipped (good enough):** drag motion ~60 fps @ 360k. **T-062 shipped:** incremental bindings — asset drop, delete (≤10k/batch), meta, editor-layers @ 360k. **T-062.2 shipped:** editor session / alt-tab — no automatic reload overlay after extended background (dev Vite guard + warm session fast path). **T-062.1 shipped:** chunked IDB slot restore — v2 `tbd-mission-persist`; determinate restoring @ ~360k (no 0→300k jump on 2nd+ load). **T-062.1.1 shipped:** Save orbat dedup — editor-only POST; Go derives ORBAT for events. **T-063 shipped:** rbush spatial index — click/marquee pick @ ~367k significantly faster vs Deck GPU pick. **T-064 shipped:** virtualized outliner — scrollable @ ~367k, no DOM explosion; T-064.1 scroll-ref hotfix. **T-065 shipped** (`845bfb2`) — cluster/LOD. **T-066 shipped** (`53bc2a8`) — worker compile + `pickMapSnapshot`; Save 201 @ ~367k. **T-067 shipped** — bulk-paste `slot-add-bulk` + chunk scaffolding; CPU viewport cull deferred (T-067.0.1). Remaining @ 1M+: lazy RAM (T-067.1) + GPU cull. Phased track:

| Tag | Focus | Entity target | FPS / UX target |
|-----|-------|---------------|-----------------|
| **T-057** ✅ | Hotfix | 200+ | ≥55 fps pan/zoom — **shipped**. Spec: [`t057_map_performance_hotfix.md`](t057_map_performance_hotfix.md) |
| **T-058** ✅ | Scale prep | — | Toolbelt **OBJ** + **SEL** — **shipped**. Spec: [`t058_entity_count_readout.md`](t058_entity_count_readout.md) |
| **T-059** ✅ | Bulk ops | 360k+ paste/pan | Batch O(n) paste; selection/outliner caps — **shipped** (validated **360k @ 100+ fps** pan). Spec: [`t059_bulk_paste_operations.md`](t059_bulk_paste_operations.md) |
| **T-060** ✅ | Fast load + save | 10k–1M | Load gate + bulk sync + overlay; chunked compile + Save progress; **256 MB** version POST + **413** — **shipped** `b1fd25a`. Spec: [`t060_fast_initial_load.md`](t060_fast_initial_load.md) |
| **T-060.1.1** ✅ | IDB progress | 300k+ | `restoring` phase + `yieldToUi` — **shipped**; legacy v1 only (0→300k jump) — **superseded by T-062.1 v2** |
| **T-060.1.2** ✅ | Save upload fixes | 300k+ | E1/E2/E3b — **shipped**. Spec: [`t060_1`](t060_1_scale_load_save_completion.md) §T-060.1.2 |
| **T-060.1.3** ✅ | Save observability | 300k+ | **Shipped** — measured size, debug panel, failure diagnosed @ 367k. Spec: [`t060_1`](t060_1_scale_load_save_completion.md) §T-060.1.3 |
| **T-060.1.4** ✅ | Fix mid-upload | 300k+ | **Shipped** — hardened skip + production-like IT; browser ~142 MB + curl 140 MB → 201. Spec: [`t060_1`](t060_1_scale_load_save_completion.md) §T-060.1.4 |
| **T-061** | Drag-move | 360k drag-move | **Shipped (good enough)** — dual IconLayer + `slotIconCache` + slot fast path. Spec: [`t061_drag_move_hotfix.md`](t061_drag_move_hotfix.md) |
| **T-061.0** | (sub) Motion | 360k drag sustained | **Shipped** — ~60 fps sustained @ 360k |
| **T-061.0.1** | (sub) Boundaries | 360k pickup/release | **Shipped** — O(k) cache + incremental slot observer |
| **T-061.1** | Optional | 50k–500k+ | **Deferred** — typed-array IconLayer; see §Deferred mega optimizations |
| **T-062** | Bindings | 50k+ | **Shipped** — interactive incremental `bindings.ts` + bulk delete @ 360k. Spec: [`t062_incremental_bindings.md`](t062_incremental_bindings.md) |
| **T-062.0** | (sub) Classifier | 360k edits | **Shipped** — `incPatchPlan` + O(k) store/icon-cache patches |
| **T-062.0.1** | (sub) Bulk delete | ≤10k/batch | **Shipped** — batched `removeEntities`, `slotCount`/`slotsRevision`, `REMOVE_PATCH_CAP` 10k |
| **T-062.2** | (sub) Session | Alt-tab / reload | **Shipped** — Vite reload guard + warm session + background yields. Spec: [`t062_2_editor_session_persistence.md`](t062_2_editor_session_persistence.md) |
| **T-062.1** ✅ | Load | 360k+ | Chunked IDB slot restore (v2 `tbd-mission-persist`) — **shipped**; spec: [`t062_1`](t062_1_idb_streaming_load.md) |
| **T-062.1.1** ✅ | Save | 360k+ | Save orbat dedup (editor-only POST; Go derives ORBAT) — **shipped**; spec: [`t062_1_1`](t062_1_1_batch_save.md) |
| **T-063** ✅ | Pick | 50k+ pick | Spatial index (rbush) for pick/marquee — **shipped**; spec: [`t063_spatial_index.md`](t063_spatial_index.md) |
| **T-064** ✅ | Outliner | 50k+ UI | Virtualized outliner — **shipped**; spec: [`t064_virtualized_outliner.md`](t064_virtualized_outliner.md) |
| **T-065** ✅ | Cluster/LOD | 100k–1M | Cluster/LOD extreme zoom — **shipped**; spec: [`t065_cluster_lod.md`](t065_cluster_lod.md) |
| **T-066** ✅ | Worker | 1M+ export | Worker offload — **shipped** (T-066.1 `pickMapSnapshot`); spec: [`t066_worker_compile.md`](t066_worker_compile.md) |
| **T-067** | Chunks | 1M–10M | **Shipped** — bulk paste + scaffolding; CPU cull deferred ([`t067_spatial_chunks.md`](t067_spatial_chunks.md)) |
| **T-110** | Terrain base | 1M–10M props | Binary world base + sparse terrain deltas — **future**; see [`t110_terrain_base_mission_layers.md`](t110_terrain_base_mission_layers.md) |

**Dual-layer north star (T-110, not current work):** **Terrain base** (millions of read-mostly map objects → binary + sparse deltas) is separate from **authored mission entities** (ORBAT slots, markers → Y.Doc + T-061..T-062). Do **not** replace the mission layer with terrain deltas. External “Base + Delta” proposal adopted **only** for the terrain track after T-067 + **T-068+**.

**Milestone ladder:**

| Objects | Pan/zoom | Bulk paste | Load / Save |
|---------|----------|------------|-------------|
| 10k–360k | ✅ 100+ fps | ✅ T-059 | ✅ T-060 (load partial pass; Save ~142 MB → 201) |
| 1M ideal | T-061–T-065 | ✅ T-059 | T-060 + T-062.1 + **≤10 s** stretch (**T-066** worker) |
| 1M–10M props | T-061–T-067 + **T-110** | ✅ T-059 | Terrain base + deltas; mission patch save |

**T-057–T-067 shipped.** **T-068+** Eden backlog → **T-110** terrain base (optional).

Spec: [`t057_map_performance_hotfix.md`](t057_map_performance_hotfix.md) (shipped T-057).

---

## Documentation (read from here)

| Doc | When to open it |
|-----|-----------------|
| **[`docs/TICKET_LEAD.md`](../../TICKET_LEAD.md)** | **Open work** — ready/queued tickets, dependency graph |
| **[`agent_execution.md`](agent_execution.md)** | Locked UX decisions, agent phase history, copy-paste agent prompt |
| **[`feature_inventory.md`](feature_inventory.md)** | Per-feature code-evidenced status (FEDS) |
| **[`engineering_plan.md`](engineering_plan.md)** | Y.Doc schema, compiler, workers, engineering phases 0–9 |
| **[`ux_spec.md`](ux_spec.md)** | Eden docked-shell UX contract |
| **[`problem_statement.md`](problem_statement.md)** | Why 200-slot GPU, DEM, nesting, registry matter |
| **[`reference/feds_schema.md`](reference/feds_schema.md)** | FEDS v2 feature-entry schema |
| **[`eden/interactions.md`](eden/interactions.md)** | Eden interaction reference |
| **[`eden/ui_anatomy.md`](eden/ui_anatomy.md)** | Panel-by-panel Eden UI |
| **[`eden/attributes.md`](eden/attributes.md)** | Attribute catalog |
| **[`eden/gap_analysis.md`](eden/gap_analysis.md)** | Eden parity backlog (`eden_id` ↔ ticket mapping) |
| **[`eden/wiki_manifest.yaml`](eden/wiki_manifest.yaml)** | Scrape manifest — 28 Bohemia Eden Editor wiki pages |
| **[`artifacts/eden-wiki/`](../../../.ai/artifacts/eden-wiki)** | **Cached wiki markdown** (generated; do not hand-edit) |
| **[`scripts/tools/scrape-eden-wiki.mjs`](../../../scripts/website/tools/scrape-eden-wiki.mjs)** | Regenerate wiki cache from manifest |
| **[`artifacts/eden-feds-draft.jsonl`](../../../.ai/artifacts/eden-feds-draft.jsonl)** | Draft FEDS entries derived from wiki research |
| **[`artifacts/README.md`](../../../.ai/artifacts/README.md)** | Generated artifacts policy |
| **[`t058_entity_count_readout.md`](t058_entity_count_readout.md)** | **T-058** — Toolbelt OBJ/SEL entity counts (shipped) |
| **[`t059_bulk_paste_operations.md`](t059_bulk_paste_operations.md)** | **T-059** — Bulk paste/delete at scale (shipped) |
| **[`t060_fast_initial_load.md`](t060_fast_initial_load.md)** | **T-060** — Fast load + save (**shipped** `b1fd25a`) |
| **[`t060_1_scale_load_save_completion.md`](t060_1_scale_load_save_completion.md)** | **T-060.1 + T-060.1.1 + T-060.1.2 + T-060.1.3 + T-060.1.4** — Load/save @ 360k (**shipped**) |
| **[`t061_drag_move_hotfix.md`](t061_drag_move_hotfix.md)** | **T-061** — Drag-move @ 360k (**shipped — good enough**) |
| **[`t062_incremental_bindings.md`](t062_incremental_bindings.md)** | **T-062** — Incremental bindings @ 360k (**shipped**) |
| **[`t062_2_editor_session_persistence.md`](t062_2_editor_session_persistence.md)** | **T-062.2** — Editor session / alt-tab resilience (**shipped**) |
| **[`t062_1_idb_streaming_load.md`](t062_1_idb_streaming_load.md)** | **T-062.1** — Chunked IDB slot restore @ 360k (**shipped**) |
| **[`t062_1_1_batch_save.md`](t062_1_1_batch_save.md)** | **T-062.1.1** — Save orbat dedup (**shipped**) |
| **[`t063_spatial_index.md`](t063_spatial_index.md)** | **T-063** — rbush spatial index for pick/marquee (**shipped**) |
| **[`t064_virtualized_outliner.md`](t064_virtualized_outliner.md)** | **T-064** — Virtualized outliner @ 100k–360k+ leaves (**shipped**) |
| **[`t065_cluster_lod.md`](t065_cluster_lod.md)** | **T-065** — Cluster / LOD @ extreme zoom (**shipped** `845bfb2`) |
| **[`t066_worker_compile.md`](t066_worker_compile.md)** | **T-066** — Worker compile + version blob (**shipped** — T-066.1 `pickMapSnapshot`) |
| **[`t067_spatial_chunks.md`](t067_spatial_chunks.md)** | **T-067** — Spatial chunks / bulk-paste scale (**shipped**) |
| **[`t057_map_performance_hotfix.md`](t057_map_performance_hotfix.md)** | **T-057** — Map perf hotfix: ≥55 fps pan/zoom @ 200+ slots (shipped) |
| **[`t056_copy_paste.md`](t056_copy_paste.md)** | **T-056** — Ctrl+C/V copy-paste at cursor (slots) (shipped) |
| **[`t055_asset_browser_search.md`](t055_asset_browser_search.md)** | **T-055** — Asset browser search (filters Factions tree) (shipped) |
| **[`t054_attributes_entry_points.md`](t054_attributes_entry_points.md)** | **T-054** — Attributes entry points (map native dblclick + ORBAT dbl-click) (shipped) |
| **[`t053_additive_select.md`](t053_additive_select.md)** | **T-053** — Ctrl/Cmd+LMB additive (toggle) select (shipped) |
| **[`t052_undo_shortcuts.md`](t052_undo_shortcuts.md)** | **T-052** — Ctrl/Cmd+Z/Y undo-redo keyboard (shipped) |
| **[`t050_cursor_z_readout.md`](t050_cursor_z_readout.md)** | **T-050** — Cursor Z readout (shipped) |
| **[`t049_terrain_title_position.md`](t049_terrain_title_position.md)** | **T-049** — Terrain + title hydrate + numeric position (shipped) |
| **[`t048_library_create_dialog.md`](t048_library_create_dialog.md)** | T-048 — Library create dialog (shipped) |
| [`docs/website/frontend/pages/mission-library.md`](../../website/frontend/pages/mission-library.md) | Surface spec for `/missions` (+ create dialog T-048) |
| [`docs/website/frontend/pages/mission-editor.md`](../../website/frontend/pages/mission-editor.md) | Surface spec for `/missions/:id/edit` |
| [`docs/website/frontend/pages/mission-creator.md`](../../website/frontend/pages/mission-creator.md) | Archived — wizard moved into library (T-048) |
| **[`t068_virtual_arsenal_program.md`](t068_virtual_arsenal_program.md)** | **T-068** — Virtual Arsenal program hub (**Phase 1 shipped**; active **T-068.7**) |
| [`t068_5_1_visual_equip_fix.md`](t068_5_1_visual_equip_fix.md) | **T-068.5.1 shipped** — visual wear on test NPC @ `b233b11` |
| [`t068_6_phase1_e2e_gate.md`](t068_6_phase1_e2e_gate.md) | **T-068.6 shipped** — Phase 1 E2E sign-off PASS @ 2026-06-27 |
| [`t068_2_registry_api.md`](t068_2_registry_api.md) | **T-068.2 shipped** — `GET /api/v1/registry`, seed, import CLI |
| [`t068_3_palette_wire.md`](t068_3_palette_wire.md) | **T-068.3 shipped** — Factions palette → live registry |
| [`t068_4_dumb_loadout_ui.md`](t068_4_dumb_loadout_ui.md) | **T-068.4 shipped** — Arsenal dumb loadout download @ `a85f16b` |
| [`t068_5_mod_equip_loadout.md`](t068_5_mod_equip_loadout.md) | **T-068.5 shipped** — mod equip scaffold @ `21ec91e` |
| [`t068_asset_registry.md`](t068_asset_registry.md) | Legacy stub → redirects to program hub |
| **[`t110_terrain_base_mission_layers.md`](t110_terrain_base_mission_layers.md)** | **T-110** — Terrain base + mission layers (future; Base + Delta for props only) |
| [`CLAUDE.md`](../../../CLAUDE.md) §Status | **ACTIVE: T-068.7**; Phase 1 T-068.0.1–T-068.6 shipped |

---

## Terminology (fixes the “kits vs armory” confusion)

| Term | What it is today | Where |
|------|------------------|-------|
| **Mission Armory** | Aggregate briefing list (“M16A2 Rifle ×45”) per faction | Backend `MissionArmory` + `GET/PUT /missions/:id/armory` — **already exists**, separate from the editor |
| **ORBAT slot `loadout`** | Short string on each slot in export (`"L85A3 + GL"`) | `json_payload.orbat[].slots[].loadout` — compiler writes `''` today |
| **Loadout (editor model)** | Full per-slot gear graph: uniform, vest, weapons, mags, attachments | Y.Doc `loadouts` + `items` maps in schema — **UI not built** |
| **Master Item Registry** | Every valid `resource_name` + slot rules + icons | **Phase 1 shipped** — API + palette + dumb export + **test NPC mod equip**; **human player loadout** = T-068.11–T-068.12; **mod slot picker** = T-068.13; ship gate **T-068.14** |
| **Loadout Forge** | Web UI to edit a slot’s loadout | **Dumb export shipped @ T-068.4** (`AttributesModal` Arsenal tab); smart Forge **T-068.10** |

**Arma Reforger** (game + modpack entity/workshop data) = **data source** for the loadout program, not something the map editor implements. The website needs an **ingest pipeline + Postgres registry**, then the editor **reads** it.

---

## DONE (code-evidenced, 2026-06-20)

### Editor shell & routing
- Lazy route `/missions/:id/edit` (`mission_maker+`, full-bleed)
- Aegis glass UI: top strip, left sidebar (ORBAT + Editor Layers), right asset palette, bottom toolbelt
- Mission Settings dialog (time, weather, view distance, thermals)
- Attributes modal (Identity / Transform editable / States stub / **Arsenal dumb loadout @ T-068.4**)

### Map engine (partial — T-049, T-050, T-057)
- Deck.gl orthographic viewport, Arma meter coords (`flipY: false`, identity projection)
- Terrain **definitions** (Everon 12800×12800 m, Arland, custom bounds)
- Vector grid base map (no satellite/topo imagery yet)
- Pan/zoom with bounds clamp; cursor X/Y/Z in toolbelt (Z=0 flat until DEM, T-050)
- Icon layer for placed **slots**; selection highlight; marquee select + live overlay
- Drag-move slots with live preview + Y.Doc commit; undo/redo (buttons + keyboard Cmd/Ctrl+Z/Y, T-052)

### Placement (partial — slots only)
- Mock asset palette (Factions tab); HTML5 drag-drop → `addSlot`
- Auto squad/faction on first drop; active Editor Layer targets drops
- Double-click slot in **Editor Layers** tree → Attributes

### State & persistence (partial)
- Y.Doc normalized store + Zustand mirror + **v2 chunked IDB** (`tbd-mission-persist`; legacy y-indexeddb migrate-once — T-062.1)
- `compileMission` → `json_payload` superset (`orbat[]` + `editor` block with positions)
- Semver Save Version to API; IndexedDB vs server conflict dialog
- Hydrate from server `json_payload` (or lossy ORBAT-only fallback)

### Documentation & Eden wiki research (T-042)
- FEDS inventory ([`feature_inventory.md`](feature_inventory.md)), Eden reference ([`eden/`](eden/))
- **Arma 3 Eden Editor wiki scrape:** 28 pages in [`artifacts/eden-wiki/`](../../../.ai/artifacts/eden-wiki) via [`eden/wiki_manifest.yaml`](eden/wiki_manifest.yaml) + [`scrape-eden-wiki.mjs`](../../../scripts/website/tools/scrape-eden-wiki.mjs); feeds [`eden/interactions.md`](eden/interactions.md), [`eden/ui_anatomy.md`](eden/ui_anatomy.md), [`eden/attributes.md`](eden/attributes.md), [`eden/gap_analysis.md`](eden/gap_analysis.md)

---

## DONE — T-060 (Fast load + save — code landed; acceptance → T-060.1)

| Item | Spec | Deliverable |
|------|------|-------------|
| **Load/save foundation** | [`t060_fast_initial_load.md`](t060_fast_initial_load.md) | ✅ **256 MB** version POST (`bodylimit.go`); bulk-sync coalesce; `docStatus` + overlay; deferred sidebar; `compileMissionWithProgress` + Save phases + 413/409 surfacing. Completion: [`t060_1_scale_load_save_completion.md`](t060_1_scale_load_save_completion.md) (**shipped**). |

## DONE — T-059 (Bulk paste/delete at scale)

| Item | Spec | Deliverable |
|------|------|-------------|
| **10k paste without freeze** | [`t059_bulk_paste_operations.md`](t059_bulk_paste_operations.md) | ✅ Batch O(n) `pasteSlots`; selection cap 500; outliner virtualization (T-064 supersedes T-059 leaf cap). **Live validated:** repeat **6k paste** smooth; **360k objects @ 100+ fps** pan/zoom. Chunked paste not needed. |

## DONE — T-058 (Toolbelt entity count readout)

| Item | Spec | Deliverable |
|------|------|-------------|
| **OBJ/SEL counts** | [`t058_entity_count_readout.md`](t058_entity_count_readout.md) | ✅ Bottom toolbelt shows **OBJ** = total placed slots (memoized `selectSlotCount(slotsById)` in `selectors.ts`, re-exported from `index.ts`) + **SEL** = `selection.ids.length` when `kind==='slot'` else 0, right of the X/Y/Z block (mono `tabular-nums`, plain integers). Both subscribe inside the already-memoized `BottomToolbelt`, so they update on add/remove/paste/delete/selection but **not** on cursor move (T-057 channel untouched). Slots only; vehicles/markers join in **T-068+**. No Deck/schema/backend change. |

## DONE — T-057 (Map performance hotfix)

| Item | Spec | Deliverable |
|------|------|-------------|
| **Map perf hotfix** | [`t057_map_performance_hotfix.md`](t057_map_performance_hotfix.md) | ✅ Restores ≥55 fps pan/zoom @ 200+ slots (manual `FpsCounter` check): cursor → transient `useMapStore.cursor` (rAF-throttled, only `BottomToolbelt` re-renders on move); drop Deck `onHover` (self-unproject for toolbelt coords); pan rAF-coalesce in `useSelectTool`; `React.memo` on `TacticalMap`, sidebars, toolbelt, modal. **UX trade:** constant `crosshair` cursor (no pointer glyph over icons). All interactions unchanged (T-053–T-056). |

## DONE — T-056 (copy-paste)

| Item | Spec | Deliverable |
|------|------|-------------|
| **Ctrl+C/V copy-paste** | [`t056_copy_paste.md`](t056_copy_paste.md) | ✅ Ctrl/Cmd+C snapshots the slot selection to an in-editor clipboard (`ClipboardSlot[]` ref); Ctrl/Cmd+V pastes at the map cursor preserving relative layout (centroid → cursor; off-map → +20m/+20m nudge). New batched `pasteSlots(md, clip, { anchorAt, layerId })` in `state/ydoc.ts` (one transact; re-attaches to source squad or default, files into active layer, clamps to terrain bounds, returns new ids → selection). Two keydown branches in `MissionCreatorPage` behind the form-field guard (native text copy/paste preserved); cursor read via ref. Scope: copy+paste, slots only (Cut / paste-orig out). Closes gap_analysis **ACTION-COPY-001** / **ACTION-PASTE-001**. |

## DONE — T-055 (asset browser search)

| Item | Spec | Deliverable |
|------|------|-------------|
| **Asset browser search** | [`t055_asset_browser_search.md`](t055_asset_browser_search.md) | ✅ `AssetBrowser` (Factions tab) gains a search field over a recursive `filterCatalog(ASSET_CATALOG, q)` (case-insensitive label substring; folder kept on self-match → full subtree, else on descendant match → filtered children; retained folders force-expanded). `TreeView` keyed on the query so its mount-time expand pass re-runs and reveals matches; empty result → "No assets match"; X/Esc clears. Filtered leaves still drag-to-place. One real file — no `TreeView`/`ASSET_CATALOG`/store change. Closes gap_analysis **RIGHT-SEARCH-001**. |

## DONE — T-054 (Attributes entry points)

| Item | Spec | Deliverable |
|------|------|-------------|
| **Attributes entry points** | [`t054_attributes_entry_points.md`](t054_attributes_entry_points.md) | ✅ Map double-click moved off the hand-rolled 350ms `lastClick` timer to a native `onDoubleClick` on the container + `deckRef.pickObject('slot-icons')` → `onEntityActivate`; `OrbatSection` gains `onActivateSlot` (threaded via `LeftSidebar`) and passes `onActivate` to its `TreeView` so an ORBAT slot row's dbl-click opens Attributes — mirrors `EditorLayersSection`. Multi-select suppression (`ids.length <= 1`) and T-053 Ctrl/Cmd toggle unchanged. Closes gap_analysis **SEL-ORBAT-DBL-001** (and hardens **SEL-MAP-004**). |

## DONE — T-053 (additive select)

| Item | Spec | Deliverable |
|------|------|-------------|
| **Ctrl/Cmd+LMB additive select** | [`t053_additive_select.md`](t053_additive_select.md) | ✅ `TacticalMap onClick` reads `event.srcEvent.ctrlKey/metaKey`; Ctrl/Cmd-click toggles a slot in/out of `selection.ids` (empties → `none`); Ctrl/Cmd + empty-click preserves selection. **Shift unbound** (reserved for range-select); marquee still replaces. One file, no store/`useSelectTool` change. Closes gap_analysis **SEL-MOD-001**. |

## DONE — T-052 (undo keyboard)

| Item | Spec | Deliverable |
|------|------|-------------|
| **Ctrl/Cmd+Z/Y undo-redo** | [`t052_undo_shortcuts.md`](t052_undo_shortcuts.md) | ✅ Host keydown in `MissionCreatorPage` + **`useMissionDoc` StrictMode `instanceKey` lifecycle** (dev undo was dead without it). Cmd/Ctrl+Z undo; Cmd/Ctrl+Shift+Z or Ctrl+Y redo; focus guard (INPUT/SELECT/TEXTAREA/contentEditable). Closes gap_analysis **TOOLBAR-UNDO-001** / **KEY-UNDO-001**. |

**Next:** see [`docs/TICKET_LEAD.md`](../../TICKET_LEAD.md). **T-068 Phase 1 shipped** @ 2026-06-27; **active T-068.7**. **Human player loadout:** T-068.11 (compiler) → T-068.12 (mod equip). **Mod slot picker POC:** T-068.13. **`ticket done T-068` @ T-068.14.** Production roster picker: **T-114**.

---

## DONE — T-061 (drag-move @ 360k)

| Item | Spec | Deliverable |
|------|------|-------------|
| **Drag-move perf** | [`t061_drag_move_hotfix.md`](t061_drag_move_hotfix.md) | ✅ T-061.0: dual IconLayer + split drag state + rAF delta (~60 fps sustained @ 360k). ✅ T-061.0.1: `slotIconCache` O(k) boundaries + bindings slot fast path. **Good enough** for Eden-blocking work; mega optimizations deferred (§Deferred mega optimizations). |

---

## DONE — T-050 (cursor Z readout)

| Item | Spec | Deliverable |
|------|------|-------------|
| **Cursor X/Y/Z** | [`t050_cursor_z_readout.md`](t050_cursor_z_readout.md) | ✅ Toolbelt **CUR** mode shows cursor **X/Y/Z** (was X/Y + dimmed `—`). `onCursorMove` payload + `TacticalMap` `onHover` carry `z: info.coordinate[2] ?? 0`; **Z = 0** on the flat map (real value, not placeholder), off-map → `—`. SEL mode unchanged. |

---

## DONE — T-049 (terrain, title, numeric position)

| Item | Spec | Deliverable |
|------|------|-------------|
| **Terrain + title + numeric position** | [`t049_terrain_title_position.md`](t049_terrain_title_position.md) | ✅ `meta.terrain` → `<TacticalMap>` viewport (key-remount on change; **MAP-TERRAIN-001**); `applyMissionRowMeta` hydrates row title/terrain/env on load (**DATA-HYD-TITLE-001**); `updateSlotPosition` → editable X/Y/Z/rotation in Attributes Transform (**ATTR-FIELD-OBJ-POSITION**), selection-aware toolbelt readout |

**T-091 shipped** @ `dde589e` (DEM + Z). **T-090.1** aligned tiles still pending. Does not include registry/markers/vehicles (**T-068+**).

---

## DONE — T-048 (platform UX)

| Item | Spec | Deliverable |
|------|------|-------------|
| **Create from Library** | [`t048_library_create_dialog.md`](t048_library_create_dialog.md) | ✅ `CreateMissionDialog` on `/missions` (header button + My-Missions empty-state CTA + Cmd/Ctrl+N, `mission_maker+`); `/missions/create` route + sidebar nav removed |

"Mission Creator" labels remain on the dossier CTA + `/missions/:id/edit` breadcrumb (only the standalone wizard tab was removed).

---

## NOT DONE — Map & positioning (**T-090 / T-091 / T-092** active program)

Required for positioning you can trust in-game. Hub: [`t090_091_map_terrain_program.md`](t090_091_map_terrain_program.md).

| Requirement | Status | Notes |
|-------------|--------|-------|
| **Aligned map imagery** (Satellite + Map basemap views, same origin as Reforger) | **Done (T-090.1 + T-090.1.1)** @ `564419e` / `6e06e679` | Satellite unified @ `.2.8`; Map cartographic pyramid + radio @ **T-090.1.1**. [`t090_basemap_dual_view.md`](t090_basemap_dual_view.md). |
| **Terrain wired to mission** (`meta.terrain` → viewport) | **Done (T-049)** | `terrainId` from `meta.terrain`, `key`-remounts `<TacticalMap>` on change. Bounds from Biki via `coords/terrains.ts`. |
| **DEM loader / `sampleElevation`** | **Done (T-091.1)** @ `2c56c2e` | `tactical-map/dem/*`; consumed by T-091.2. |
| **DEM assets (export)** | **Done (T-091.0)** @ `6d96339` | 16-bit PNG + 11 anchors; `make verify-terrain-strict` PASS. |
| **Z on place & move** (sample DEM at x,y) | **Done (T-091.2)** @ `dde589e` | `terrainZ` in `addSlot` / `pasteSlots` / `moveEntities`; Attributes X/Y re-sample. |
| **Z in UI** (toolbelt + Attributes, editable) | **Done (T-091.2)** @ `dde589e` | CUR/SEL + X/Y/Z @ 3 dp; hillshade + grid toggles in Mission Settings. |
| **Satellite / Map basemap views** | **Done (T-090.1 + T-090.1.1)** @ `564419e` / `6e06e679` | Satellite unified bundle + Map cartographic pyramid; Mission Settings **Satellite | Map** switch. Rebuild: `make map-water-everon` / `make map-cartographic-everon`. [`t090_basemap_dual_view.md`](t090_basemap_dual_view.md). |
| **Typed world objects** (roads, trees, buildings on map) | **T-090.2–.5** — spec ready | Taxonomy → Workbench export → Z audit → Deck layers. Eden UX ref: [`t090_eden_map_reference.md`](t090_eden_map_reference.md). |
| **Z burial / floating props** | **T-090.4** + **T-090.6** — automated @ 1M | Phase A: pivot vs DEM. Phase B: OBB samples + `visibleAboveGroundPct` (no manual verify). |
| **Building floor selector** | **T-129** — idea | Explicit out of T-090 scope. |
| **Numeric X/Y/Z edit** (no “eyeball only”) | **Done (T-049)** | `updateSlotPosition` + Attributes `NumberField`s (blur/Enter commit; x/y clamped to terrain). |
| **Rotation** (numeric + map) | **Partial (T-049/T-073)** | Numeric rotation editable in Transform (normalized 0–360); on-map rotate handle → **T-073**. |
| **Mod spawn parity** (x/z/y/headingDeg) | **Done (T-092)** @ `a73224f2` | Schema 1.2 optional `y`; `GET /api/v1/missions/:id/compiled`; mod loader v1 + `X-Service-Token`; wb_play E2E PASS. Hub: [`t092_spawn_transform_program.md`](t092_spawn_transform_program.md). |
| **Export contract verified** | **Done (T-092.2)** @ `a73224f2` | `/compiled` validates against `mission.schema.json`; mod round-trip @ 4 slots. |
| **Title hydrate from API** | **Done (T-049)** | `applyMissionRowMeta` applies the mission row `title` (+ terrain/env) to `meta` on load, including empty-`json_payload` missions. No PATCH-back (deferred **T-051**). |
| **Autosave to mission version** | **Partial** | Save Version works; continuous autosave debounce not fully wired per [`engineering_plan.md`](engineering_plan.md). |

**Accuracy note:** Deck.gl `unproject` is exact in **world meters** for the defined terrain bounds. “Off by 10%” failures usually mean **(1)** map tiles not aligned to world origin, **(2)** wrong terrain bounds vs game, or **(3)** Z always zero. Fix aligned tiles + DEM + numeric edit before tuning icons.

---

## DONE — T-068.4 (Arsenal dumb loadout UI)

| Item | Spec | Deliverable |
|------|------|-------------|
| **Arsenal tab dumb export** | [`t068_4_dumb_loadout_ui.md`](t068_4_dumb_loadout_ui.md) | ✅ `loadoutExport.ts` + `AttributesModal` `ArsenalTab` — 4 registry gear dropdowns + **Download loadout JSON**; character-only guard; stub removed. Tag **T-068.4** @ `a85f16b`. Closes **ATTR-TAB-004** (dumb export). |

---

## DONE — T-068.5 (mod equip scaffold)

| Item | Spec | Deliverable |
|------|------|-------------|
| **Mod loadout equip scaffold** | [`t068_5_mod_equip_loadout.md`](t068_5_mod_equip_loadout.md) | ✅ `TBD_LoadoutEquipComponent.c` — profile JSON → **test NPC** @ 6400. Tag **T-068.5** @ `21ec91e`. Visual wear fixed in **T-068.5.1**. |

---

## DONE — T-068.5.1 (visual wear on test NPC)

| Item | Spec | Deliverable |
|------|------|-------------|
| **Visual wear fix** | [`t068_5_1_visual_equip_fix.md`](t068_5_1_visual_equip_fix.md) | ✅ `EquipCloth`/`EquipWeapon` + worn-verify. Test **NPC** dressed @ spawn. Tag **T-068.5.1** @ `b233b11`. **Not** human player. |

---

## DONE — T-068.6 (Phase 1 E2E gate)

| Item | Spec | Deliverable |
|------|------|-------------|
| **Phase 1 sign-off** | [`t068_6_phase1_e2e_gate.md`](t068_6_phase1_e2e_gate.md) | ✅ E1–E12 PASS @ 2026-06-27. Phase 2 approved. |

---

## IN PROGRESS — T-068 Virtual Arsenal (Phase 2)

| Slice | Spec | Status |
|-------|------|--------|
| **T-068.7** | [`t068_7_compat_matrix_spec.md`](t068_7_compat_matrix_spec.md) | **active** — compat matrix spec (cursor-docs) |
| **T-068.8** | [`t068_8_workbench_compat_export.md`](t068_8_workbench_compat_export.md) | queued |
| **T-068.9** | [`t068_9_registry_worker_ingest.md`](t068_9_registry_worker_ingest.md) | queued |
| **T-068.10** | [`t068_10_smart_forge_ui.md`](t068_10_smart_forge_ui.md) | queued |
| **T-068.11** | [`t068_11_compiler_loadout_export.md`](t068_11_compiler_loadout_export.md) | queued — per-slot loadout in compiled JSON |
| **T-068.12** | [`t068_12_mod_player_loadout_equip.md`](t068_12_mod_player_loadout_equip.md) | queued — **mod:** dress **human player** on deploy |
| **T-068.13** | [`t068_13_mod_slotting_screen_poc.md`](t068_13_mod_slotting_screen_poc.md) | queued — **mod:** LOBBY slot picker (production UI) |
| **T-068.14** | [`t068_14_phase2_e2e_gate.md`](t068_14_phase2_e2e_gate.md) | queued — human E2E → `ticket done T-068` |
| **T-114** | platform mod queue | queued after **T-068.13** + **T-118** — roster-synced picker (**not** full web ORBAT) |

### Phase 1 (shipped @ 2026-06-27)

| Slice | Spec | Status |
|-------|------|--------|
| **T-068.0.1** | [`t068_0_1_registry_schemas.md`](t068_0_1_registry_schemas.md) | ✅ shipped `2487d59` |
| **T-068.1** | [`t068_1_workbench_flat_export.md`](t068_1_workbench_flat_export.md) | ✅ shipped `ca4f2cd` |
| **T-068.2** | [`t068_2_registry_api.md`](t068_2_registry_api.md) | ✅ shipped `4c609fe` |
| **T-068.3** | [`t068_3_palette_wire.md`](t068_3_palette_wire.md) | ✅ shipped `da78452` |
| **T-068.4** | [`t068_4_dumb_loadout_ui.md`](t068_4_dumb_loadout_ui.md) | ✅ shipped `a85f16b` |
| **T-068.5** | [`t068_5_mod_equip_loadout.md`](t068_5_mod_equip_loadout.md) | ✅ shipped `21ec91e` |
| **T-068.5.1** | [`t068_5_1_visual_equip_fix.md`](t068_5_1_visual_equip_fix.md) | ✅ shipped `b233b11` |
| **T-068.6** | [`t068_6_phase1_e2e_gate.md`](t068_6_phase1_e2e_gate.md) | ✅ E2E PASS |

Hub: [`t068_virtual_arsenal_program.md`](t068_virtual_arsenal_program.md)

---

## ORBAT — web status (honest)

**Most ORBAT work is still ahead.** T-008–T-010 shipped **Event attach + inline slot claim + squad hold** — not Eden-grade mission ORBAT authoring.

| Area | Status | Ticket |
|------|--------|--------|
| MC left ORBAT tree | **Partial** — read-only mirror; default squads on drop | **T-071** |
| Squad names / numbering / order / membership | **Not built** | **T-071.1–T-071.2** |
| ORBAT Manager modal | **Not built** | **T-071** — [`t071_orbat_manager_program.md`](t071_orbat_manager_program.md) |
| Event slotting UX + admin | **Partial** | **T-118** |
| Mod slot picker (verify kits) | **Not built** | **T-068.13** (production LOBBY UI; requires **T-092.2**) |

**T-068.13** requires compiled mod `slots[]` (**T-092.2**). **T-071.2** improves squad labels in export but is not required for picker v1.

---

## NOT DONE — T-068+ Eden backlog

Required to place **real objects**, not just generic slots. **Queue and dependencies:** [`docs/TICKET_LEAD.md`](../../TICKET_LEAD.md). **Per-feature status:** [`eden/gap_analysis.md`](eden/gap_analysis.md).

| Ticket | Requirement | Status |
|--------|-------------|--------|
| **T-068** | Virtual Arsenal — Phase 1 shipped; Phase 2 paused | **Phase 2 paused** — resume after **T-090–T-092 + T-071.2 + T-068.13** — [`t068_virtual_arsenal_program.md`](t068_virtual_arsenal_program.md) |
| **T-069** | Markers on map — `addMarker`, render, select, move, delete | **Queued** (benefits from T-091 Z; not a separate gate) |
| **T-070** | Vehicles placeable — `addVehicle`, map layer, drop creates correct kind | **Queued** |
| **T-071** | ORBAT Manager modal — squad names, numbering, membership, slotting order, logos, arsenal | **Queued — blocked on T-092** — [`t071_orbat_manager_program.md`](t071_orbat_manager_program.md) |
| **T-072** | Ctrl multi-place | **Queued** |
| **T-073** | Shift + map rotation | **Queued** |
| **T-074** | Faction submode / catalog filter | **Queued** |
| **T-075** | Spacebar flyTo vs widget | **Queued** |
| **T-076** | Vehicle crew UI | **Queued** |
| **T-077+** | Compositions, triggers, waypoints, … | **Queued** — see TICKET_LEAD |

**T-068 Factions palette @ T-068.3** (`useRegistry` + `buildCatalogTree`, `resource_name` on drop). **T-068.4** dumb loadout export shipped (`a85f16b`). **T-068.5.1** mod equip on **test NPC** shipped (`b233b11`) — **not** human player. **T-068.6** Phase 1 E2E PASS. **Phase 2 paused @ T-068.7**. Full attachment compatibility rules remain Phase 2.

---

## NOT DONE — Loadouts & registry (future program)

**Hardest program.** Separate from “put a unit on the map.” Not ticketed on the active queue yet.

### What “complete” means
- Every gear variant in DB: uniforms, vests, weapon variants, ammo, attachments, grenades, items, vehicle inventories
- Slot compatibility validation (cannot put X on Y)
- Loadout Forge UI (paper doll + search grid)
- Per-slot `loadoutId` → resolved export in `json_payload.loadouts` + human string in `orbat[].loadout`
- Optional: sync with **Mission Armory** totals (aggregate counts for briefing page)

### Prerequisites (all missing)
- **Ingest format** from **Arma Reforger** / modpack export — define JSON schema for one export run
- **Postgres schema** — items, attachments, slot types, compat matrix, modpack version
- **Ingest job** — idempotent upsert per modpack version
- **Registry worker** (frontend) — IndexedDB cache, `canEquip` / `canAttach`
- **Loadout Forge UI** — `ArsenalInspector`, `SoldierDoll`, `ItemPicker`
- **Compiler** — resolve `loadoutId` → classnames for mod export
- **Golden loadout** — one kit exported → correct in Reforger spawn

**Do not start Phase 2 loadout until map verify (T-090–T-092) completes** — otherwise loadouts attach to slots that cannot spawn accurately in-game.

---

## NOT DONE — T-110 terrain base (future)

Millions of read-mostly map props via binary world base + sparse terrain deltas — separate from authored mission entities. Spec: [`t110_terrain_base_mission_layers.md`](t110_terrain_base_mission_layers.md). Runs **after T-090/T-091** hosted world assets + Eden **T-068+** ship.

---

## Current vs target (one glance)

```
TODAY                          TARGET (functional v1)
─────────────────────────────────────────────────────────
Grid map                       Aligned topo/sat map + DEM
Slots only                     Units + vehicles + markers
z = 0 always                   z = DEM sample, editable
Mock catalog                   Registry-backed catalog (shipped T-068.3 @ da78452)
loadout = ''                   Named loadout per slot (loadout program)
editor block positions         Positions verified in-game
Local IndexedDB + manual save  Autosave + semver versions
```

---

## Recommended program order

**Active strategy: map-verify gate** (see §Current strategy). **T-090 → T-091 → T-092** before T-071 and T-068 Phase 2.

| Phase | Deliverable | Depends on |
|-------|-------------|------------|
| **1** | Terrain wired, title hydrate, numeric X/Y | — ✅ **T-049** |
| **1b** | Scale program T-057–T-067 | — ✅ shipped |
| **2** | **T-090** aligned tiles + manifest | Workbench export |
| **3** | **T-091** DEM + Z on place/move | T-090 manifest |
| **4** | **T-092** mod compile + spawn Y/yaw | T-091 |
| **5** | **T-071** ORBAT Manager | T-092 |
| **6** | **T-068.13–.14** player loadout + LOBBY picker | T-071.2 + T-092.2 |
| **7** | **T-069+** markers, vehicles, … | Full **T-068** ship |
| **8** | **T-110** terrain base @ scale | T-090/T-091 |
| **9** | Full item matrix + compiler loadouts (T-068.7–.14) | T-092 + T-071.2 |

Phases **2–4** = **map + accurate positions (tiles, DEM, mod spawn).** Phases **5–6** = **ORBAT + player loadout path.** Phase **7+** = **Eden entity backlog.** Phase **8** = **terrain props at millions scale.**

---

## Deferred mega optimizations (not current work)

**Product decision (2026-06):** T-061 drag-move @ ~360k is **good enough** for now. T-062 shipped interactive bindings @ 360k. Do **not** pursue further render/bindings micro-optimizations until **T-063..T-067**, **T-068+**, and core feature gaps are closed. Revisit only if profiling shows Eden-blocking regressions or scale targets (1M+) demand it.

| Item | Tag / area | What | When |
|------|------------|------|------|
| Typed-array / binary IconLayer buffers | **T-061.1** | GPU-stable buffers instead of JS `SlotIcon[]` rebuilds | After T-062+ if profiling warrants |
| Collapse drag-release to one cache bump | T-061 follow-up | Merge restore + `_patchSlots` into single `iconCacheVersion` tick | Optional polish; known residual |
| Editor session / alt-tab resilience | **T-062.2** ✅ | Warm session + dev Vite guard; spec [`t062_2`](t062_2_editor_session_persistence.md) |
| Full incremental bindings (interactive edits) | **T-062** ✅ | Classifier + O(k) patches for drop/delete/meta/layers; bulk delete ≤10k | **Shipped** — spec [`t062_incremental_bindings.md`](t062_incremental_bindings.md) |
| IDB streaming + Save dedup | **T-062.1** ✅ load / **T-062.1.1** ✅ save | Chunked v2 restore; editor-only Save + Go ORBAT derive | **Both shipped** |
| Spatial index for pick/marquee | **T-063** ✅ | rbush instead of Deck `pickObjects` | **Shipped** — spec [`t063_spatial_index.md`](t063_spatial_index.md) |
| Virtualized outliner | **T-064** ✅ | Sidebar @ 100k+ leaves | **Shipped** — spec [`t064_virtualized_outliner.md`](t064_virtualized_outliner.md) |
| Cluster / LOD extreme zoom | **T-065** ✅ | Pan-stable clusters @ zoom ≤ -4; detail @ -2 | [`t065_cluster_lod.md`](t065_cluster_lod.md) |
| Worker offload compile/export | **T-066** ✅ | `compiler.worker.ts` + `pickMapSnapshot`; Save 201 @ ~367k | [`t066_worker_compile.md`](t066_worker_compile.md) |
| Spatial chunks / lazy regions | **T-067** shipped | Bulk paste + scaffolding; CPU cull deferred; follow-ons **T-111** / **T-112** (`idea`) | [`t067_spatial_chunks.md`](t067_spatial_chunks.md) |
| Terrain base + sparse deltas | **T-110** | Millions of map props (separate from mission layer) | After T-068+ |
| ≤10 s load @ 1M | T-062.1 ✅ + T-066 | Chunked IDB + worker — not drag perf | Stretch north star |

**Do not block Eden or T-065 on the items above.** T-061 + T-062 + T-062.2 + **T-063** + **T-064** closed Eden-blocking interactive edits, session reload, pick/marquee, and outliner @ 360k.

---

## Related docs

All linked in **Documentation** section above. Quick pointers:

| Need | Doc |
|------|-----|
| **Open work queue** | [`docs/TICKET_LEAD.md`](../../TICKET_LEAD.md) |
| Code-evidenced feature list | [`feature_inventory.md`](feature_inventory.md) |
| Eden UI parity backlog | [`eden/gap_analysis.md`](eden/gap_analysis.md) |
| Engineering ADRs + compiler | [`engineering_plan.md`](engineering_plan.md) |
| Agent execution + Decisions log | [`agent_execution.md`](agent_execution.md) |

---

## Open decisions (need human input)

1. **Map assets** — Do we have Everon top-down tiles + heightmap exports, or must we generate them from Reforger/workshop tools? *(Gather in parallel; **implementation deferred** until T-068+ per §Current strategy.)*
2. **Mod JSON contract** — Who provides the golden `json_payload` / spawn format for position + loadout verification?
3. **Arma Reforger / modpack export** — Exact file/API format for ingest (this unlocks loadout program scope: “50 vests” = count rows in export).
4. **Mission Armory vs slot loadouts** — Should Loadout Forge changes update `MissionArmory` quantities automatically, or stay separate?
