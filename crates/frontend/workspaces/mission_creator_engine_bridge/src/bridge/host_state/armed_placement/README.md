# Armed placement

The in-flight placement of the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator):
what the operator picked up from a palette and has not yet dropped on the map, the zone and trigger
draw that rides the same arm, and the map release that commits it to the
[mission](/documentation/glossary/g_to_m.md#mission).

## Contents

```text
crates/frontend/workspaces/mission_creator_engine_bridge/src/bridge/host_state/armed_placement/
├── map_release.rs     the canvas release that commits the arm, selects it, runs the post-edit tail
├── mod.rs             the module tree; re-exports; the arm: `arm`, `has_pending`, `cancel_pending`
├── palette_arming.rs  the palette arms: character, vehicle, object, saved composition, marker
└── zone_draw.rs       the multi-click zone and trigger draw: a circle or a ring, then one row
```

## How it works

The armed value is a `Pending` on the installed editor context (`Character`, `Vehicle`, `Object`,
`Composition`, `Marker` or `Zone`). A palette leaf's `pointerdown` arms it through `begin_place`,
`begin_place_vehicle`, `begin_place_object`, `begin_place_composition` or `begin_place_marker`;
`arm` asks the map engine's `placement_is_armable` whether the Objects mode admits that kind and
clears the arm when it does not, and a marker icon outside the schema's closed enum is never armed.
Every arm change bumps the document tick, so the docks re-read their "click the map" hints.

```text
palette pointerdown ──> begin_place_* ──> arm: Pending on the editor context
canvas pointerup ──> has_pending?
   ├── zone draw armed ──> advance_zone_draw: take a vertex, or close the circle
   ├── left button over the map ──> place_at_alt, or place_at_keep with Ctrl or Cmd (re-arms)
   │      take the arm ──> commit_armed_placement ──> select it ──> after_local_edit
   ├── left button over the chrome ──> nothing: the arm stays for the next map release
   └── right button, pointercancel ──> cancel_pending
```

The release takes the armed value before it opens the document, so a release commits at most once,
and hands it with the active side, the crew toggle and the Alt flag (a vehicle without its crew) to
the map engine's `commit_armed_placement`, which files the new entity under the active folder in the
same undo step. A placed vehicle rebinds the vehicle lane in the same frame, and a stamped
composition joins the right dock's recently placed list. A zone draw keeps its draft on the same
armed value, so "is a draw in flight" has one source: `cancel_pending` leaves it armed,
`close_zone_polygon` refuses a ring under three vertices, and only a closed shape writes, as one row
through `mission_editing_commands::hosted_commands`; `cancel_zone_draw` abandons it with no
write. An arm itself is never document state and never an undo step.

## Boundaries

- Depends on: the editor context in
  `crates/frontend/workspaces/mission_creator_engine_bridge/src/bridge/host_state/editor_context/` (`EDITOR_CONTEXT`,
  `Pending`, `bump_doc_tick`, `place_with_crew`); the undo driver in
  `crates/frontend/workspaces/mission_creator_engine_bridge/src/bridge/document_host/`; `mission_operations::entity` for the
  arm gate, the placement commit and the zone draft; `mission_editing_commands::hosted_commands`
  for the zone and trigger rows; from the editor's state layer
  (`crates/frontend/workspaces/mission_creator_state/src/`), the asset catalog's `PlacePayload`,
  `marker_icons::marker_icon_is_authorable`, `recent_placements::record_placed` and the zone
  predicates of `zones`; the active folder's `ensure_active_layer`.
- Used by:
  - the palette, favourites, compositions, markers and triggers panels of the right dock in
    `crates/frontend/workspaces/mission_creator_workspace/src/ui/docks/dock_right/`, the zones panel and the audio
    emitters in `crates/frontend/workspaces/mission_creator_workspace/src/ui/inspector/`, and the asset picker in
    `crates/frontend/workspaces/mission_creator_engine_bridge/src/bridge/overlays/`, which arm it;
  - the pointer gestures and the window keydown in `crates/frontend/workspaces/mission_creator_engine_bridge/src/input/`
    and the input listeners of
    `crates/frontend/workspaces/mission_creator_workspace/src/mission_editor/canvas_mount/`, which release or cancel
    it.
- Rules: the discriminant lives on the armed value, never on a reading of which palette tab is open,
  since the tab can change between the pick-up and the commit; no file here calls
  `ensure_default_squad`.

## Related documentation

- [Mission Creator feature inventory](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/README.md)
  — palette placement and click-to-place.
- [Mission Creator feature inventory: placement](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/placement.md) — the pick-up, release, repeat and cancel rules, one entry each.
