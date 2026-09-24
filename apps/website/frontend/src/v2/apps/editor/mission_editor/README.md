# Mission Creator page parts

The parts of the [Mission Creator](/documentation_v2/glossary.md#mission-creator)'s route component:
the canvas mount that installs the editor's handles and starts the boot of the
[mission](/documentation_v2/glossary.md#mission) document, the page's reactive effects, the loading
of the item [registry](/documentation_v2/glossary.md#registry), the transform and snap model, the
toolbar dispatch and the helpers the input layer picks and draws with. The route component
`MissionEditorPage` lives in the page module,
`apps/website/frontend/src/v2/apps/editor/mission_editor.rs`, which declares these files with
`#[path]` attributes and re-exports what the rest of the editor uses.

## Contents

```text
apps/website/frontend/src/v2/apps/editor/mission_editor/
├── armed_place.rs        the armed-place release decision and its pure transition model
├── canvas_mount/         the mount's parts: document setup, boot tasks, registry, input listeners
├── canvas_mount.rs       `install_canvas_mount`: installs handles, seams, effects; starts the boot
├── document_helpers.rs   connection segments, hover pick, map cursor, the Arrange chord gate
├── page_effects.rs       payload-size estimate, Arrange chords, widget tick, catalog rebuild
├── registry_loading.rs   the paged registry fetch and the compatibility and cargo-defaults fetch
├── toolbar_dispatch.rs   `EditorToolbarDispatch`: the toolbar's widget, snap and select-all calls
└── transform.rs          the snap ladders, `SnapState`, `WidgetVariant`, the rotation-ring hit test
```

## How it works

`MissionEditorPage` creates the page signals, installs the page effects and hands the signals to
`install_canvas_mount` as `PageMountSignals` (wasm only). Once the canvas loads, the mount:

```text
size the canvas ──> seed the document (canvas_mount/document_setup.rs)
──> selection set, ruler, line-of-sight and viewshed state ──> register the tool seams,
    the widget pivot, the toolbar dispatch and the validation panel's seams
──> website_map_engine::editing::host::install, history set_ctx, editor_context::install,
    the side signals, __editorHistory, the undo and redo keys, the unload guard
──> effects: the connection lane on each document tick; chrome and dock changes into
    shell::layout, an engine resize, and the pane centre held while a dock collapses
──> canvas_mount/ boot tasks and input listeners
```

`page_effects.rs` re-estimates the compiled payload size 500 ms after the OBJ count settles
(`shell::mission_size::estimate_compiled_bytes`), bumps the transform widget's tick when the
selection changes, rebuilds the active side's catalog when the registry or the side changes, and
installs the Arrange chords (Alt+L, Alt+R, Alt+T, Alt+B, Alt+H, Alt+V), which act only with at least
two entities selected (`ARRANGE_MIN_SELECTION`). The six key codes are written out in
`page_effects.rs` as well as in the top strip's shared `ARRANGE` list, whose `arrange_for_code`
lookup only the strip's tests call. `registry_loading.rs` fetches the item registry in pages of 500
until the reported total and the compatibility feed the
[arsenal](/documentation_v2/glossary.md#arsenal) reads. `transform.rs` holds the snap model:
translate rungs of 0, 1, 5 and 10 m, the map engine's rotate ladder, the G toggle and the `[` and
`]` steps, and the widget variants None, Translate and Rotate on the keys 1, 2 and 3.
`armed_place.rs` decides what a release does while a place is armed: a left release on the map
places, off the map keeps the arm, the middle button pans and the right button disarms.

## Public surface

Everything reaches the rest of the editor through the page module's declarations and re-exports:

- `canvas_mount::install_canvas_mount` and `PageMountSignals`: the page's mount call.
- `transform` (`SnapState`, `WidgetVariant`, `press_on_ring`, `WIDGET_RADIUS_PX`): the overlays in
  `apps/website/frontend/src/v2/apps/editor/bridge/` and the pointer gestures in
  `apps/website/frontend/src/v2/apps/editor/input/`.
- `armed_place::{decide_armed_pointerup, ArmedUp}`: the pointer-up handler.
- `toolbar_dispatch_generation` and `with_editor_toolbar_dispatch`: the top strip's tool row.
- `document_helpers` (`live_connection_segments`, `hover_hit`, `set_map_cursor`, `HoverPoints`,
  `arrange_chord`): the pointer gestures and the canvas mount.

## Boundaries

- Depends on: in `apps/website/frontend/src/v2/apps/editor/`, the whole `bridge/` seam, the input
  layer, the session's layout, draft writer, hydrate, review mode, mission size and document
  commands in `shell/`, the top strip's Arrange commands and the validation panel in `ui/`, and the
  asset catalog and rules of `arsenal/`; `crate::v2::core` (the
  [API](/documentation_v2/glossary.md#api) client, `AuthStore`, the `RegistryItem` DTOs);
  `website_map_engine` (`frame`, `streaming::host`, `editing`, `data::store`, `overlay::symbology`,
  `camera`); over HTTP, `GET /api/v1/registry` and `GET /api/v1/registry/compat`.
- Used by: the page module `apps/website/frontend/src/v2/apps/editor/mission_editor.rs`, and through
  its re-exports the bridge, the input layer and the top strip; the source pins in
  `apps/website/frontend/src/v2/apps/editor/tests/`, the keymap census in
  `apps/website/frontend/src/v2/apps/editor/ui/modals/tests/help_modal/keymap_census/`, and
  `mission_editor_move_commit_names_the_atomic_mix_api` in
  `apps/website/map-engine/src/data/store/rows/tests/cases_1.rs`, which reads `canvas_mount.rs`.
- Rules:
  - `canvas_mount.rs` sits at the 500-line ceiling that `cargo xtask verify file-length` holds, so a
    new install goes into a part under `canvas_mount/`;
  - across `canvas_mount.rs` and the pointer-up handler exactly one drag-move commit exists
    (`mission_editor_move_commit_names_the_atomic_mix_api`);
  - the Arrange chords claim no chord another keydown listener holds
    (`no_two_listeners_claim_the_same_chord` in
    `apps/website/frontend/src/v2/apps/editor/ui/modals/tests/help_modal/keymap_census/tests.rs`);
  - a changed Arrange chord changes both `page_effects.rs` and the `ARRANGE` list in
    `apps/website/frontend/src/v2/apps/editor/ui/docks/top_strip/arrange.rs`: the help rows and the
    six keydown arms in `page_effects.rs` are both checked against the list
    (`arrange_help_rows_match_the_shared_list`; `the_editor_keydown_binds_the_arrange_chords` and
    `the_bound_codes_are_exactly_the_shared_lists_chorded_rows` in
    `apps/website/frontend/src/v2/apps/editor/tests/mission_editor/arrange_chords.rs`).

## Related documentation

- [Mission Creator feature inventory](/documentation_v2/website/frontend/apps/editor/feature_inventory/README.md)
  — the route and layout, the load and the transform features.
- [Mission Creator UX specification](/documentation_v2/website/frontend/apps/editor/ux_spec.md) —
  the layout and the keyboard shortcuts.
