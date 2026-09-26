**Status:** live

# Placement

How a mission maker puts things into a [mission](/documentation_v2/glossary/g_to_m.md#mission) in the
[Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator): a palette pick-up that the next
map release commits, the multi-place and cancel rules, where a placed character lands in the
[ORBAT](/documentation_v2/glossary/n_to_z.md#orbat), and the other ways in: the empty-ground picker,
compositions, briefing markers, zone and trigger areas and map comments.

## Where it lives

- Code: the arm and the map release in
  [`apps/website/frontend/src/v2/apps/editor/bridge/host_state/armed_placement/`](/apps/website/frontend/src/v2/apps/editor/bridge/host_state/armed_placement/README.md);
  the release decision in `apps/website/frontend/src/v2/apps/editor/mission_editor/armed_place.rs`;
  the release branch in `apps/website/frontend/src/v2/apps/editor/input/pointer_gestures/pointer_up.rs`;
  the commit in `apps/website/map-engine/src/data/store/operations/entity/armed_placement.rs` and
  the arm gate in `entity/arming.rs`
  ([entity operations README](/apps/website/map-engine/src/data/store/operations/entity/README.md));
  the squad rule in
  [`apps/website/map-engine/src/data/store/operations/place_orbat/`](/apps/website/map-engine/src/data/store/operations/place_orbat/README.md).
- Entry: a palette leaf's press in the [right dock](/apps/website/frontend/src/v2/apps/editor/ui/docks/dock_right/README.md),
  then a release on the map.
- Related features: [right asset palette](/documentation_v2/website/frontend/apps/editor/feature_inventory/right_asset_palette.md)
  (what can be picked up), [left sidebar](/documentation_v2/website/frontend/apps/editor/feature_inventory/left_sidebar.md)
  (the active folder, LEFT-LAYER-002).
- Eden counterpart: [entity placement](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/interactions/entity_placement.md)
  and [vehicle crew](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/interactions/vehicle_crew.md).

## Behaviour

| ID | Feature | Status |
|---|---|---|
| PLACE-DROP-001 | Press a palette leaf, release on the map | shipped |
| PLACE-CLICK-001 | Click a leaf, then click the map | shipped |
| PLACE-DROP-002 | Vehicles and objects place as their own kinds | partial |
| PLACE-MULTI-001 | Ctrl/Cmd keeps the pick-up for another place | shipped |
| PLACE-CANCEL-001 | Right button, Escape or a pointer cancel drops the pick-up | shipped |
| PLACE-PREVIEW-001 | Preview under the cursor while armed | shipped |
| PLACE-SQUAD-001 | A placed character joins its side's open squad | shipped |
| PLACE-CREW-001 | A vehicle with or without its crew | shipped |
| PLACE-PICKER-001 | Double-click empty ground: "Place asset…" | partial |
| PLACE-COMP-001 | Stamp a saved composition | shipped |
| PLACE-MARKER-001 | Drop a briefing marker | shipped |
| PLACE-ZONE-001 | Draw a zone or trigger area | shipped |
| PLACE-COMMENT-001 | Place a map comment | shipped |

The status legend is in the [inventory index](/documentation_v2/website/frontend/apps/editor/feature_inventory/README.md#how-it-works).
PLACE-MULTI-001 to PLACE-COMMENT-001 are rows added for shipped code.

### PLACE-DROP-001 and PLACE-CLICK-001 — Pick up and place

1. A press on a palette leaf arms a placement: the kind (character, vehicle, object,
   composition or marker) and the asset ride the armed value, so switching tabs before the
   release changes nothing (`begin_place*`, `armed_placement/palette_arming.rs`). The press is
   not an HTML drag: the page's release handler commits it.
2. While armed, a left press on the map opens no selection gesture, and a left release on the
   map, clear of the docks, the top strip and the toolbelt band, places at the cursor
   (`pointer_up.rs:62-124`, `decide_armed_pointerup` in `mission_editor/armed_place.rs:13-21`).
3. A left release over the chrome keeps the arm, so dragging a leaf onto the map and clicking a
   leaf then clicking the map both place.
4. The release takes the arm before it writes, so one release places at most once; the new
   entity is one undo step, and a placed character or composition becomes the selection
   (`place_at_impl`, `armed_placement/map_release.rs:88-136`).
5. Characters and vehicles land at height 0 m and rotation 0; characters, compositions and
   comments are filed in the active folder, and a new character starts with the cargo its asset
   carries (`commit_armed_placement`, `apps/website/map-engine/src/data/store/operations/entity/armed_placement.rs`).

### PLACE-DROP-002 — Kinds

1. A leaf's kind comes from its [registry](/documentation_v2/glossary/n_to_z.md#registry) row, so a
   vehicle leaf places a vehicle and an object leaf a world object, never a
   [slot](/documentation_v2/glossary/n_to_z.md#slot) (`placeable_palette`,
   `apps/website/frontend/src/v2/apps/editor/arsenal/asset_catalog.rs:195-209`).
2. The arm gate lets a side chip arm characters and vehicles and the "Objects" chip arm objects
   only; compositions and markers arm in either (`placement_is_armable`,
   `apps/website/map-engine/src/data/store/operations/entity/arming.rs`).
3. Partial: the Factions tree of a side also files that side's registered objects
   (`build_faction_catalog_tree`, `arsenal/asset_catalog/faction_catalog_trees.rs:77-81`), and
   pressing one arms nothing because the side mode refuses objects. The leaf still says "Drag
   onto the map to place this object".

### PLACE-MULTI-001 and PLACE-CANCEL-001 — Repeat and cancel

1. Holding Ctrl or Cmd on the release places and re-arms the same pick-up, so the next click
   places another (`place_at_keep`, `map_release.rs:37-54`).
2. A right-button release, Escape with no dialog open, or a pointer cancel drops the arm and
   clears the preview (`pointer_up.rs:66-72`; `input/window_keydown.rs:85-94`;
   `mission_editor/canvas_mount/input_listeners.rs:126-135`). A middle-button drag pans and
   keeps it.

### PLACE-PREVIEW-001 — Preview

While a pick-up is armed, moving the pointer over the map draws the placement preview at the
cursor (`set_place_preview`, `pointer_gestures/pointer_move.rs:116-122`), and the hover cursor
stays quiet.

### PLACE-SQUAD-001 — ORBAT filing

A placed character goes under the active side's faction, `faction-{SIDE}`, created on first use.
It joins the side's last squad while that squad is still open (an unnamed or "Squad N" squad with
no callsign and no vehicles), otherwise a new "Squad N"; the first slot of a squad becomes its
leader (`place_character_under_side`,
`apps/website/map-engine/src/data/store/operations/place_orbat/placement.rs:28-31`).

### PLACE-CREW-001 — Crew

The Vehicles tab's "Place with crew" checkbox sets whether a placed vehicle brings its crew;
holding Alt on the release places this one empty whatever the checkbox says
(`vehicle_places_its_crew`, `entity/armed_placement.rs:73-76`). Alt can only remove the crew.

### PLACE-PICKER-001 — Empty-ground picker

1. A double-click on empty map opens "Place asset…" at the pointer: a search over the active
   side's character leaves (`bridge/overlays/asset_picker.rs:48-127`).
2. Picking a row arms that character and closes the picker; the next click on the map places it.
3. Partial: the picker records the double-clicked world point (`AssetPickerState`,
   `asset_picker.rs:8-13`) and never uses it, and it lists characters only.

### PLACE-COMP-001 — Compositions

A row of the Compositions tab arms its stamp ("Click the map to stamp it at the cursor. Esc or
right-click to cancel."); the release places every entity of the composition around the cursor,
files them in the active folder, selects the stamped slots and adds the composition to "Recently
placed" (RIGHT-COMP-001).

### PLACE-MARKER-001 — Briefing markers

An icon row of the Markers tab arms a marker ("Click the map to drop it on the active side's
briefing."); the release adds a briefing marker with that icon to the active side's faction.
An icon outside the schema's closed `$defs/marker.icon` list is never armed
(`begin_place_marker`, `armed_placement/palette_arming.rs:49-54`).

### PLACE-ZONE-001 — Zone and trigger areas

The Zones tab's "Circle" takes a centre click and a rim click; "Polygon" takes a click per vertex
and "Close ring", which refuses fewer than three. The Triggers tab draws its areas with the same
tool. Only a closed shape writes, as one row; Escape abandons the draw with no write
([zones panel README](/apps/website/frontend/src/v2/apps/editor/ui/inspector/zones_panel/README.md),
`armed_placement/zone_draw.rs`).

### PLACE-COMMENT-001 — Comments

The map context menu's "Place Comment", on empty ground, places a comment at the right-clicked
point in the active folder (`ui/docks/context_menu/menu_dispatch.rs:88-92`). Comments are
editor notes: they are saved with the mission and never compiled for the game.

### Known discrepancies

- The Factions tab reads "Drag a role onto the map to place its slot." and the leaves "Drag onto
  the map to place" (`ui/docks/dock_right/shell/factions_panel.rs:43`,
  `ui/docks/dock_right/palette/mod.rs:34-42`) — a click on the leaf followed by a click on the
  map places too.
- An object leaf in a side's Factions tree is recorded in "Recently placed" on press
  (`palette/mod.rs:272-290`) although the arm is refused and nothing is placed.

## Data

- No API call. A placement is a local document edit; the item registry the palettes read is
  fetched once at boot (`GET /api/v1/registry`, see the
  [right asset palette](/documentation_v2/website/frontend/apps/editor/feature_inventory/right_asset_palette.md)).

## Design

- Design target: Eden's placement in the
  [Eden entity placement reference](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/interactions/entity_placement.md)
  and the [UX specification](/documentation_v2/website/frontend/apps/editor/ux_spec.md).
  Differences: Alt forces an empty vehicle rather than inverting the crew setting; the
  empty-ground picker arms rather than places at the double-clicked point; areas are drawn by
  clicks, not a held drag.

## Open work

- [T-816 — Armed composition hint open; one Esc clears both layers wrongly](/documentation_v2/tickets/specs/t816_esc_hint_layer.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-816_plan.md)): one Escape cancels one layer.
- [T-090.4 — Z placement audit (buried / floating objects)](/documentation_v2/tickets/specs/t090_4_z_placement_audit.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-090_4_plan.md)) and
  [T-143 — Water mask placement guard and exact hydrology](/documentation_v2/tickets/specs/t090_091_map_terrain_program.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-143_plan.md)): heights and water checks at the
  drop.
