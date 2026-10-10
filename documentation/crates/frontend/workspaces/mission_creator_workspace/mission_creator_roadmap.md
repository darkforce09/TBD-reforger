**Status:** live

# Mission Creator roadmap

The planning view of the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator): what the
editor ships today, area by area, the open work that changes it, grouped into tracks, the work
deferred for now, and the questions no [ticket](/documentation/glossary/n_to_z.md#ticket) answers yet.
Every ticket here is named by its slug in the central ticket manager (`ttm --project reforger show
<ticket>`), which holds its status; the feature inventory and the code READMEs hold the detail.

## Recommended next work

The central ticket manager orders the open work, editor and platform tickets alike:
`ttm --project reforger next` prints the active tickets and the next ones to take.

## Where the Mission Creator stands

The editor is a Leptos page over the graphics engine's wgpu renderer, a top-down 2D map of the
[mission](/documentation/glossary/g_to_m.md#mission)'s terrain; the
[UX specification](/documentation/crates/frontend/workspaces/mission_creator_workspace/ux_spec.md) gives its layout,
gestures and shortcuts. What ships, by area:

| Area | What ships | Detail |
|---|---|---|
| Shell and layout | chromeless route; two 240 px docks that collapse to a stub; the 48 px top strip and the status bar; hidden chrome on Backspace | [shell and layout](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/shell_route_and_layout.md), [top strip](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/top_command_strip.md) |
| Map and camera | north-up orthographic map, middle-button pan, wheel zoom about the cursor, 1 km grid, cursor and selection X/Y/Z from the terrain's elevation | [viewport and camera](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/map_viewport_and_camera.md) |
| Basemap and world | satellite and map basemaps, hillshade, contours, forests, roads, buildings and labels, with per-browser world-layer switches | [basemap and world objects](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/map_basemap_and_world_objects.md) |
| Placement | characters, vehicles, objects, compositions, markers, triggers and zones from the seven-tab asset browser; the asset picker on a double-click | [placement](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/placement.md), [asset palette](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/right_asset_palette.md) |
| Selection | click, Ctrl/Cmd toggle, marquee, select all in view, the mission search and the narrowing chips | [selection](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/selection.md) |
| Transform | drag-move, the translate and rotate widget, the Z arm, the snap grid, nineteen Arrange commands, Delete | [transform and delete](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/transform_and_delete.md) |
| Layers and ORBAT | editor layers with hide and lock; the [ORBAT](/documentation/glossary/n_to_z.md#orbat) Manager over sides, squads, [slots](/documentation/glossary/n_to_z.md#slot) and squad vehicles; the faction library | [left dock](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/left_sidebar.md) |
| Attributes and arsenal | the Attributes dialog, single and multi-edit, with the Transform, Identity and [Arsenal](/documentation/glossary/a_to_f.md#arsenal) tabs and the vehicle heading, cargo and crew | [attributes and settings](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/attributes_and_settings.md) |
| Mission logic | win conditions, spawn modules, tasks, radio nets, audio, the weather timeline, tactical graphics, connections and comments | [inspectors README](/crates/frontend/workspaces/mission_creator_workspace/src/ui/inspector/README.md) |
| Measuring tools | the ruler, the line of sight and the viewshed | [toolbelt](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/bottom_toolbelt.md) |
| Persistence and versions | the per-account IndexedDB draft, the server hydrate and conflict dialog, one writer tab per mission, Save Version, Export JSON and Export Compiled, the read-only review mode | [persistence and compile](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/data_persistence_and_compile.md) |
| Scale | windowed outliner trees, slot clustering at far zoom, GPU culling, and world assets streamed around the camera | [performance at scale](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/performance_at_scale.md) |

The [Eden gap analysis](/documentation/crates/frontend/workspaces/mission_creator_workspace/eden_editor_reference/eden_gap_analysis.md)
pairs each feature with its Arma 3 Eden counterpart and names the ticket that closes each gap.

## Open work

Each row names its ticket by slug; the ticket manager (`ttm`) holds its status, spec and plan.

### Editor usability and fixes

| Ticket | What changes |
|---|---|
| Editor usability: selection, gizmo, arrange, templates (`editor-usability-selection-gizmo`) | batch faction and squad reassign (`editor-usability-selection-gizmo.batch-faction-squad-reassign`), Arrange in the context menu with shortcuts (`editor-usability-selection-gizmo.arrange-tools-context-menu`), squad templates (`editor-usability-selection-gizmo.faction-catalog-squad-templates`), canvas error badges and connection wires (`editor-usability-selection-gizmo.error-badges-connection-wires`), a virtualized vehicles panel (`editor-usability-selection-gizmo.vehicles-panel-virtualization-memoized`), Ctrl+F for the document search (`editor-usability-selection-gizmo.ctrl-f-focuses-document`) |
| Command palette over every editor command (`command-palette-over-editor`) | one searchable list of every editor command |
| Editor shell UX consolidation (`editor-shell-ux-consolidation`) | one settings entry point; the inert top-strip buttons wired |
| MC shell layout polish (`mc-shell-layout-polish`) | toolbelt placement, Attributes grouping, stub-tool visibility |
| Save version prefill static; second save 409s (`save-version-prefill-static`) | the Save Version pre-fill bumps from the current version |
| OBJ readout must count vehicles or rename honestly (`obj-readout-must-count`) | "OBJ" counts what it names |
| Vehicles cannot be deleted (`vehicles-cannot-deleted-slots`) | Delete removes vehicles |
| A selected vehicle looks identical to an unselected one (`selected-vehicle-looks-identical`) | selected vehicles are highlighted |
| Vehicle Attributes Transform/Position tab (`vehicle-attributes-transform-position`) | vehicles gain X, Y, Z and heading fields |
| Map markers selectable; outliner lists; dblclick opens Attributes (`map-markers-selectable-outliner`) | markers join selection, the outliner and the Attributes dialog |
| Per-side marker authoring audit then explicit UI (`side-marker-authoring-audit`) | markers authored per side |
| Marker captions drift when zoom changes without rebind (`marker-captions-drift-when`) | captions stay on their markers |
| Placed zones must render visibly at rest on map (`placed-zones-must-render`) | zones drawn when idle |
| Rotation ring: relative delta plus live preview (`rotation-ring-relative-delta`) | the rotate ring turns by the drag's delta, with a preview |
| Group to must use exclusive ORBAT membership (`group-must-use-exclusive`), Add ungroup leave-squad verb (`add-ungroup-leave-squad`), Squad tether must follow drag (`squad-tether-must-follow`) | squad grouping on the map |
| Retire floating Select/Ruler/LoS bottom-centre pill (`retire-floating-select-ruler`) | the mode toolbar goes |
| Armed composition hint; one Esc clears both layers (`armed-composition-hint-open`) | Escape closes one layer at a time |
| Grid labels lag on stationary wheel zoom (`grid-labels-lag-1`) | grid labels follow the zoom |
| Catalog failure generic cause; chips visible wrongly (`catalog-failure-generic-cause`) | the catalog failure names its cause |
| Outliner dblclick must not open asset picker (`outliner-dblclick-must-not`), Editor chrome dblclick leak to map (`editor-chrome-dblclick-leak`) | a double-click on the chrome stays off the map |
| Validation chip red under 4.5:1 (`validation-chip-red-under`), Outliner rows cramped (`outliner-rows-cramped-density`), Type picker popover translucent (`type-picker-popover-translucent`) | contrast and density of the chrome |
| Wave-205 residue (`wave-205-residue-two`), Preserve the three headless editor-screenshot findings (`preserve-three-headless-editor`) | stale comments and dead code; the screenshot runbook |

### Mission data and the wire to the game

| Ticket | What changes |
|---|---|
| Typed per-side objectives with attributes (`typed-side-objectives-attributes`) | objectives become typed, placed, per-side entities |
| Nine dead flatten fields mod never reads (`nine-dead-flatten-fields`) | the [mod](/documentation/glossary/g_to_m.md#mod) reads, or the compiler drops, nine compiled fields |
| Slot identity reaches the wire (`slot-identity-reaches-wire`) | slot identity fields reach the compiled document |
| Vehicle roster reaches game (`vehicle-roster-reaches-game`) | placed vehicles and crews reach the compiled document |
| Parked briefing markers survive server save/reload (`parked-briefing-markers-survive`) | parked briefing markers survive a save and reload |
| FactionDoc squad level for Apply Template (`factiondoc-squad-level-apply`) | faction templates keep their squads |
| Asset Browser Data Wiring (`asset-browser-data-wiring`) | [registry](/documentation/glossary/n_to_z.md#registry) vehicles and crates placeable from the asset browser |
| Mission client payload budget (`mission-client-payload-budget`) | compile reports a payload-budget diagnostic |
| Procedural slot naming (`procedural-slot-naming`) | generated slot display names with a manual override |
| Phase 2 E2E gate editor to player (`virtual-arsenal.phase-2-e2e-gate`) | a human sign-off from an editor loadout to a dressed player in game |

### Map and terrain

| Ticket | What changes |
|---|---|
| Map visualization program (`map-visualization-program`) | the open children below |
| Z placement audit (`map-visualization-program.z-placement-audit`), Geometry-aware placement audit (`map-visualization-program.geometry-aware-placement-audit`) | buried and floating objects found automatically |
| Eden AI world object schema (`map-visualization-program.eden-ai-world-object`) | the exact field contract of a world object |
| World-object interaction (`map-visualization-program.world-object-interaction`) | hover, inspect, filter and legend for world objects |
| Map Engine v2 cleanup (`map-visualization-program.engine-v2-delete`) | the legacy map-view branches and tile fallback go |
| Docs pass: LOS tool, world-los bench, map-assets, MCP (`map-visualization-program.docs-pass-los-tool`) | documentation of the line-of-sight tool and the map assets |
| Building floor selector (`building-floor-selector`) | placement on a chosen floor of a building |
| Water mask placement guard and exact hydrology (`water-mask-placement-guard`) | placement refuses the ocean and warns in lakes |
| Arland has a manifest and no object data (`arland-has-manifest-no`) | Arland gets its world objects |
| Route planner tool (`route-planner-tool`) | routes planned on the road graph, with distance and profile |

### Map asset storage

| Ticket | What changes |
|---|---|
| Map binary storage — hybrid rkyv + POD (`map-binary-storage-hybrid`) | `map-binary-storage-hybrid.finish-gz-cutover-left` finishes the gz-JSON cutover; its remaining children replace the chunk files with one object container, a spatial index and range fetches |

### Collaboration

| Ticket | What changes |
|---|---|
| Realtime collaborative editing (`realtime-collaborative-editing`) | several authors edit one mission live over a websocket |
| Multiplayer MC + visual git (`multiplayer-mc-visual-git`) | an entity-level diff between two versions, for review before publishing |

### Idea-stage findings

The code audits filed these tickets, each without a spec: the status bar's dead "OPEN"
button (`fix-editor-status-bar`), the hidden side census (`fix-top-strip-slot`), multi-folder drop
onto the dock header (`fix-multi-folder-drop`), the paper-doll hotspots and the keyboard
(`fix-arsenal-paper-doll`), dead frontend code (`remove-dead-frontend-code`), stale doc comments
(`rewrite-stale-frontend-doc`), compile findings misfiling refused blocks
(`fix-compile-findings-misfiling`), validation policy for cargo and loadouts
(`decide-whether-mission-creator`), block checks against the mission schema
(`check-align-mission-block`), stale connections after a re-hydrate (`fix-mission-re-hydrate`),
minted-id collisions (`check-whether-minted-vehicle`), placement scatter across releases
(`decide-whether-placement-scatter`), one duplicate-slot check for upload and save
(`decide-whether-mission-upload`), unused document store helpers (`remove-unused-mission-document`),
the basemap switch back to satellite (`fix-map-basemap-switch`), terrain-size-derived map layers
(`derive-map-grid-basemap`) and hosted commands on no-op edits (`check-hosted-commands-running`);
and, beyond the editor, frontend pages importing the editor (`refactor-frontend-core-pages`).

## Deferred work

| Ticket | What waits |
|---|---|
| Virtual Arsenal (`virtual-arsenal`) | the program closes with `virtual-arsenal.phase-2-e2e-gate`, a human two-client sign-off |
| Terrain base + sparse deltas (`terrain-base-sparse-deltas`) | a binary terrain base with sparse prop deltas for a million or more map objects, kept apart from the authored mission layer |
| `map-visualization-program.asset-export`, `map-visualization-program.object-render-layer`, `map-visualization-program.forest-vegetation-regions` | the remaining map asset export, world-object render layer and forest region slices |
| Terrain DEM export automation (`terrain-dem-export-automation`) | an automated elevation export |
| `top-menu-bar`, `continuous-autosave-polish`, `typed-array-iconlayer` | an Eden-style full menu bar, autosave feedback polish, typed-array icon buffers |
| `vehicle-seats-crew-roles`, `item-data-78-empty`, `rocks-not-rendered` | vehicle seats and turrets, the empty item data, rock rendering |
| the deferred editor defect tickets in `ttm` | deferred editor defects from the headless audits |
| Rename scenario to mission across code, data and mod (`rename-scenario-mission-across`) | code identifiers that still say scenario |

## Open questions

- Mission armory and slot loadouts: the armory (`GET` and `PUT /api/v1/missions/{id}/armory`) is a
  separate list that a write replaces whole, and nothing derives it from the slots' loadouts.
  Whether loadout edits in the Arsenal update the armory's quantities, or the two stay apart, is
  not decided and no ticket covers it.

## Related documentation

- [Mission Creator UX specification](/documentation/crates/frontend/workspaces/mission_creator_workspace/ux_spec.md) —
  the layout, the gestures, the shortcuts and the save flow.
- [Mission Creator feature inventory](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/README.md) — every feature by area, with its status in
  the code.
- [Mission Creator decisions](/documentation/crates/frontend/workspaces/mission_creator_workspace/decisions.md) — the
  dated decisions behind the editor.
- [Eden gap analysis](/documentation/crates/frontend/workspaces/mission_creator_workspace/eden_editor_reference/eden_gap_analysis.md)
  — parity with Arma 3 Eden, feature by feature.
- [Mission Creator code](/crates/frontend/workspaces/mission_creator_workspace/src/README.md) — the editor's folders and boundaries.
