# Documentation tag glossary

Use the correct prefix everywhere. **Do not reuse T-0xx for frontend deferred work.**

## Naming convention (T-045)

Every domain has exactly one **`ROADMAP.md`** — the AI/human entry point for planning and doc links.

| Domain | Path |
|--------|------|
| Platform hub | [`docs/README.md`](README.md) → points to domain ROADMAPs |
| Frontend | [`docs/frontend/ROADMAP.md`](frontend/ROADMAP.md) |
| Backend | [`docs/backend/ROADMAP.md`](backend/ROADMAP.md) |
| Mission Creator | [`Design_Docs/Mission_Creator_Architecture/ROADMAP.md`](../Design_Docs/Mission_Creator_Architecture/ROADMAP.md) |

Supporting docs use **descriptive snake_case** filenames (no numeric prefixes): `engineering_plan.md`, `ux_spec.md`, `agent_execution.md`, etc. Old numbered paths redirect via stubs.

## Tag prefixes

| Prefix | Meaning | Where |
|--------|---------|-------|
| **T-0xx** | Platform git milestones | [`CLAUDE.md`](../CLAUDE.md) §Status |
| **T-048** | Library create dialog (shipped) | [`t048_library_create_dialog.md`](../Design_Docs/Mission_Creator_Architecture/t048_library_create_dialog.md) |
| **T-049** | Track A quick P0 (shipped) | [`t049_track_a_quick_p0.md`](../Design_Docs/Mission_Creator_Architecture/t049_track_a_quick_p0.md) |
| **T-050** | Cursor Z readout (shipped) | [`t050_cursor_z_readout.md`](../Design_Docs/Mission_Creator_Architecture/t050_cursor_z_readout.md) |
| **T-052** | Eden P1 undo keyboard shortcuts (shipped) | [`t052_eden_p1_undo_shortcuts.md`](../Design_Docs/Mission_Creator_Architecture/t052_eden_p1_undo_shortcuts.md) |
| **T-053** | Eden P1 Ctrl/Cmd+LMB additive toggle select (shipped) | [`t053_eden_p1_additive_select.md`](../Design_Docs/Mission_Creator_Architecture/t053_eden_p1_additive_select.md) |
| **T-054** | Eden P1 Attributes entry points — map dbl-click + ORBAT tree (shipped) | [`t054_attributes_entry_points.md`](../Design_Docs/Mission_Creator_Architecture/t054_attributes_entry_points.md) |
| **T-055** | Eden P1 asset browser search (shipped) | [`t055_asset_browser_search.md`](../Design_Docs/Mission_Creator_Architecture/t055_asset_browser_search.md) |
| **T-056** | Eden P1 Ctrl+C/V copy-paste at cursor (slots) (shipped) | [`t056_eden_p1_copy_paste.md`](../Design_Docs/Mission_Creator_Architecture/t056_eden_p1_copy_paste.md) |
| **T-057** | Map perf hotfix — ≥55 fps @ 200+ slots (shipped) | [`t057_map_performance_hotfix.md`](../Design_Docs/Mission_Creator_Architecture/t057_map_performance_hotfix.md) |
| **T-058** | Toolbelt OBJ/SEL entity counts (shipped) | [`t058_entity_count_readout.md`](../Design_Docs/Mission_Creator_Architecture/t058_entity_count_readout.md) |
| **T-059** | Bulk paste/delete at scale — validated **360k @ 100+ fps** pan (shipped) | [`t059_bulk_paste_operations.md`](../Design_Docs/Mission_Creator_Architecture/t059_bulk_paste_operations.md) |
| **T-060** | Fast load + save @ ~360k — shipped `b1fd25a` (browser ~142 MB → 201 + curl 140 MB → 201) | [`t060_fast_initial_load.md`](../Design_Docs/Mission_Creator_Architecture/t060_fast_initial_load.md) · [`t060_1_scale_load_save_completion.md`](../Design_Docs/Mission_Creator_Architecture/t060_1_scale_load_save_completion.md) |
| **T-060.1** | Determinate load + save upload timeouts (shipped in T-060) | [`t060_1_scale_load_save_completion.md`](../Design_Docs/Mission_Creator_Architecture/t060_1_scale_load_save_completion.md) |
| **T-060.1.1** | IDB restoring phase + paint fix (shipped; load partial pass @ ~360k) | [`t060_1_scale_load_save_completion.md`](../Design_Docs/Mission_Creator_Architecture/t060_1_scale_load_save_completion.md) §T-060.1.1 |
| **T-060.1.2** | Save upload fixes — E1/E2/E3b (shipped in T-060) | [`t060_1_scale_load_save_completion.md`](../Design_Docs/Mission_Creator_Architecture/t060_1_scale_load_save_completion.md) §T-060.1.2 |
| **T-060.1.3** | Save observability — measured size + debug (shipped; failure diagnosed @ 367k) | [`t060_1_scale_load_save_completion.md`](../Design_Docs/Mission_Creator_Architecture/t060_1_scale_load_save_completion.md) §T-060.1.3 |
| **T-060.1.4** | Fix mid-upload socket reset @ ~135 MB (shipped; hardened `GlobalBodyLimit` skip + production-like IT) | [`t060_1_scale_load_save_completion.md`](../Design_Docs/Mission_Creator_Architecture/t060_1_scale_load_save_completion.md) §T-060.1.4 |
| **T-061** | Drag-move @ 360k — dual IconLayer + `slotIconCache` + slot fast path (**shipped — good enough**) | [`t061_drag_move_hotfix.md`](../Design_Docs/Mission_Creator_Architecture/t061_drag_move_hotfix.md) |
| **T-061.0** | (sub) Motion hotfix — dual IconLayer + split drag state (**shipped**) | [`t061_drag_move_hotfix.md`](../Design_Docs/Mission_Creator_Architecture/t061_drag_move_hotfix.md) §T-061.0 |
| **T-061.0.1** | (sub) Boundary hotfix — `slotIconCache` + bindings fast path (**shipped**) | [`t061_drag_move_hotfix.md`](../Design_Docs/Mission_Creator_Architecture/t061_drag_move_hotfix.md) §T-061.0.1 |
| **T-061.1** | Typed-array IconLayer (**deferred** — mega optimizations backlog) | MC [`ROADMAP.md`](../Design_Docs/Mission_Creator_Architecture/ROADMAP.md) §Deferred mega optimizations |
| **T-062** | Incremental bindings @ 360k — classifier + bulk delete (**shipped**) | [`t062_incremental_bindings.md`](../Design_Docs/Mission_Creator_Architecture/t062_incremental_bindings.md) |
| **T-062.0** | (sub) `incPatchPlan` + O(k) store patches (**shipped**) | [`t062_incremental_bindings.md`](../Design_Docs/Mission_Creator_Architecture/t062_incremental_bindings.md) §T-062.0 |
| **T-062.0.1** | (sub) Batched delete + `slotCount`/`slotsRevision` (**shipped**) | [`t062_incremental_bindings.md`](../Design_Docs/Mission_Creator_Architecture/t062_incremental_bindings.md) §T-062.0.1 |
| **T-062.2** | Editor session / alt-tab resilience — Vite reload guard + warm session (**shipped**) | [`t062_2_editor_session_persistence.md`](../Design_Docs/Mission_Creator_Architecture/t062_2_editor_session_persistence.md) |
| **T-062.1** | Chunked IDB slot restore (**shipped**) | [`t062_1_idb_streaming_load.md`](../Design_Docs/Mission_Creator_Architecture/t062_1_idb_streaming_load.md) |
| **T-062.1.1** | Save orbat dedup (**shipped**) | [`t062_1_1_batch_save.md`](../Design_Docs/Mission_Creator_Architecture/t062_1_1_batch_save.md) |
| **T-063** | Spatial index — rbush pick/marquee (**shipped**) | [`t063_spatial_index.md`](../Design_Docs/Mission_Creator_Architecture/t063_spatial_index.md) |
| **T-064** ✅ | Scale program | Virtualized outliner @ ~367k — **shipped** | MC [`ROADMAP.md`](../Design_Docs/Mission_Creator_Architecture/ROADMAP.md) §Map performance |
| **T-065** ✅ | Scale program | Cluster/LOD @ extreme zoom — **shipped** | [`t065_cluster_lod.md`](../Design_Docs/Mission_Creator_Architecture/t065_cluster_lod.md) |
| **T-066..T-067** | Scale program → 1M–10M (**active**) | Worker, chunks | MC [`ROADMAP.md`](../Design_Docs/Mission_Creator_Architecture/ROADMAP.md) §Map performance |
| **T-070+** | Terrain base + sparse deltas (future — millions of map props) | [`t070_terrain_base_mission_layers.md`](../Design_Docs/Mission_Creator_Architecture/t070_terrain_base_mission_layers.md) |
| **T-051** | Title PATCH sync (**deferred**, not started) | [`t049_track_a_quick_p0.md`](../Design_Docs/Mission_Creator_Architecture/t049_track_a_quick_p0.md) amendment |
| **FD-0xx** | Frontend deferred work | [`frontend/docs/TRACKING.md`](../frontend/docs/TRACKING.md) |
| **BE-0xx** | Backend deferred work | [`docs/backend/ROADMAP.md`](backend/ROADMAP.md) |
| **P0–P3** | Eden parity backlog | MC `eden/gap_analysis.md` |
| **A / B / C** | MC functional tracks | MC [`ROADMAP.md`](../Design_Docs/Mission_Creator_Architecture/ROADMAP.md) |

## T-0xx vs engineering phases

[`CLAUDE.md`](../CLAUDE.md) T-029–T-040 = git milestones.  
[`engineering_plan.md`](../Design_Docs/Mission_Creator_Architecture/engineering_plan.md) phases 0–9 = design doc — not 1:1.

## Historical commits

Frontend TRACKING once used T-0xx for deferred items — those are **FD-0xx** from T-043 onward.
