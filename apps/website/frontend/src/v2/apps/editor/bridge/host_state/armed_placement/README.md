# Armed placement

The in-flight placement of the [Mission Creator](/documentation_v2/glossary.md#mission-creator):
what the operator picked up from a palette and has not yet dropped on the map, the zone and trigger
draw that rides the same arm, and the map release that commits it to the
[mission](/documentation_v2/glossary.md#mission).

## Contents

```text
apps/website/frontend/src/v2/apps/editor/bridge/host_state/armed_placement/
├── map_release.rs     the canvas release that commits the arm, selects it, runs the post-edit tail
├── mod.rs             the module tree and the arm itself: `arm`, `has_pending`, `cancel_pending`
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
   ├── left button ──> place_at_alt, or place_at_keep with Ctrl or Cmd (re-arms after a place)
   │      take the arm ──> commit_armed_placement ──> select it ──> after_local_edit
   └── right button, release over chrome, pointercancel ──> cancel_pending
```

The release takes the armed value before it opens the document, so a release commits at most once,
and hands it with the active side, the crew toggle and the Alt flag (a vehicle without its crew) to
the map engine's `commit_armed_placement`, which files the new entity under the active folder in the
same undo step. A placed vehicle rebinds the vehicle lane in the same frame, and a stamped
composition joins the right dock's recently placed list. A zone draw keeps its draft on the same
armed value, so "is a draw in flight" has one source: `cancel_pending` leaves it armed,
`close_zone_polygon` refuses a ring under three vertices, and only a closed shape writes, as one row
through `website_map_engine::editing::hosted_commands`; `cancel_zone_draw` abandons it with no
write. An arm itself is never document state and never an undo step.

## Boundaries

- Depends on: the editor context in
  `apps/website/frontend/src/v2/apps/editor/bridge/host_state/editor_context/` (`EDITOR_CONTEXT`,
  `Pending`, `bump_doc_tick`, `place_with_crew`); the undo driver in
  `apps/website/frontend/src/v2/apps/editor/bridge/document_host/`; `website_map_engine`
  (`data::store::operations::entity` for the arm gate, the placement commit and the zone draft,
  `editing::hosted_commands` for the zone and trigger rows); the asset catalog's `PlacePayload`; the
  right dock's `marker_icon_is_authorable` and `record_placed`; the zone predicates re-exported by
  `apps/website/frontend/src/v2/apps/editor/shell/eden_chrome.rs`; the outliner's
  `ensure_active_layer`.
- Used by:
  - the palette, favourites, compositions, markers and triggers panels of the right dock in
    `apps/website/frontend/src/v2/apps/editor/ui/docks/dock_right/`, the zones panel and the audio
    emitters in `apps/website/frontend/src/v2/apps/editor/ui/inspector/`, and the asset picker in
    `apps/website/frontend/src/v2/apps/editor/bridge/overlays/`, which arm it;
  - the pointer gestures and the window keydown in `apps/website/frontend/src/v2/apps/editor/input/`
    and the input listeners of
    `apps/website/frontend/src/v2/apps/editor/mission_editor/canvas_mount/`, which release or cancel
    it;
  - the source pins that read these files through `ENTITY` in
    `apps/website/frontend/src/v2/core/test_support/editor_operations.rs`, and
    `apps/website/frontend/src/v2/apps/editor/tests/t647_placement_interactions.rs`.
- Rules: the discriminant lives on the armed value, never on a reading of which palette tab is open,
  since the tab can change between the pick-up and the commit; every file here is on the place path
  that `cargo xtask verify editor-orbat-coherency` scans, which bans `ensure_default_squad` and
  fails when a listed file is missing, so a renamed file updates the gate's list in
  `tools_v2/xtask/src/verifications/architecture/editor_orbat_coherency.rs`.

## Related documentation

- [Mission Creator feature inventory](/documentation_v2/website/frontend/apps/editor/feature_inventory/README.md)
  — palette placement and click-to-place.
