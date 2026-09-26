# Right dock

The [Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator)'s right dock, its asset
browser: seven tabs from which an author places characters, vehicles, objects, compositions,
markers and trigger and zone areas into the [mission](/documentation_v2/glossary/g_to_m.md#mission), with
the side chips, favourites and recently placed assets beside them. The module root is
`apps/website/frontend/src/v2/apps/editor/ui/docks/dock_right.rs`: it declares these folders so
they share one scope, holds the vehicle tab's "Place with crew" checkbox, re-exports the items
below and mounts the dock's tests.

## Contents

```text
apps/website/frontend/src/v2/apps/editor/ui/docks/dock_right/
├── compositions/  the Compositions tab: save the selection as a stamp, list, arm, edit and delete
├── eden/          the side chips, the tab sub-modes, the search grammar hint and the failure view
├── favourites/    the starred assets in local storage, and the Favourites and Recently placed lists
├── markers/       the Markers tab: the schema's icon vocabulary, the picker and the marker form
├── palette/       the recursive tree rows whose leaves arm a placement when pressed
├── recent/        the session's recently placed list and the recorder for off-dock placements
├── shell/         `DockRight`: the tab strip, each tab's body, the Factions tab, the zone hook
└── triggers/      the Triggers tab: draw with the zone tool, attributes, rules, the owner line
```

## How it works

`DockRight` in `shell/` receives what the editor page loads and owns (the catalog states, the
[registry](/documentation_v2/glossary/n_to_z.md#registry) rows, `doc_tick`, `active_side`, `objects_mode`
and the collapse flag) and draws one tab body at a time:

| Tab | Drawn by |
|---|---|
| Factions | `shell/` with the chips of `eden/` and the merged faction tree of `palette/` |
| Vehicles | `shell/` with the tree rows of `palette/` |
| Zones | the inspector's zones panel |
| Compositions, Triggers, Markers | `compositions/`, `triggers/` and `markers/` |
| Favourites | `favourites/`, whose second list reads the session list of `recent/` |

A leaf press arms a placement through `bridge::host_state::armed_placement`, and the canvas
release commits it into the document. Every panel that reads the document asks the map engine's
`editing::hosted_commands` and reads again whenever `doc_tick` moves; every write goes through
the same module and bumps `doc_tick`. Those panels compile for the browser build only, each with a
native sibling that draws nothing, so the native tests compile the whole dock.

Two hooks let code outside the dock reach state that lives inside `DockRight`: the zone selection
hook in `shell/`, through which the editor's selection router selects a zone, and the recently
placed recorder in `recent/`, through which placements made elsewhere join the list. `DockRight`
registers both at mount, and each cleanup removes only its own registration.

## Public surface

- `DockRight`: the dock component, which
  `apps/website/frontend/src/v2/apps/editor/shell/eden_chrome.rs` re-exports and
  `apps/website/frontend/src/v2/apps/editor/mission_editor.rs` mounts.
- `route_select_zone`: selects a zone in the mounted dock; the selection router in
  `apps/website/frontend/src/v2/apps/editor/mission_editor/canvas_mount.rs` calls it for a zone.
- `record_placed`: adds an off-dock placement to the recently placed list; called by the canvas
  release in
  `apps/website/frontend/src/v2/apps/editor/bridge/host_state/armed_placement/map_release.rs` and
  the [ORBAT](/documentation_v2/glossary/n_to_z.md#orbat) manager's vehicle picker in
  `apps/website/frontend/src/v2/apps/editor/ui/modals/orbat_manager/tree_rows.rs`.
- `marker_icon_is_authorable`: the closed marker icon test `begin_place_marker` applies in
  `apps/website/frontend/src/v2/apps/editor/bridge/host_state/armed_placement/palette_arming.rs`.
- The module root's other re-exports serve only the dock's tests.

## Boundaries

- Depends on:
  - in the editor: `crate::v2::apps::editor::shell::layout` (`DOCK_R`, `STUB_PX`), the asset
    catalog in `arsenal::asset_catalog`, `bridge::host_state` (`armed_placement`,
    `editor_context`), the inspector's zones panel, the outliner's tree rows and styles, and the
    left dock's `collapse_chevron`;
  - `crate::v2::core`: the [API](/documentation_v2/glossary/a_to_f.md#api) client and `RegistryItem`, the
    auth store and `MaterialIcon`;
  - `website_map_engine`: `editing::hosted_commands`, `editing::host`, `editing::tools::selection`,
    `streaming::host` and `overlay::symbology::markers`;
  - `contracts_v2/definitions/mission.schema.json`, through the zones panel's embed, and the
    browser's local storage.
- Used by: the callers above; the tests in
  `apps/website/frontend/src/v2/apps/editor/ui/docks/tests/dock_right/`; the outliner smoke test
  in `tools_v2/developer-tools/src/browser_testing/editor_smoke_tests/outliner_palette.rs`, which
  drives the Factions tab and drags a leaf onto the map.
- Rules: the dock's source checks read the production files listed in
  `DOCK_RIGHT_PRODUCTION_SOURCE` in
  `apps/website/frontend/src/v2/apps/editor/ui/docks/tests/dock_right/mod.rs`, so a new production
  file here joins that list; a mount hook's cleanup never removes a newer registration
  (`an_older_owners_cleanup_does_not_clobber_a_newer_registration` in that folder's
  `zone_hook_lifecycle.rs`); the tab strip fits the 240 px dock (`the_tab_strip_fits_the_dock`).

## Related documentation

- [Mission Creator UX specification](/documentation_v2/website/frontend/apps/editor/ux_spec.md) —
  the right palette's place in the layout and its interactions.
- [Mission Creator feature inventory](/documentation_v2/website/frontend/apps/editor/feature_inventory/README.md)
  — the asset palette's features, one entry each.
- [Eden editor UI anatomy](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/ui_anatomy.md)
  — the Eden asset browser this dock follows.
- [Mission Creator feature inventory: placement](/documentation_v2/website/frontend/apps/editor/feature_inventory/placement.md) — what a palette pick-up does on the map.
