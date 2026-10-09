# Mission Creator page parts

The parts of the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s route component:
the canvas mount that installs the editor's handles and starts the boot of the
[mission](/documentation/glossary/g_to_m.md#mission) document, the page's reactive effects, the loading
of the item [registry](/documentation/glossary/n_to_z.md#registry), the toolbar dispatch and the
page's document helpers. The route component `MissionEditorPage` lives in the page module,
`crates/frontend/workspaces/mission_creator_workspace/src/mission_editor.rs`, which declares these files with
`#[path]` attributes and imports what the page itself uses.

## Contents

```text
crates/frontend/workspaces/mission_creator_workspace/src/mission_editor/
├── canvas_mount/         the mount's parts: document setup, boot tasks, registry, input listeners
├── canvas_mount.rs       `install_canvas_mount`: installs handles, seams, effects; starts the boot
├── document_helpers.rs   the subject resolver type and the Arrange chord gate
├── page_effects.rs       payload-size estimate, Arrange chords, widget tick, catalog rebuild
├── registry_loading.rs   the paged registry fetch and the compatibility and cargo-defaults fetch
└── toolbar_dispatch.rs   `EditorToolbarDispatch`: the toolbar's widget, snap and select-all calls
```

## How it works

`MissionEditorPage` first registers the lower layers' hooks (the session's draft writer into the
undo driver's draft-persist hook, the context menu as the right-click opener, the validation panel
as the compile-findings publisher), then creates the page signals, installs the page effects and
hands the signals to `install_canvas_mount` as `PageMountSignals` (wasm only). Once the canvas loads, the mount:

```text
size the canvas ──> seed the document (canvas_mount/document_setup.rs)
──> selection set, ruler, line-of-sight and viewshed state ──> register the tool seams,
    the widget pivot, the toolbar dispatch and the validation panel's seams
──> mission_editing_session::host::install, history set_ctx, editor_context::install,
    the side signals, __editorHistory, the undo and redo keys, the unload guard
──> effects: the connection lane on each document tick; chrome and dock changes into
    state::layout, an engine resize, and the pane centre held while a dock collapses
──> canvas_mount/ boot tasks and input listeners
```

`page_effects.rs` re-estimates the compiled payload size 500 ms after the OBJ count settles
(`session::mission_size::estimate_compiled_bytes`), bumps the transform widget's tick when the
selection changes, rebuilds the active side's catalog when the registry or the side changes, and
installs the Arrange chords (Alt+L, Alt+R, Alt+T, Alt+B, Alt+H, Alt+V), which act only with at least
two entities selected (`ARRANGE_MIN_SELECTION`). The six key codes are written out in
`page_effects.rs` as well as in the top strip's shared `ARRANGE` list, whose `arrange_for_code`
lookup only the strip's tests call. `registry_loading.rs` fetches the item registry in pages of 500
until the reported total and the compatibility feed the
[arsenal](/documentation/glossary/a_to_f.md#arsenal) reads. The snap model (translate rungs of 0,
1, 5 and 10 m, the map engine's rotate ladder, the widget variants None, Translate and Rotate on the
keys 1, 2 and 3) and the armed-place release decision are the state layer's `transform` and
`armed_place` in `crates/frontend/workspaces/mission_creator_state/src/`; the hover hit test is the bridge's
`hover_hit_testing`.

## Public surface

- `canvas_mount::install_canvas_mount` and `PageMountSignals`: the page's mount call.
- `toolbar_dispatch::{toolbar_dispatch_generation, with_editor_toolbar_dispatch}`: the top strip's
  tool row.
- `document_helpers` (`SubjectResolver`, `arrange_chord`): the canvas mount and the page effects.

## Boundaries

- Depends on: in `crates/frontend/workspaces/mission_creator_workspace/src/`, the whole `bridge/` seam, the input
  layer, the session's draft writer, hydrate, conflict dialog, mission size and document commands
  in `session/`, the top strip's Arrange commands and the validation panel in `ui/`, and the state
  layer's layout, review mode, asset catalog, rules, transform and scale math in `state/`; the foundation crates (the
  [API](/documentation/glossary/a_to_f.md#api) client, `AuthStore`, the `RegistryItem` DTOs);
  `map_renderer`, `map_streaming_host` and `map_streaming_model`; `mission_editing_session` (`host`);
  `mission_document`, `mission_crdt`, `mission_payload` and `mission_operations`;
  `unit_symbology` (`classification`, `markers`) and `terrain_elevation::grid`; over HTTP, `GET /api/v1/registry` and `GET /api/v1/registry/compat`.
- Used by: the page module `crates/frontend/workspaces/mission_creator_workspace/src/mission_editor.rs`, and the top
  strip through `toolbar_dispatch`; the source pins in
  `crates/frontend/workspaces/mission_creator_workspace/src/tests/`, the keymap census in
  `crates/frontend/workspaces/mission_creator_workspace/src/ui/modals/tests/help_modal/keymap_census/`.
- Rules:
  - `canvas_mount.rs` sits at the 500-line ceiling that `cargo xtask verify file-length` holds, so a
    new install goes into a part under `canvas_mount/`;
  - across `canvas_mount.rs` and the pointer-up handler exactly one drag-move commit exists
    (`only_one_move_arm_commits_the_atomic_mix` in
    `crates/frontend/workspaces/mission_creator_workspace/src/tests/t648_transform.rs`);
  - the Arrange chords claim no chord another keydown listener holds
    (`no_two_listeners_claim_the_same_chord` in
    `crates/frontend/workspaces/mission_creator_workspace/src/ui/modals/tests/help_modal/keymap_census/tests.rs`);
  - a changed Arrange chord changes both `page_effects.rs` and the `ARRANGE` list in
    `crates/frontend/workspaces/mission_creator_workspace/src/ui/docks/top_strip/arrange.rs`: the help rows and the
    six keydown arms in `page_effects.rs` are both checked against the list
    (`arrange_help_rows_match_the_shared_list`; `the_editor_keydown_binds_the_arrange_chords` and
    `the_bound_codes_are_exactly_the_shared_lists_chorded_rows` in
    `crates/frontend/workspaces/mission_creator_workspace/src/tests/mission_editor/arrange_chords.rs`).

## Related documentation

- [Mission Creator feature inventory](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/README.md)
  — the route and layout, the load and the transform features.
- [Mission Creator UX specification](/documentation/crates/frontend/workspaces/mission_creator_workspace/ux_spec.md) —
  the layout and the keyboard shortcuts.
- [Mission Creator feature inventory: shell route and layout](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/shell_route_and_layout.md) — the chrome the page raises.
- [Mission Creator feature inventory: transform and delete](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/transform_and_delete.md) — the snap model and the transform widget.
