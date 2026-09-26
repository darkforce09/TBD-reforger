# Top command strip view fragments

The three view fragments `TopCommandStrip` expands in place: the menu row, the tool row and the
overlays the strip raises. Each file holds one `macro_rules!` macro that receives the component's
signals and closures by name, so the fragment runs in the component's own reactive scope.

## Contents

```text
apps/website/frontend/src/v2/apps/editor/ui/docks/top_strip/view/
├── menu_row.rs  `menu_row!`: the title field, the six menus, "ORBAT Manager", the draft chip
├── overlays.rs  `overlays!`: the Controls Hint, the click-away backdrop and the Save Version dialog
└── tool_row.rs  `tool_row!`: undo, redo, widgets, snap, time, weather, validation, save and export
```

## How it works

The menu row edits the [mission](/documentation_v2/glossary/g_to_m.md#mission) title in place, shows a
dot for unsaved changes, opens the "File", "Edit", "Arrange", "Mission", "Environment" and "Help"
menus, opens the [ORBAT](/documentation_v2/glossary/n_to_z.md#orbat) manager, and shows "Draft saved …"
after the local draft is written. A per-side [slot](/documentation_v2/glossary/n_to_z.md#slot) census and
the mission summary line also render in this row, inside a `hidden` element. The tool row holds
"History" (always disabled), undo and redo, the three transform widget buttons, the snap toggle and
its step buttons, the "Time of day" slider and "Weather" select with the settings gear, the
validation chip and its findings dropdown, the save status, "Save Version", and "Export" with
"Export JSON" and "Export Compiled". The overlays add the Controls Hint, a backdrop that closes an
open menu, export list or findings dropdown on a click, and the "Save Version" dialog: version and
notes fields, the size estimate ("~<size> · N objects"), progress while saving, the save findings
and "Save", which calls the shell's `save_now`.

## Boundaries

- Depends on: the `top_strip` scope through `super::*` (the strip classes, `MENUS`, `MenuAction`,
  the clock and draft helpers, `WEATHER_OPTIONS`, `trap_tab_in_dialog`, `selection_count`);
  `Select`, `Slider`, `MaterialIcon` and `cn` from `crate::v2::core::ui`;
  `bridge::host_state::editor_context` (title and slots) and `bridge::document_host::history`
  (undo, redo); the inspector's `author_env` and validation panel (`Rollup`, `findings_dropdown`);
  `ControlsHint` from `ui::modals::help_modal`; `shell::mission_size` and
  `shell::document_commands::save_now`.
- Used by: `TopCommandStrip` in
  `apps/website/frontend/src/v2/apps/editor/ui/docks/top_strip/view.rs`, the only place that
  expands the three macros.
- Rules: the three macros take the same 34 named arguments, and
  `apps/website/frontend/src/v2/apps/editor/ui/docks/tests/top_strip/test_source.rs` expands them
  by substituting those names, so a renamed argument changes there too; "Save Version" is the one
  primary action and both exports sit behind the one "Export" trigger
  (`exactly_one_action_is_primary` and `the_exports_live_behind_one_secondary_trigger` in that
  folder's `menu_row_layout.rs`); the dialog focuses its version field and keeps Tab inside it
  (`version_input_takes_focus_on_open` and `traps_tab_within_the_dialog_subtree` in
  `save_version_dialog.rs`).

## Related documentation

- [Mission Creator feature inventory: top command strip](/documentation_v2/website/frontend/apps/editor/feature_inventory/top_command_strip.md) — the menus, the tool row and the Save Version dialog.
