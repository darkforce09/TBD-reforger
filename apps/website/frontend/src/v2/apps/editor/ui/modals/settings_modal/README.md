# Mission Settings dialog parts

The [Mission Creator](/documentation_v2/glossary.md#mission-creator)'s Mission Settings dialog for
the open [mission](/documentation_v2/glossary.md#mission), and the two dialogs it raises: the
read-only All Settings list and the per-browser Editor Preferences. The parent module,
`apps/website/frontend/src/v2/apps/editor/ui/modals/settings_modal.rs`, declares these modules and
re-exports `MissionSettingsDialog` and the settings catalog.

## Contents

```text
apps/website/frontend/src/v2/apps/editor/ui/modals/settings_modal/
├── all_settings_dialog.rs    `AllSettingsDialog`: every authored setting against its schema default
├── environment_sections.rs   the "Mission flow" section and the map display section
├── mission_dialog.rs         `MissionSettingsDialog`: terrain, time, weather and every section
├── mission_row_mirror.rs     `ShapeMirror`: loads and patches the mission's row; `ShapeSeq` ordering
├── mission_row_model.rs      `RowShape`, the game modes, the player count and their notes
├── mission_row_sections.rs   the "Presentation" and "Mission shape" sections
├── preferences_dialog.rs     `EditorPreferencesDialog`: the basemap and world layers of this browser
├── presentation_model.rs     the briefing and thumbnail columns, the link check, the briefing mirror
├── settings_catalog.rs       `aggregate_settings`: settings, owners and defaults from the schema
└── settings_navigation.rs    whether a setting's owner can be selected; the list's notes
```

## How it works

The editor page mounts `MissionSettingsDialog` through `shell::eden_chrome`; the top strip's
"Mission settings" button and its Mission and Environment menu entries open it. The dialog re-reads
the document on every `doc_tick`, so an undo refreshes it, and it holds three kinds of setting:

- **Document settings.** "Time" and "Weather" (also mirrored to the mission's row through the top
  strip's `RowMirror`), the "Mission flow" durations in seconds and "Join in progress", and the map
  display settings (hillshade, its strength, the grid) are written with `author_env`, one key and
  one undo step each. The win conditions card and the spawn modules section of the sibling inspector
  folder write their own blocks.
- **Row settings.** "Presentation" (briefing and thumbnail link) and "Mission shape" (game mode)
  live on the mission's row: `ShapeMirror` reads `GET /api/v1/missions/{id}` and writes each change
  with `PATCH /api/v1/missions/{id}`, optimistically, putting the stored value back and raising a
  toast when the [API](/documentation_v2/glossary.md#api) refuses. `ShapeSeq` drops a load that
  would overwrite a patch in flight. A saved briefing is also mirrored into the document's meta. The
  players figure is the count of [slots](/documentation_v2/glossary.md#slot) placed, beside the
  maximum declared at creation. In a review workspace the dialog shows the row values the
  [artifact](/documentation_v2/glossary.md#artifact) compiled from.
- **Browser settings.** "Editor Preferences" (basemap and world layers) are saved to this browser
  through `shell::world_layer_prefs`, never to the mission.

"All settings in this mission" opens `AllSettingsDialog`: `aggregate_settings` walks the document
for every authored setting, whichever entity owns it, and compares each with the default that
`contracts_v2/definitions/mission.schema.json` declares; "Changed from default" hides the provably
unchanged rows, and a row whose owner can be selected selects it through the validation router.

## Boundaries

- Depends on: `env`, `win_conditions_card`, `spawn_modules`, `zones_panel` and `validation_panel`
  of `apps/website/frontend/src/v2/apps/editor/ui/inspector/`; the top strip's `RowMirror` and
  `is_mission_row_id`; the editor's `shell/` (`document_commands`, `review_mode`,
  `world_layer_prefs`) and bridge (`editor_context::read_env`, the document handle);
  `crate::v2::core` (the API client, `AuthStore`, toasts, the URL guard, `modal_stack`,
  `MaterialIcon`, the `MissionEnv` DTO); over HTTP, `/api/v1/missions/{id}`.
- Used by: the parent module, whose `MissionSettingsDialog` `shell::eden_chrome` re-exports for
  `apps/website/frontend/src/v2/apps/editor/mission_editor.rs`; the smoke test
  `tools_v2/developer-tools/src/browser_testing/editor_smoke_tests/cur.rs`, which finds the dialog
  by its "Mission Settings" heading.
- Rules: all three dialogs answer Escape only while topmost
  (`settings_dialogs_gate_escape_on_modal_stack`), a row load that races a patch never applies
  (`a_get_that_races_a_patch_cannot_apply`), and the All Settings list edits nothing
  (`the_aggregated_view_is_not_a_second_editing_surface`), all in
  `apps/website/frontend/src/v2/apps/editor/ui/modals/tests/`.

## Related documentation

- [Missions domain](/apps/website/api_v2/src/missions/README.md) — the mission row routes the
  dialog reads and patches.