- [T-939.5 — Faction catalog: squad templates from compositions and defaults](/documentation_v2/tickets/specs/t939_editor_usability.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-939_5_plan.md)): whole squads from the
  palette.
- [T-824 — Placed zones must render visibly at rest on map](/documentation_v2/tickets/specs/t824_zone_render_at_rest.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-824_plan.md)) and
  [T-831 — Per-side marker authoring audit then explicit UI](/documentation_v2/tickets/specs/t831_per_side_markers_audit.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-831_plan.md)): zones and markers after the
  drop.
- [T-718 — Slot removal crew residue; picker flags; undo Alt lies](/.ai/tickets/T-718.toml)
  (deferred, no plan): the crew seats and the Alt wording.
- [T-1051 — Check whether minted vehicle and object ids can collide](/.ai/tickets/T-1051.toml)
  and [T-1052 — Decide whether placement scatter must stay stable across Rust releases](/.ai/tickets/T-1052.toml)
  (idea, no plan).

No open ticket covers the unarmable object leaves or the picker's unused drop point.

## Decisions

- The armed value carries its own kind, so a tab switch between the pick-up and the release
  never changes what is placed.
- The arm gate is read when the pick-up is armed: a pick-up left over from a mode the mission
  maker has since left is dropped, never committed under the new mode.
- A placed character never takes the lead from a squad that already has one.
