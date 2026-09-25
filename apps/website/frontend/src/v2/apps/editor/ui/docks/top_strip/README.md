# Top command strip

The two-row strip across the top of the
[Mission Creator](/documentation_v2/glossary.md#mission-creator): the
[mission](/documentation_v2/glossary.md#mission) title, the menu bar, the
[ORBAT](/documentation_v2/glossary.md#orbat) manager button, undo and redo, the transform widget
and snap controls, the time of day and weather, the validation chip, "Save Version" and "Export".
The module root, `apps/website/frontend/src/v2/apps/editor/ui/docks/top_strip.rs`, declares these
files, holds the strip's button and menu classes, re-exports the items below and mounts the
strip's tests.

## Contents

```text
apps/website/frontend/src/v2/apps/editor/ui/docks/top_strip/
├── arrange.rs          `ArrangeKind` and `ARRANGE`: the nineteen Arrange commands and their invoker
├── clock_and_draft.rs  clock parsing and formatting, the draft chip, the mission row id check
├── dialog_focus.rs     the Tab trap of the Save Version dialog
├── menu_catalog.rs     `MENUS`: the six menus and their rows, and the live selection count
├── mission_summary.rs  `SlotCensus` and `summary_line`: the per-side slot census, the summary line
├── row_mirror.rs       `RowMirror`: debounced, single-flight writes of time and weather to the row
├── view/               the `menu_row!`, `tool_row!` and `overlays!` view fragments
└── view.rs             `TopCommandStrip`: its signals and memos, the Escape order and `run_action`
```

## How it works

The editor page mounts `TopCommandStrip` in the 48 px band above the map. Every menu row and
button ends in `run_action`: "Save Version" opens its dialog, whose "Save" calls the shell's
`save_now`; the two exports call the shell's `export_now` and `export_compiled_now` once
`begin_export_gesture` accepts the click; undo and redo call the document history; the settings
rows open the Mission Settings dialog; "Select All on Screen", the widget digits and the snap
controls go through the editor page's toolbar dispatch; the Arrange rows run `run_arrange_action`.

`ARRANGE` is the one list of the nineteen Arrange commands (patterns, align, space, orient) that
the "Arrange" menu, the context menu's Arrange submenu and the six Alt chords (Alt + L, R, T, B, H
and V) share, and `run_arrange` is their one invoker: patterns, spacing and orientation through
the map engine's `selection_transform`, alignment through the editor's grouped undo gestures. The
menu rows stay disabled, titled "Select entities first", while nothing is selected; the chords and
the context submenu need two or more entities (`ARRANGE_MIN_SELECTION`).

The "Time of day" slider and the "Weather" select write the document through the inspector's
`author_env`, and `RowMirror` copies the same value onto the mission row with
`PATCH /api/v1/missions/{id}`, sending `time_of_day` as HH:MM or `weather` as one of `clear`,
`overcast`, `heavy_rain` and `dense_fog`. The mirror waits for 400 ms of quiet
(`MIRROR_DEBOUNCE_MS`), keeps at most one request per column in flight, lets only the newest
value report a failure, and does nothing for a route id that is not a UUID or in review mode. A
failure raises a toast that names the setting and says it reverts on reload. The Mission Settings
dialog builds its own `RowMirror`; both share one module-level state per column.

Escape closes, in order, an open menu, the export list, the findings dropdown, the Save Version
dialog and the Controls Hint, unless the modal stack has consumed the key, and the strip registers
its closer with the modal stack so that a dialog opening elsewhere closes them. The draft chip
reads the shell's last local-draft write and refreshes every second ("Draft saved just now" under
5 s). `census_from_rows` counts [slots](/documentation_v2/glossary.md#slot) per side from the ORBAT
rows the map engine returns, and `summary_line` prints
"<mode> <total> on <Terrain> — WEST n v EAST n", adding "(+n IND)" and "(n unassigned)" when
nonzero; both render in a hidden element of the menu row.

## Public surface

- `TopCommandStrip`: the strip, which
  `apps/website/frontend/src/v2/apps/editor/shell/eden_chrome.rs` re-exports and
  `apps/website/frontend/src/v2/apps/editor/mission_editor.rs` mounts.
- `ArrangeKind`, `ARRANGE`, `ARRANGE_MIN_SELECTION` and `run_arrange`: the Arrange list and its
  invoker, for the context menu in `apps/website/frontend/src/v2/apps/editor/ui/docks/context_menu/`
  and the chords in `apps/website/frontend/src/v2/apps/editor/mission_editor/page_effects.rs` and
  `apps/website/frontend/src/v2/apps/editor/mission_editor/document_helpers.rs`.
- `RowMirror` and `is_mission_row_id`: the row mirror and its route check, for the Mission Settings
  dialog in `apps/website/frontend/src/v2/apps/editor/ui/modals/settings_modal/`.
- The module root's other re-exports (the clock helpers, the census, the summary line,
  `arrange_for_code`) serve only the strip's tests.

## Boundaries

- Depends on:
  - in the editor: `crate::v2::apps::editor::shell` (`layout` classes, `document_commands`,
    `persist`, `mission_size`, `review_mode`), `bridge::host_state` (`editor_context`,
    `undo_grouped_gestures`), `bridge::document_host::history`, the page's toolbar dispatch in
    `mission_editor`, the inspector's `author_env` and validation panel, the outliner's
    `FactionRow` and `SquadRow`, and `ui::modals::help_modal`;
  - `crate::v2::core`: `api_patch` and `MissionEnv` from `api`, `AuthStore`, and `Select`,
    `Slider`, `MaterialIcon`, the toasts and the modal stack from `ui`;
  - `website_map_engine`: `editing::hosted_commands` (`census_input`, `selection_transform`),
    `editing::host::selection_len` and the placement kinds of `editing::tools::placement`;
  - over HTTP, `PATCH /api/v1/missions/{id}` of the
    [missions](/documentation_v2/glossary.md#missions) domain.
- Used by: the callers above; the tests in
  `apps/website/frontend/src/v2/apps/editor/ui/docks/tests/top_strip/`; the outliner smoke test in
  `tools_v2/developer-tools/src/browser_testing/editor_smoke_tests/outliner_palette.rs`, which
  opens the ORBAT manager from its button.
- Rules: that tests folder holds these: the menu bar renders the shared Arrange list in order,
  and a menu click and a chord share one invoker
  (`the_menu_bar_renders_the_shared_list_in_order` and
  `the_menu_click_and_the_chord_share_one_invoker` in `arrange_actions.rs`); a burst of values
  becomes one request, and a second cannot start while one is in flight
  (`a_burst_collapses_to_the_settled_value` and
  `a_second_patch_cannot_start_while_one_is_in_flight` in `mission_summary_and_mirror.rs`);
  the summary line only ever appends its suffixes, in order
  (`summary_suffixes_are_append_only_and_ordered`); Escape yields to the modal stack
  (`top_command_strip_escape_yields_when_modal_stack_consumed_escape` in `escape_modal_stack.rs`);
  the weather options are the wire enum's values, in order
  (`the_weather_options_are_the_wire_enum` in `form_controls.rs`); `test_source.rs` there
  reassembles the strip's production files, fragments expanded, for the source checks, so a new
  file here joins its list.

## Related documentation

- [Mission Creator UX specification](/documentation_v2/website/frontend/apps/editor/ux_spec.md) —
  the command strip's place in the layout and the shipped shortcuts.
- [Mission Creator feature inventory](/documentation_v2/website/frontend/apps/editor/feature_inventory/README.md)
  — the command strip's features, one entry each.
- [Mission Creator feature inventory: top command strip](/documentation_v2/website/frontend/apps/editor/feature_inventory/top_command_strip.md) — every control of the strip, one entry each.
