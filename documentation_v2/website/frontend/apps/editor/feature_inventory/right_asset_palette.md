**Status:** live

# Right asset palette

The [Mission Creator](/documentation_v2/glossary.md#mission-creator)'s right dock, its asset
browser: seven tabs from which a mission maker picks characters, vehicles, objects, saved
compositions, briefing markers and zone and trigger areas for the
[mission](/documentation_v2/glossary.md#mission), with the side chips, the search, the favourites
and the faction library beside them.

## Where it lives

- Code: [`apps/website/frontend/src/v2/apps/editor/ui/docks/dock_right/`](/apps/website/frontend/src/v2/apps/editor/ui/docks/dock_right/README.md)
  (`shell/` holds `DockRight`, the tab strip and the Factions tab; `palette/`, `eden/`,
  `compositions/`, `markers/`, `triggers/`, `favourites/` and `recent/` the rest); the catalog
  trees and the search in
  [`apps/website/frontend/src/v2/apps/editor/arsenal/asset_catalog/`](/apps/website/frontend/src/v2/apps/editor/arsenal/asset_catalog/README.md);
  the Zones tab body in
  [`apps/website/frontend/src/v2/apps/editor/ui/inspector/zones_panel/`](/apps/website/frontend/src/v2/apps/editor/ui/inspector/zones_panel/README.md);
  the faction library dialog in `apps/website/frontend/src/v2/apps/editor/ui/modals/faction_manager.rs`
  ([dialogs README](/apps/website/frontend/src/v2/apps/editor/ui/modals/README.md)).
- Entry: `MissionEditorPage` mounts `DockRight` and loads the item
  [registry](/documentation_v2/glossary.md#registry) at boot
  (`apps/website/frontend/src/v2/apps/editor/mission_editor/registry_loading.rs`).
- Related features: [placement](/documentation_v2/website/frontend/apps/editor/feature_inventory/placement.md)
  (what a pick-up does on the map).
- Eden counterpart: [asset browser](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/interactions/asset_browser.md)
  and [compositions](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/interactions/compositions.md).

## Behaviour

| ID | Feature | Status |
|---|---|---|
| RIGHT-TABS-001 | Seven-tab strip and the collapsed stub | shipped |
| RIGHT-CAT-001 | Factions tab: the side's asset tree | shipped |
| RIGHT-CHIPS-001 | "BLUFOR", "OPFOR", "INDFOR" and "Objects" chips | shipped |
| RIGHT-SEARCH-001 | Search by name | shipped |
| RIGHT-SEARCH-002 | `class:`, `mod:`, wildcard and `/…/` search | shipped |
| RIGHT-FAIL-001 | Catalog failure view with "Retry" | shipped |
| RIGHT-STUB-001 | Vehicles tab | shipped |
| RIGHT-STUB-002 | Markers tab | shipped |
| RIGHT-STUB-003 | Objectives tab | not built |
| RIGHT-ZONES-001 | Zones tab | shipped |
| RIGHT-TRIG-001 | Triggers tab | shipped |
| RIGHT-COMP-001 | Compositions tab: save, list, arm, edit, delete | shipped |
| RIGHT-FAV-001 | Favourites and "Recently placed" | shipped |
| RIGHT-FACTIONS-001 | "Manage factions": the faction library | shipped |

The status legend is in the [inventory index](/documentation_v2/website/frontend/apps/editor/feature_inventory/README.md#how-it-works).
RIGHT-CHIPS-001, RIGHT-SEARCH-002, RIGHT-FAIL-001 and RIGHT-ZONES-001 to RIGHT-FACTIONS-001 are
rows added for shipped code; RIGHT-STUB-001 and RIGHT-STUB-002 keep their IDs, and the tabs they
name work.

### RIGHT-TABS-001 — Tab strip

1. The dock is 240 px wide. Its strip holds seven glyph tabs, each named by its title: "Factions",
   "Vehicles", "Zones", "Compositions", "Triggers", "Favourites" and "Markers", then the "Manage
   factions" button and the collapse chevron (`ui/docks/dock_right/shell/layout.rs:126-139`).
2. Collapsed (the chevron, or R), the dock is a 24 px stub with the chevron alone. No function
   key switches tabs.

### RIGHT-CAT-001 and RIGHT-CHIPS-001 — Factions tab

1. The tab reads "Drag a role onto the map to place its slot." over the chip row "BLUFOR",
   "OPFOR", "INDFOR" and "Objects"; a disabled "Custom" chip also shows while a side chip is
   selected (`shell/factions_panel.rs:43-101`).
2. A side chip sets the active side, the side the next placement joins, and shows that side's
   tree: its characters, the non-template vehicles and the registered objects filed under that
   side, in folders that follow each row's category (`build_faction_catalog_tree`,
   `arsenal/asset_catalog/faction_catalog_trees.rs:9-86`). Only the top-level folders start open.
3. The "Objects" chip switches to objects mode and shows the objects tree: world objects whose
   `prop:` or `comp:` alias the [mod](/documentation_v2/glossary.md#mod)'s spawn registry holds
   ("No placeable objects in the registry." when there are none). Choosing a side chip again
   restores the last side.
4. Pressing a leaf arms its placement (PLACE-DROP-001) and records it in "Recently placed"; a star
   on a leaf adds it to the favourites.

### RIGHT-SEARCH-001 and RIGHT-SEARCH-002 — Search

1. Each tree has a search box, "Search assets", "Search objects" or "Search vehicles", whose
   placeholder ends " — class: mod: * /re/"; a hint below it reads
   "class:Character_US · mod:ArmaReforger · *Rifleman · /^us (mg|ar)$/"
   (`ui/docks/dock_right/eden/mod.rs:146-150`).
2. A plain query is a case-insensitive substring of a label; `*` and `?` make it a whole-label
   wildcard; `/…/` a regular expression. `class:` matches a leaf's resource name or its class-name
   tail as a prefix; `mod:` matches the top-level folders. A folder whose label matches keeps its
   whole subtree, and a filtered tree is drawn fully open (`filter_catalog`).
3. An empty result names why, for example "Type a class name after class:" or "No objects
   match."; a regular expression is evaluated within fixed step, depth and length caps, so a
   pathological pattern stops matching rather than freezing the tab.

### RIGHT-FAIL-001 — Catalog failure

While the registry loads, the tree shows a loading state. If the fetch fails, the view reads
"Could not load the asset catalog. The request to the registry failed." or, when the registry
answers 404, "No modpack is configured, so the asset catalog is empty. Set a current modpack, then
retry.", with "Retry", which fetches the registry again (`catalog_failure_view` in
`ui/docks/dock_right/eden/mod.rs`).

### RIGHT-STUB-001 — Vehicles tab

The tab reads "Every placeable vehicle, across factions — a filtered view of the catalog. Drag one
onto the map to place it.", with a vehicle search and the "Place with crew" checkbox
(PLACE-CREW-001); abstract templates are left out ("No placeable vehicles." when none remain).
With the "Objects" chip selected it says "Objects place from the Factions tab while the Objects
chip is selected." instead (`shell/layout.rs:160-210`).

### RIGHT-STUB-002 — Markers tab

The tab says "Map markers for the active side's briefing. Pick an icon, then click the map to drop
it. Select a marker to caption it or nudge its position." It offers one row per marker glyph the
map draws, with "Search icons" over labels, slugs and aliases, and lists the placed markers;
selecting one opens "Marker <id> — <side>" with "Type", "Text", "Position (x, z metres)" and
"Delete marker" (`ui/docks/dock_right/markers/panel.rs`).

### RIGHT-STUB-003 — Objectives tab

Not built: the dock has no Objectives tab. Win conditions are set in the Mission Settings dialog
(TOP-SETTINGS-001), and trigger areas have their own tab (RIGHT-TRIG-001).

### RIGHT-ZONES-001 and RIGHT-TRIG-001 — Zones and Triggers tabs

1. Zones lists the "Authored zones" and draws new ones with "Circle" or "Polygon" (PLACE-ZONE-001);
   "Whole-terrain zone" adds a "Play Area" boundary over the whole terrain. Selecting a zone opens
   its attributes, rules and "Redraw circle" or "Redraw polygon". The same tab arms a tactical
   graphic draw.
2. Triggers picks an activation, "presence", "radio" or "timer", and draws the area with the same
   tool. Selecting a trigger opens "Attributes — <id>" with "Name", "Activation", "Owner", the
   redraw buttons, the rules and "Delete trigger"; a line on the map joins a selected trigger to
   its owner ([triggers README](/apps/website/frontend/src/v2/apps/editor/ui/docks/dock_right/triggers/README.md)).

### RIGHT-COMP-001 — Compositions tab

1. The tab reads "Reusable multi-entity stamps. Select entities and Save; click a saved row to
   arm, then click the map to place." With entities selected, "Save composition… (N selected)"
   asks for "Title" and "Category" and saves the whole selection into the mission, the signed-in
   user as its author (`ui/docks/dock_right/compositions/mod.rs`).
2. The saved rows sit under their categories as "<title>" with "by <author> · N item(s)"; a row
   arms its stamp (PLACE-COMP-001), and its hover actions edit the title, category and author or
   delete it. Every change is one undo step.

### RIGHT-FAV-001 — Favourites and recently placed

1. "Favourites" lists the starred assets, newest first, kept in this browser under
   `tbd-mc-editor-favourites` (at most 250). An asset the catalog does not offer stays in the
   list, crossed out and removable.
2. "Recently placed" lists this session's pick-ups and stamps, newest first; it starts empty on
   every mount. A press on either list arms the same placement as the palette leaf.

### RIGHT-FACTIONS-001 — Faction library

"Manage factions" opens the Faction Manager: faction templates, each with a side, named roles
(each role a character, with a loadout) and vehicles. "New faction", "Add role", "Add vehicle"
and "Save faction" edit the library, which the server keeps per mission maker; it refuses a
save without a faction name ("A faction name is required.") or with a role lacking a name or a
character ("Every role needs a name and a character.").

### Known discrepancies

- The disabled "Custom" chip's tooltip reads "Custom groups arrive in T-078"
  (`shell/factions_panel.rs:96`) — interface text names a ticket, and T-078 is cancelled.
- An object leaf in a side's tree says "Drag onto the map to place this object"
  (`ui/docks/dock_right/palette/mod.rs:38`) — the side mode refuses to arm objects
  (`placement_is_armable` in `apps/website/map-engine/src/data/store/operations/entity/arming.rs`),
  so the press places nothing (PLACE-DROP-002).

## Data

- `GET /api/v1/registry` (`list_registry` in
  `apps/website/api_v2/src/missions/handlers/registry_items.rs`): the item rows, fetched in pages
  of 500 at boot and read as `RegistryItem`; the palettes build every tree from them.
  `GET /api/v1/registry?limit=1&offset=0` probes a failure for the "No modpack" wording.
- `GET /api/v1/factions`, `POST /api/v1/factions`, `PUT /api/v1/factions/{id}` and
  `DELETE /api/v1/factions/{id}` (`apps/website/api_v2/src/missions/handlers/faction_library.rs`):
  the signed-in mission maker's own faction library; every call needs the mission-maker tier.
- Compositions, markers, zones and triggers live in the mission document; favourites in this
  browser's local storage.

## Design

- Design target: Eden's asset browser in the
  [Eden asset browser reference](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/interactions/asset_browser.md)
  and the right palette of the [UX specification](/documentation_v2/website/frontend/apps/editor/ux_spec.md).
  Differences: seven tabs instead of Eden's six F-key modes, with no key to switch them; units,
  vehicles and objects of a side share one Factions tree, beside a separate Vehicles tab; no
  waypoint or system modes; compositions are saved inside the mission, not published.

## Open work

- [T-146 — Asset Browser Data Wiring](/documentation_v2/tickets/specs/t146_asset_browser_data_wiring.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-146_plan.md)): the registry's vehicles and
  crates in the asset browser.
- [T-820 — Catalog failure generic cause; chips visible wrongly](/documentation_v2/tickets/specs/t820_catalog_failure_cause.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-820_plan.md)): the failure view names its
  cause, and the chips hide when there is no catalog.
- [T-838 — Map markers selectable; outliner lists; dblclick opens Attributes](/documentation_v2/tickets/specs/t838_marker_select_outliner.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-838_plan.md)) and
  [T-932 — Parked briefing markers survive server save/reload](/documentation_v2/tickets/specs/t932_parked_markers_persist.md)
  (queued, [plan](/documentation_v2/tickets/plans/t-932_plan.md)): markers.
- [T-212 — Typed per-side objectives with attributes](/documentation_v2/tickets/specs/t212_typed_objectives.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-212_plan.md)): objectives as their own
  entities.
- [T-939.7 — Vehicles panel virtualization, memoized outliner flatten](/documentation_v2/tickets/specs/t939_editor_usability.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-939_7_plan.md)): a long vehicle list stays
  fast.
- [T-728 — Save-composition affordance reads a channel selection never bumps](/.ai/tickets/T-728.toml)
  and [T-729 — Owner-line materialize per frame; zones mislabels triggers](/.ai/tickets/T-729.toml)
  (deferred, no plan).

## Decisions

- A leaf's kind comes from its registry row, not from the tab it sits in, so a vehicle leaf in a
  side's tree never places a slot.
- Favourites belong to the browser, compositions to the mission: a favourite follows the mission
  maker, a composition travels with the mission.
