**Status:** live

# Mission Creator roadmap

The planning view of the [Mission Creator](/documentation_v2/glossary.md#mission-creator): what the
editor ships today, area by area, the open work that changes it, grouped into tracks, the work
deferred for now, and the questions no [ticket](/documentation_v2/glossary.md#ticket) answers yet.
Every status here is the ticket's `status` field in `.ai/tickets/`; the feature inventory and the
code READMEs hold the detail.

## Recommended next work

`cargo xtask ticket sync` writes the list between the two markers from the whole ticket registry,
editor and platform tickets alike; `cargo xtask ticket check` fails when a marker is missing. The
list is never edited by hand.

<!-- ticket-sync:next:start -->
### Recommended next work (auto-generated)

- **T-940** — Website platform: events, telemetry, admin, content (queued)
- **T-090** — Map visualization program (ready)
- **T-935** — Map binary storage — hybrid rkyv + POD (queued)
- **T-212** — Typed per-side objectives with attributes (ready)
- **T-674** — T-216 follow-on: slot identity reaches the wire (queued)
- **T-675** — Vehicle roster reaches game — T-076 compile half (queued)
- **T-936** — Mission logic the audit found missing (queued)
- **T-290** — Nine dead flatten fields mod never reads (ready)
- **T-941** — Enfusion mod lifecycle: safestart, lobby, screens, HUD (queued)
- **T-937** — Editor data layer: id arrays, undo, persist (queued)
<!-- ticket-sync:next:end -->

## Where the Mission Creator stands

The editor is a Leptos page over the graphics engine's wgpu renderer, a top-down 2D map of the
[mission](/documentation_v2/glossary.md#mission)'s terrain; the
[UX specification](/documentation_v2/website/frontend/apps/editor/ux_spec.md) gives its layout,
gestures and shortcuts. What ships, by area:

| Area | What ships | Detail |
|---|---|---|
| Shell and layout | chromeless route; two 240 px docks that collapse to a stub; the 48 px top strip and the status bar; hidden chrome on Backspace | [shell and layout](/documentation_v2/website/frontend/apps/editor/feature_inventory/shell_route_and_layout.md), [top strip](/documentation_v2/website/frontend/apps/editor/feature_inventory/top_command_strip.md) |
| Map and camera | north-up orthographic map, middle-button pan, wheel zoom about the cursor, 1 km grid, cursor and selection X/Y/Z from the terrain's elevation | [viewport and camera](/documentation_v2/website/frontend/apps/editor/feature_inventory/map_viewport_and_camera.md) |
| Basemap and world | satellite and map basemaps, hillshade, contours, forests, roads, buildings and labels, with per-browser world-layer switches | [basemap and world objects](/documentation_v2/website/frontend/apps/editor/feature_inventory/map_basemap_and_world_objects.md) |
| Placement | characters, vehicles, objects, compositions, markers, triggers and zones from the seven-tab asset browser; the asset picker on a double-click | [placement](/documentation_v2/website/frontend/apps/editor/feature_inventory/placement.md), [asset palette](/documentation_v2/website/frontend/apps/editor/feature_inventory/right_asset_palette.md) |
| Selection | click, Ctrl/Cmd toggle, marquee, select all in view, the mission search and the narrowing chips | [selection](/documentation_v2/website/frontend/apps/editor/feature_inventory/selection.md) |
| Transform | drag-move, the translate and rotate widget, the Z arm, the snap grid, nineteen Arrange commands, Delete | [transform and delete](/documentation_v2/website/frontend/apps/editor/feature_inventory/transform_and_delete.md) |
| Layers and ORBAT | editor layers with hide and lock; the [ORBAT](/documentation_v2/glossary.md#orbat) Manager over sides, squads, [slots](/documentation_v2/glossary.md#slot) and squad vehicles; the faction library | [left dock](/documentation_v2/website/frontend/apps/editor/feature_inventory/left_sidebar.md) |
| Attributes and arsenal | the Attributes dialog, single and multi-edit, with the Transform, Identity and [Arsenal](/documentation_v2/glossary.md#arsenal) tabs and the vehicle heading, cargo and crew | [attributes and settings](/documentation_v2/website/frontend/apps/editor/feature_inventory/attributes_and_settings.md) |
| Mission logic | win conditions, spawn modules, tasks, radio nets, audio, the weather timeline, tactical graphics, connections and comments | [inspectors README](/apps/website/frontend/src/v2/apps/editor/ui/inspector/README.md) |
| Measuring tools | the ruler, the line of sight and the viewshed | [toolbelt](/documentation_v2/website/frontend/apps/editor/feature_inventory/bottom_toolbelt.md) |
| Persistence and versions | the per-account IndexedDB draft, the server hydrate and conflict dialog, one writer tab per mission, Save Version, Export JSON and Export Compiled, the read-only review mode | [persistence and compile](/documentation_v2/website/frontend/apps/editor/feature_inventory/data_persistence_and_compile.md) |
| Scale | windowed outliner trees, slot clustering at far zoom, GPU culling, and world assets streamed around the camera | [performance at scale](/documentation_v2/website/frontend/apps/editor/feature_inventory/performance_at_scale.md) |

The [Eden gap analysis](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/eden_gap_analysis.md)
pairs each feature with its Arma 3 Eden counterpart and names the ticket that closes each gap.

## Open work

Each row links the ticket's spec, or its ticket file when it has none, then its status and plan.

### Editor usability and fixes

| Ticket | Status | What changes |
|---|---|---|
| [T-939 — Editor usability: selection, gizmo, arrange, templates](/documentation_v2/tickets/specs/t939_editor_usability.md) | queued, [plan](/documentation_v2/tickets/plans/t-939_plan.md) | batch faction and squad reassign (T-939.2), Arrange in the context menu with shortcuts (T-939.4), squad templates (T-939.5), canvas error badges and connection wires (T-939.6), a virtualized vehicles panel (T-939.7), Ctrl+F for the document search (T-939.8); all ready |
| [T-704 — Command palette over every editor command](/documentation_v2/tickets/specs/t704_command_palette.md) | ready, [plan](/documentation_v2/tickets/plans/t-704_plan.md) | one searchable list of every editor command |
| [T-158 — Editor shell UX consolidation](/documentation_v2/tickets/specs/t158_editor_shell.md) | ready, [plan](/documentation_v2/tickets/plans/t-158_plan.md) | one settings entry point; the inert top-strip buttons wired |
| [T-142 — MC shell layout polish](/documentation_v2/tickets/specs/t131_north_star_backlog.md) | ready, [plan](/documentation_v2/tickets/plans/t-142_plan.md) | toolbelt placement, Attributes grouping, stub-tool visibility |
| [T-821 — Save version prefill static; second save 409s](/documentation_v2/tickets/specs/t821_save_version_prefill.md) | ready, [plan](/documentation_v2/tickets/plans/t-821_plan.md) | the Save Version pre-fill bumps from the current version |
| [T-823 — OBJ readout must count vehicles or rename honestly](/documentation_v2/tickets/specs/t823_obj_readout_vehicles.md) | ready, [plan](/documentation_v2/tickets/plans/t-823_plan.md) | "OBJ" counts what it names |
| [T-837 — Vehicles cannot be deleted](/documentation_v2/tickets/specs/t837_vehicle_delete.md) | ready, [plan](/documentation_v2/tickets/plans/t-837_plan.md) | Delete removes vehicles |
| [T-845 — A selected vehicle looks identical to an unselected one](/documentation_v2/tickets/specs/t845_selected_vehicle_treatment.md) | ready, [plan](/documentation_v2/tickets/plans/t-845_plan.md) | selected vehicles are highlighted |
| [T-926 — Vehicle Attributes Transform/Position tab](/documentation_v2/tickets/specs/t926_vehicle_transform_tab.md) | ready, [plan](/documentation_v2/tickets/plans/t-926_plan.md) | vehicles gain X, Y, Z and heading fields |
| [T-838 — Map markers selectable; outliner lists; dblclick opens Attributes](/documentation_v2/tickets/specs/t838_marker_select_outliner.md) | ready, [plan](/documentation_v2/tickets/plans/t-838_plan.md) | markers join selection, the outliner and the Attributes dialog |
| [T-831 — Per-side marker authoring audit then explicit UI](/documentation_v2/tickets/specs/t831_per_side_markers_audit.md) | ready, [plan](/documentation_v2/tickets/plans/t-831_plan.md) | markers authored per side |
| [T-828 — Marker captions drift when zoom changes without rebind](/documentation_v2/tickets/specs/t828_marker_caption_drift.md) | ready, [plan](/documentation_v2/tickets/plans/t-828_plan.md) | captions stay on their markers |
| [T-824 — Placed zones must render visibly at rest on map](/documentation_v2/tickets/specs/t824_zone_render_at_rest.md) | ready, [plan](/documentation_v2/tickets/plans/t-824_plan.md) | zones drawn when idle |
| [T-833 — Rotation ring: relative delta plus live preview](/documentation_v2/tickets/specs/t833_rotation_ring_relative.md) | ready, [plan](/documentation_v2/tickets/plans/t-833_plan.md) | the rotate ring turns by the drag's delta, with a preview |
| [T-848 — Group to must use exclusive ORBAT membership](/documentation_v2/tickets/specs/t848_group_to_exclusive_orbat.md), [T-849 — Add ungroup leave-squad verb](/documentation_v2/tickets/specs/t849_ungroup_verb.md), [T-850 — Squad tether must follow drag](/documentation_v2/tickets/specs/t850_squad_tether_drag.md) | ready, plans [848](/documentation_v2/tickets/plans/t-848_plan.md), [849](/documentation_v2/tickets/plans/t-849_plan.md), [850](/documentation_v2/tickets/plans/t-850_plan.md) | squad grouping on the map |
| [T-839 — Retire floating Select/Ruler/LoS bottom-centre pill](/documentation_v2/tickets/specs/t839_retire_floating_pill.md) | ready, [plan](/documentation_v2/tickets/plans/t-839_plan.md) | the mode toolbar goes |
| [T-816 — Armed composition hint; one Esc clears both layers](/documentation_v2/tickets/specs/t816_esc_hint_layer.md) | ready, [plan](/documentation_v2/tickets/plans/t-816_plan.md) | Escape closes one layer at a time |
| [T-817 — Grid labels lag on stationary wheel zoom](/documentation_v2/tickets/specs/t817_grid_label_zoom_lag.md) | ready, [plan](/documentation_v2/tickets/plans/t-817_plan.md) | grid labels follow the zoom |
| [T-820 — Catalog failure generic cause; chips visible wrongly](/documentation_v2/tickets/specs/t820_catalog_failure_cause.md) | ready, [plan](/documentation_v2/tickets/plans/t-820_plan.md) | the catalog failure names its cause |
| [T-822 — Outliner dblclick must not open asset picker](/documentation_v2/tickets/specs/t822_outliner_dblclick_bubble.md), [T-927 — Editor chrome dblclick leak to map](/documentation_v2/tickets/specs/t927_chrome_dblclick_leak.md) | ready, plans [822](/documentation_v2/tickets/plans/t-822_plan.md), [927](/documentation_v2/tickets/plans/t-927_plan.md) | a double-click on the chrome stays off the map |
| [T-827 — Validation chip red under 4.5:1](/documentation_v2/tickets/specs/t827_validation_chip_contrast.md), [T-830 — Outliner rows cramped](/documentation_v2/tickets/specs/t830_outliner_density.md), [T-841 — Type picker popover translucent](/documentation_v2/tickets/specs/t841_type_picker_opaque.md) | ready, plans [827](/documentation_v2/tickets/plans/t-827_plan.md), [830](/documentation_v2/tickets/plans/t-830_plan.md), [841](/documentation_v2/tickets/plans/t-841_plan.md) | contrast and density of the chrome |
| [T-834 — Wave-205 residue](/documentation_v2/tickets/specs/t834_wave205_residue.md), [T-653 — Preserve the three headless editor-screenshot findings](/documentation_v2/tickets/specs/t653_headless_screenshot.md) | ready, plans [834](/documentation_v2/tickets/plans/t-834_plan.md), [653](/documentation_v2/tickets/plans/t-653_plan.md) | stale comments and dead code; the screenshot runbook |

### Mission data and the wire to the game

| Ticket | Status | What changes |
|---|---|---|
| [T-212 — Typed per-side objectives with attributes](/documentation_v2/tickets/specs/t212_typed_objectives.md) | ready, [plan](/documentation_v2/tickets/plans/t-212_plan.md) | objectives become typed, placed, per-side entities |
| [T-290 — Nine dead flatten fields mod never reads](/documentation_v2/tickets/specs/t290_dead_flatten_fields.md) | ready, [plan](/documentation_v2/tickets/plans/t-290_plan.md) | the [mod](/documentation_v2/glossary.md#mod) reads, or the compiler drops, nine compiled fields |
| [T-674 — Slot identity reaches the wire](/documentation_v2/tickets/specs/t674_slot_identity_wire.md) | queued, [plan](/documentation_v2/tickets/plans/t-674_plan.md) | slot identity fields reach the compiled document |
| [T-675 — Vehicle roster reaches game](/documentation_v2/tickets/specs/t675_vehicle_roster_wire.md) | queued, [plan](/documentation_v2/tickets/plans/t-675_plan.md) | placed vehicles and crews reach the compiled document |
| [T-932 — Parked briefing markers survive server save/reload](/documentation_v2/tickets/specs/t932_parked_markers_persist.md) | queued, [plan](/documentation_v2/tickets/plans/t-932_plan.md) | parked briefing markers survive a save and reload |
| [T-309 — FactionDoc squad level for Apply Template](/documentation_v2/tickets/specs/t309_faction_doc_squads.md) | ready, [plan](/documentation_v2/tickets/plans/t-309_plan.md) | faction templates keep their squads |
| [T-146 — Asset Browser Data Wiring](/documentation_v2/tickets/specs/t146_asset_browser_data_wiring.md) | ready, [plan](/documentation_v2/tickets/plans/t-146_plan.md) | [registry](/documentation_v2/glossary.md#registry) vehicles and crates placeable from the asset browser |
| [T-140 — Mission client payload budget](/documentation_v2/tickets/specs/t131_north_star_backlog.md) | ready, [plan](/documentation_v2/tickets/plans/t-140_plan.md) | compile reports a payload-budget diagnostic |
| [T-141 — Procedural slot naming](/documentation_v2/tickets/specs/t131_north_star_backlog.md) | ready, [plan](/documentation_v2/tickets/plans/t-141_plan.md) | generated slot display names with a manual override |
| [T-068.14 — Phase 2 E2E gate editor to player](/documentation_v2/tickets/specs/t068_14_phase2_e2e_gate.md) | queued, no plan | a human sign-off from an editor loadout to a dressed player in game |

### Map and terrain

| Ticket | Status | What changes |
|---|---|---|
| [T-090 — Map visualization program](/documentation_v2/tickets/specs/t090_091_map_terrain_program.md) | ready, no plan | the open children below |
| [T-090.4 — Z placement audit](/documentation_v2/tickets/specs/t090_4_z_placement_audit.md), [T-090.6 — Geometry-aware placement audit](/documentation_v2/tickets/specs/t090_6_geometry_placement_audit.md) | ready, plans [090.4](/documentation_v2/tickets/plans/t-090_4_plan.md), [090.6](/documentation_v2/tickets/plans/t-090_6_plan.md) | buried and floating objects found automatically |
| [T-090.7 — Eden AI world object schema](/documentation_v2/tickets/specs/t090_eden_ai_world_object_schema.md) | ready, [plan](/documentation_v2/tickets/plans/t-090_7_plan.md) | the exact field contract of a world object |
| [T-090.9 — World-object interaction](/documentation_v2/tickets/specs/t090_9_world_object_interaction.md) | ready, [plan](/documentation_v2/tickets/plans/t-090_9_plan.md) | hover, inspect, filter and legend for world objects |
| [T-090.10.2 — Map Engine v2 cleanup](/documentation_v2/tickets/specs/t090_10_map_engine_v2.md) | ready, [plan](/documentation_v2/tickets/plans/t-090_10_2_plan.md) | the legacy map-view branches and tile fallback go |
| [T-090.12.7 — Docs pass: LOS tool, world-los bench, map-assets, MCP](/documentation_v2/tickets/specs/t090_091_map_terrain_program.md) | ready, [plan](/documentation_v2/tickets/plans/t-090_12_7_plan.md) | documentation of the line-of-sight tool and the map assets |
| [T-129 — Building floor selector](/documentation_v2/tickets/specs/t090_eden_map_reference.md) | ready, [plan](/documentation_v2/tickets/plans/t-129_plan.md) | placement on a chosen floor of a building |
| [T-143 — Water mask placement guard and exact hydrology](/documentation_v2/tickets/specs/t090_091_map_terrain_program.md) | ready, [plan](/documentation_v2/tickets/plans/t-143_plan.md) | placement refuses the ocean and warns in lakes |
| [T-294 — Arland has a manifest and no object data](/documentation_v2/tickets/specs/t294_arland_objects.md) | ready, [plan](/documentation_v2/tickets/plans/t-294_plan.md) | Arland gets its world objects |
| [T-131 — Route planner tool](/documentation_v2/tickets/specs/t131_north_star_backlog.md) | ready, [plan](/documentation_v2/tickets/plans/t-131_plan.md) | routes planned on the road graph, with distance and profile |

### Map asset storage

| Ticket | Status | What changes |
|---|---|---|
| [T-935 — Map binary storage — hybrid rkyv + POD](/documentation_v2/tickets/specs/t935_map_binary_storage.md) | queued, [plan](/documentation_v2/tickets/plans/t-935_plan.md) | T-935.1 to T-935.14 shipped; T-935.16 (ready) finishes the gz-JSON cutover; T-935.15 and T-935.17 to T-935.20 (queued) replace the chunk files with one object container, a spatial index and range fetches |

### Collaboration

| Ticket | Status | What changes |
|---|---|---|
| [T-295 — Realtime collaborative editing](/documentation_v2/tickets/specs/t295_realtime_collab.md) | ready, [plan](/documentation_v2/tickets/plans/t-295_plan.md) | several authors edit one mission live over a websocket |
| [T-132 — Multiplayer MC + visual git](/documentation_v2/tickets/specs/t131_north_star_backlog.md) | ready, [plan](/documentation_v2/tickets/plans/t-132_plan.md) | an entity-level diff between two versions, for review before publishing |

### Programs whose children have all shipped

[T-936 — Mission logic the audit found missing](/documentation_v2/tickets/specs/t936_mission_logic.md),
[T-937 — Editor data layer: id arrays, undo, persist](/documentation_v2/tickets/specs/t937_editor_data_layer.md) and
[T-938 — Engine and wasm performance](/documentation_v2/tickets/specs/t938_engine_perf.md) are queued, while every child of
each has shipped; they wait only for their own closure.

### Idea-stage findings

The code audits filed these as `idea` tickets, each without a spec: the status bar's dead "OPEN"
button (T-1033), the hidden side census (T-1034), multi-folder drop onto the dock header
(T-1032), the paper-doll hotspots and the keyboard (T-1035), dead frontend code (T-1043), stale
doc comments (T-1045), compile findings misfiling refused blocks (T-1046), validation policy for
cargo and loadouts (T-1047), block checks against the mission schema (T-1048), stale connections
after a re-hydrate (T-1050), minted-id collisions (T-1051), placement scatter across releases
(T-1052), one duplicate-slot check for upload and save (T-1053), unused document store helpers
(T-1054), the basemap switch back to satellite (T-1058), terrain-size-derived map layers (T-1062)
and hosted commands on no-op edits (T-1077); and, beyond the editor, frontend pages importing the
editor (T-1005).

## Deferred work

| Ticket | What waits |
|---|---|
| [T-068 — Virtual Arsenal](/documentation_v2/tickets/specs/t068_virtual_arsenal_program.md) | the program closes with T-068.14, a human two-client sign-off |
| [T-110 — Terrain base + sparse deltas](/documentation_v2/tickets/specs/t110_terrain_base_mission_layers.md) | a binary terrain base with sparse prop deltas for a million or more map objects, kept apart from the authored mission layer |
| [T-090.3](/documentation_v2/tickets/specs/t090_3_map_asset_export.md), [T-090.5](/documentation_v2/tickets/specs/t090_5_map_object_render_layer.md), [T-090.8](/documentation_v2/tickets/specs/t090_8_forest_vegetation_regions.md) | the remaining map asset export, world-object render layer and forest region slices |
| [T-121 — Terrain DEM export automation](/documentation_v2/tickets/specs/t121_terrain_dem_export_automation.md) | an automated elevation export |
| [T-083](/.ai/tickets/T-083.toml), [T-093](/.ai/tickets/T-093.toml), [T-094](/.ai/tickets/T-094.toml) | an Eden-style full menu bar, autosave feedback polish, typed-array icon buffers |
| [T-205](/.ai/tickets/T-205.toml), [T-206](/.ai/tickets/T-206.toml), [T-652](/.ai/tickets/T-652.toml) | vehicle seats and turrets, the empty item data, rock rendering |
| T-712 to T-734, T-825, T-829, T-835, T-840, T-844, T-846, T-847, T-851, T-852 | deferred editor defects from the headless audits, each in its ticket file |
| [T-1000 — Rename scenario to mission across code, data and mod](/.ai/tickets/T-1000.toml) | code identifiers that still say scenario |

## Open questions

- Mission armory and slot loadouts: the armory (`GET` and `PUT /api/v1/missions/{id}/armory`) is a
  separate list that a write replaces whole, and nothing derives it from the slots' loadouts.
  Whether loadout edits in the Arsenal update the armory's quantities, or the two stay apart, is
  not decided and no ticket covers it.

## Related documentation

- [Mission Creator UX specification](/documentation_v2/website/frontend/apps/editor/ux_spec.md) —
  the layout, the gestures, the shortcuts and the save flow.
- [Mission Creator feature inventory](/documentation_v2/website/frontend/apps/editor/feature_inventory/README.md) — every feature by area, with its status in
  the code.
- [Mission Creator decisions](/documentation_v2/website/frontend/apps/editor/decisions.md) — the
  dated decisions behind the editor.
- [Eden gap analysis](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/eden_gap_analysis.md)
  — parity with Arma 3 Eden, feature by feature.
- [Mission Creator code](/apps/website/frontend/src/v2/apps/editor/README.md) — the editor's folders and boundaries.
