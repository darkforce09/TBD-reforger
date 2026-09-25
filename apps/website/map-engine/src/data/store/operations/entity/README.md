# Entity authoring operations

The headless operations the [Mission Creator](/documentation_v2/glossary.md#mission-creator) runs
on each kind of placed thing in the [mission](/documentation_v2/glossary.md#mission) document:
[slots](/documentation_v2/glossary.md#slot) and the [ORBAT](/documentation_v2/glossary.md#orbat)
manager, vehicles, world objects, markers, comments, connections, zones and triggers, layer
folders, the clipboard, and what a map release commits for an armed placement.

## Contents

```text
apps/website/map-engine/src/data/store/operations/entity/
├── armed_placement.rs  `commit_armed_placement`: what a map release writes for an armed pick-up
├── arming.rs           which kinds may be armed in objects or side mode, and the debug slot seed
├── clipboard.rs        delete, copy and paste of the selection, and the saved-composition stamp
├── comments.rs         map comment rows and details, comment ids, place and duplicate
├── connections.rs      connection list and finding rows, connection ids, connect and delete
├── factions.rs         a side's roster as a faction template, and the ORBAT manager's apply
├── identity.rs         slot and layer id minting against the live document, new layer names
├── layer_drag.rs       the armed layer-tree drag: a folder reparent, a slot or comment refile
├── layers.rs           layer folders: create, rename, delete, reparent, refile, hide, lock, resolve
├── markers.rs          briefing marker rows and marker ids
├── mod.rs              the module tree; re-exports every operation and row type
├── placement.rs        world-object placement, and the ORBAT manager's add slot and add vehicle
├── refile.rs           the armed squad-row drag that moves a slot into another squad
├── roster.rs           the ORBAT manager snapshot, slot details, side factions, anchors, slot edits
├── selection.rs        the selection's slots, hidden state and centroid; the terrain key and bounds
├── selection_index.rs  the selected entities, read off the document index
├── tests/              unit tests for arming, release commits, the drags, layers, triggers, draws
├── triggers.rs         trigger edits, the owner link and the line it draws on the map
├── vehicles.rs         vehicle rows, cargo, heading, placement with crew, the owner options
├── zone_draw.rs        the in-flight zone or trigger draw and the geometry it closes into
└── zones.rs            zone and trigger rows, rule edits, row ids and the row write
```

## How it works

Every operation takes the document as `&MissionDocCore` plus plain values, and the host supplies
what only it knows as arguments: the `next_id` counter behind `n<k>` ids, and callbacks for the
folder a placement files into (`ensure_layer`), the cargo seed, and the zone rules a draw must meet.
A placement moves from a palette pick-up to one map release:

```text
palette pick-up ──▶ placement_is_armable(kind, objects mode)          arming.rs
     │  the host keeps the ArmedPlacement until the release
     ▼
map release ──▶ commit_armed_placement(core, request, next_id, ensure_layer)
     ├── Character    place_character_under_side, cargo seeded from the asset; selects the slot
     ├── Vehicle      place_vehicle_in_core, with or without crew; flags the vehicle lane rebind
     ├── Object       place_object_in_core under the side's faction key
     ├── Marker       a briefing marker on the side's faction
     ├── Composition  place_saved_composition; selects its slots, reports the stamp
     └── ZoneDraw     nothing: the release goes to the draw machine in zone_draw.rs
```

The `PlacementCommit` a release returns says what the host still owes (the new selection, a
vehicle lane rebind, the recently placed list); selection, rendering and the post-edit refresh stay
with the caller. Three pieces of session state live here in thread-locals: the armed layer-tree
drag, the armed squad refile, and the layer-id counter with the folder whose inline rename opens
next. Each drag is consumed by exactly one drop, and a release off target clears it untouched.

A new id is checked against the live document first, because undo frees ids and a restore can bring
back used ones: `mint_id` against the slots, `mint_ids` against slots, vehicles, objects and
comments, and the `cmt-<n>`, `conn-<n>`, `mk-<n>`, `z<n>`, `t<n>` and `layer-<n>` minters against
their own collections. A value outside a closed set is refused, never coerced: a side other than
`BLUFOR`, `OPFOR` or `INDFOR`, an activation outside `TRIGGER_ACTIVATIONS`, a blank folder name, a
ring below three vertices.

## Boundaries

- Depends on: `crate::data::store` (`MissionDocCore`, `NONE_IDX`, `place_character_under_side`,
  `apply_faction_library` and the apply anchors); the sibling operation modules for the place
  payload, attributes, compositions, the document index, faction templates, row projections, cargo
  and draw targets; `crate::data::scenario::compile::terrain_bounds`; `serde_json`.
- Used by:
  - `crate::editing::hosted_commands` (the document search, editor layers, clipboard,
    connections, comments, markers, triggers, ORBAT roster, placed vehicles, slot loadouts and zone
    authoring commands), and the sibling modules
    `apps/website/map-engine/src/data/store/operations/attrs.rs`, `cargo.rs`, `document_index.rs`
    and `transform.rs`;
  - the Mission Creator in `apps/website/frontend/src/v2/apps/editor/`: the armed placement and
    zone draw in `apps/website/frontend/src/v2/apps/editor/bridge/host_state/armed_placement/`, the
    editor context and selection in `apps/website/frontend/src/v2/apps/editor/bridge/host_state/`,
    the debug seed in `apps/website/frontend/src/v2/apps/editor/bridge/document_host/doc_host.rs`,
    the pointer release in `apps/website/frontend/src/v2/apps/editor/input/pointer_gestures/`, the
    zones panel in `apps/website/frontend/src/v2/apps/editor/ui/inspector/zones_panel/` and the
    outliner in `apps/website/frontend/src/v2/apps/editor/ui/outliner/`;
  - `apps/website/map-engine/tests/operation_boundaries.rs` and the re-export pins in
    `apps/website/map-engine/src/data/store/tests/reexports.rs`.
- Rules:
  - a release commits at most one placement, and a refused one writes nothing
    (`a_character_the_document_refuses_places_nothing`,
    `a_vehicle_the_document_refuses_places_nothing` and
    `a_draw_in_flight_commits_nothing_on_a_release` in `tests/armed_placement.rs`);
  - objects mode arms world objects only, a side mode characters and vehicles, and a zone is never
    armed (`tests/arming.rs`); a cancelled drag or refile leaves the document alone
    (`a_cancelled_drag_leaves_the_document_alone`, `a_cancelled_refile_is_not_a_drop`);
  - the document always keeps one folder
    (`the_last_folder_is_never_deleted_so_the_document_keeps_a_layer` in `tests/layers.rs`), and an
    out-of-set activation never reaches it
    (`an_activation_outside_the_closed_set_never_reaches_the_document` in `tests/triggers.rs`);
  - `cargo xtask verify editor-orbat-coherency` scans the files it lists from this folder for
    `ensure_default_squad` and fails when one of them is missing.

## Related documentation

- [Mission Creator feature inventory: placement](/documentation_v2/website/frontend/apps/editor/feature_inventory/placement.md) — what a map release commits, per kind.
- [Mission Creator feature inventory: transform and delete](/documentation_v2/website/frontend/apps/editor/feature_inventory/transform_and_delete.md) — what Delete removes.
