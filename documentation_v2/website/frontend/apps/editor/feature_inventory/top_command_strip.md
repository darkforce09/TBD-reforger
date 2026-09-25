**Status:** live

# Top command strip

The two rows across the top of the [Mission Creator](/documentation_v2/glossary.md#mission-creator):
the [mission](/documentation_v2/glossary.md#mission)'s title and menus, the
[ORBAT](/documentation_v2/glossary.md#orbat) Manager button, undo and redo, the time of day and
weather, the validation chip, "Save Version" and "Export", and the Mission Settings dialog the
strip opens.

## Where it lives

- Code: [`apps/website/frontend/src/v2/apps/editor/ui/docks/top_strip/`](/apps/website/frontend/src/v2/apps/editor/ui/docks/top_strip/README.md)
  (`view.rs` and the [view fragments](/apps/website/frontend/src/v2/apps/editor/ui/docks/top_strip/view/README.md)
  `menu_row.rs`, `tool_row.rs`, `overlays.rs`; `menu_catalog.rs`); the save and export transport
  in [`apps/website/frontend/src/v2/apps/editor/shell/document_commands/imp/`](/apps/website/frontend/src/v2/apps/editor/shell/document_commands/imp/README.md);
  the settings dialog in
  [`apps/website/frontend/src/v2/apps/editor/ui/modals/settings_modal/`](/apps/website/frontend/src/v2/apps/editor/ui/modals/settings_modal/README.md);
  the validation chip's findings in
  [`apps/website/frontend/src/v2/apps/editor/ui/inspector/validation_panel/`](/apps/website/frontend/src/v2/apps/editor/ui/inspector/validation_panel/README.md).
- Entry: `MissionEditorPage` mounts `TopCommandStrip` in the 48 px band above the map.
- Related features: [data persistence and compile](/documentation_v2/website/frontend/apps/editor/feature_inventory/data_persistence_and_compile.md)
  (the "Draft saved" chip, what Save Version sends, the row mirrors),
  [transform and delete](/documentation_v2/website/frontend/apps/editor/feature_inventory/transform_and_delete.md)
  (the widget, snap and Arrange controls), [keyboard shortcuts](/documentation_v2/website/frontend/apps/editor/feature_inventory/keyboard_shortcuts.md)
  (the Controls Hint).
- Eden counterpart: [toolbar, keys, actions and status bar](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/interactions/toolbar_keys_actions_and_status_bar.md).

## Behaviour

| ID | Feature | Status |
|---|---|---|
| TOP-MENU-001 | Menu bar: six menus | shipped |
| TOP-TITLE-001 | Mission title edited in place | shipped |
| TOP-DIRTY-001 | Unsaved-changes dot | shipped |
| TOP-ORBAT-001 | "ORBAT Manager" button | shipped |
| TOP-HIST-001 | Version history button | not built |
| TOP-UNDO-001 | Undo | shipped |
| TOP-REDO-001 | Redo | shipped |
| TOP-ENV-001 | "Time of day" slider | shipped |
| TOP-ENV-002 | "Weather" select | shipped |
| TOP-VALID-001 | Validation chip and findings | shipped |
| TOP-SAVE-001 | "Save Version" dialog | partial |
| TBD-SAVE-001 | Immutable SemVer versions (not in Eden) | shipped |
| TOP-EXPORT-001 | "Export JSON" | shipped |
| TBD-EXPORT-001 | The exported file reloads losslessly (not in Eden) | shipped |
| TOP-EXPORT-002 | "Export Compiled" | shipped |
| TOP-SETTINGS-001 | Mission Settings dialog | shipped |
| ENV-SETTINGS-002 | View distance and thermals | not built |
| TOP-CENSUS-001 | Per-side slot census and summary line | not built |
| TOP-MERGE-001 | Merge another mission into this one | not built |

The status legend is in the [inventory index](/documentation_v2/website/frontend/apps/editor/feature_inventory/README.md#how-it-works).
TOP-ORBAT-001, TOP-VALID-001, TOP-EXPORT-002, TOP-CENSUS-001 and TOP-MERGE-001 are rows added
for the code; the two `TBD-` rows are features Eden has no counterpart for.

### TOP-MENU-001 — Menus

"File" ("Save Version…", "Export JSON", "Export Compiled Mission"), "Edit" (undo, redo, "Select
All on Screen (Ctrl+A)", the three widget rows, "Toggle Snap Grid (G)" and the two snap steps),
"Arrange" (the nineteen Arrange commands, XFORM-ALIGN-001), "Mission" ("Mission Settings…",
"Briefing & Thumbnail…"), "Environment" ("Time & Weather…") and "Help" ("Keyboard Shortcuts
(Controls Hint)") (`ui/docks/top_strip/menu_catalog.rs:6-93`). Every row runs an action; the three
Mission and Environment rows all open the Mission Settings dialog. A click outside or Escape
closes an open menu.

### TOP-TITLE-001 — Title

1. The first field shows the document's title, or the route's mission id while the document has
   none (`ui/docks/top_strip/view/menu_row.rs:10-40`; `mission_editor.rs:267`).
2. A changed, non-blank value is written to the document when the field loses focus or Enter is
   pressed; a blank value is ignored.
3. The title reaches the mission library only with the next Save Version (DATA-HYD-TITLE-001);
   the row mirror never sends it.

### TOP-DIRTY-001 — Unsaved changes

A "•" titled "Unsaved changes" follows the title while the document holds edits no saved version
has (`menu_row.rs:41-52`); a successful save clears it. The "Draft saved …" chip at the row's
right edge is the local draft's (DATA-IDB-001).

### TOP-ORBAT-001 — ORBAT Manager

"ORBAT Manager" ("Open the ORBAT Manager") closes any open menu and opens the ORBAT Manager
dialog, the only home of the ORBAT tree (`menu_row.rs:193-213`; LEFT-ORBAT-001).

### TOP-HIST-001 — Version history

Not built: the history button is always disabled, titled "Version history (soon)"
(`ui/docks/top_strip/view/tool_row.rs:9-17`). A mission's versions are listed in the mission
library.

### TOP-UNDO-001 and TOP-REDO-001 — Undo and redo

The undo ("Undo (Ctrl+Z)") and redo ("Redo (Ctrl+Shift+Z)") buttons and the Edit menu rows step
the document history and disable themselves when there is nothing to step
(`tool_row.rs:18-47`); the keys are KEY-UNDO-001 and KEY-REDO-001.

### TOP-ENV-001 and TOP-ENV-002 — Time and weather

1. "Time of day" is a slider over the day's minutes with the time beside it as HH:MM; "Weather"
   offers "Clear", "Overcast", "Heavy Rain" and "Dense Fog" (`tool_row.rs:129-165`).
2. Each change is one undo step in the document and is mirrored onto the mission row after
   400 ms of quiet (`RowMirror`); the Mission Settings dialog edits the same two values.

### TOP-VALID-001 — Validation chip

The chip reads "No issues" or counts such as "2 errors · 1 warning", coloured by the worst
severity; clicking it opens the findings grouped by rule, and a finding whose subject can be
selected selects it (`tool_row.rs:183-242`). The findings are re-evaluated 250 ms after the last
document change, and Export Compiled adds its compile findings.

### TOP-SAVE-001 and TBD-SAVE-001 — Save Version

1. "Save Version" ("Save an immutable version of this mission") opens "Save Version" —
   "Versions are immutable — pick a new semver." — with the "Version" field focused and selected,
   "Notes", the size line "~<size> · N objects" (yellow over 200 MB) and "Save"; Tab stays inside
   the dialog and Escape closes it (`ui/docks/top_strip/view/overlays.rs:24-168`).
2. "Save" refuses a document with duplicate slot ids and names them, else compiles the payload
   and posts it; the status reads "Saving v…", then "Saved v…", "Version … already exists",
   "Payload too large", "Sign in to save" or "Save rejected (…): …" with one line per problem
   (`save_now`, `shell/document_commands/imp/mission_saving.rs:12-92`). The dialog stays open.
3. Each saved version is immutable and numbered by the mission maker; a taken number is refused,
   never overwritten (TBD-SAVE-001; what is sent is DATA-COMP-001).
4. Partial: the "Version" field starts at "0.1.0" (`mission_editor.rs:120`), which every mission
   already has, and never advances after a save, so the proposed number is refused until the
   mission maker types a new one.

### TOP-EXPORT-001, TBD-EXPORT-001 and TOP-EXPORT-002 — Exports

1. "Export" opens two rows (`tool_row.rs:257-317`); a second activation carrying the same click
   is dropped (`begin_export_gesture`, `shell/document_commands/imp/exports.rs:17-24`).
2. "Export JSON" ("The editor superset envelope — re-imports here; the mod cannot load it")
   downloads `mission-<id>.json`: the payload with its ORBAT in the export envelope, the game mode
   and player count taken from the mission row (`export_now`, `exports.rs:93-118`).
3. TBD-EXPORT-001: the file carries the whole `editor` block, so uploading it as a version from
   the mission library and opening that version restores the document.
4. "Export Compiled" ("The compiled mission document the game server receives") downloads
   `mission-<id>.compiled.json`, the document the mod loads, and posts its compile findings to the
   validation chip. It refuses while the mission row is missing and says in the toast when unsaved
   edits make it differ from every saved version (`export_compiled_now`, `exports.rs:40-84`).

### TOP-SETTINGS-001 and ENV-SETTINGS-002 — Mission Settings

1. The gear ("Mission Settings — the rest of the environment") and the Mission and Environment
   menu rows open "Mission Settings" — "Environment and flow for this mission."
   (`ui/modals/settings_modal/mission_dialog.rs:7-148`).
2. It shows the terrain read-only, "Time" and "Weather", "All settings in this mission" (a
   read-only list against the schema's defaults), "Presentation" ("Briefing", "Thumbnail link"),
   "Mission shape" ("Game mode", "Players" beside the declared maximum), "Mission flow" (the
   durations and "Join in progress"), the win conditions, the spawn modules, and the map display
   ("Show hillshade", its strength, "Grid"), with a pointer to the per-browser Editor Preferences.
3. ENV-SETTINGS-002 is not built: the dialog has no view distance or thermals control and says
   "View distance and thermals are not part of a compiled mission — it carries time and weather
   only." (`ui/inspector/env.rs:94-95`); `author_env` refuses any environment key no reader takes.

### TOP-CENSUS-001 — Census and summary

Not built as a visible feature: the menu row computes a per-side count ("WEST", "EAST", "IND",
"UNA", "TOTAL") and a summary line such as "<mode> <total> on <Terrain> — WEST n v EAST n", but
renders both inside a hidden element (`menu_row.rs:242-289`).

### TOP-MERGE-001 — Merge

Not built as a surface: `merge_mission_now` merges another mission's current version into the
open document as one undo step, but no button or menu row calls it; only the test harness does
(`shell/document_commands/imp/mission_merge.rs:49`).

### Known discrepancies

- "Export JSON" writes the Save Version field's value into the envelope's `version`
  (`run_action`, `ui/docks/top_strip/view.rs:191-196`) — the export's own comment says it is the
  current semver (`exports.rs:86-92`), and the page keeps the last saved one separately
  (`current_semver`, `mission_editor.rs:191`).
- The Arrange rows enable with one entity selected; the Alt chords need two
  (see [transform and delete](/documentation_v2/website/frontend/apps/editor/feature_inventory/transform_and_delete.md#known-discrepancies)).

## Data

- `POST /api/v1/missions/{id}/versions` (`create_version` in
  `apps/website/api_v2/src/missions/handlers/mission_versions.rs`): Save Version, described in
  [data persistence and compile](/documentation_v2/website/frontend/apps/editor/feature_inventory/data_persistence_and_compile.md#data).
- `PATCH /api/v1/missions/{id}` (`update_mission` in
  `apps/website/api_v2/src/missions/handlers/mission_lifecycle.rs`): the time, weather and
  Mission Settings row fields; `GET /api/v1/missions/{id}` for the row the dialog shows.
- The exports make no call: they compile the local document and download it.

## Design

- The menu row holds the identity, the menus and the ORBAT Manager; the tool row the commands,
  with "Save Version" the one primary action and both exports behind one "Export" button.
- Design target: Eden's menu bar and toolbar in the
  [Eden toolbar reference](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/interactions/toolbar_keys_actions_and_status_bar.md)
  and the [UX specification](/documentation_v2/website/frontend/apps/editor/ux_spec.md).
  Differences: saving is a numbered, immutable server version, not a file; there is no "Play"
  or preview; the history button waits on version history.

## Open work

- [T-821 — Save version prefill static; second save 409s](/documentation_v2/tickets/specs/t821_save_version_prefill.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-821_plan.md)): the dialog proposes a free
  version number.
- [T-827 — Validation chip red under 4.5:1 live-effective](/documentation_v2/tickets/specs/t827_validation_chip_contrast.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-827_plan.md)): the chip's error colour meets
  contrast.
- [T-158 — Editor shell UX consolidation](/documentation_v2/tickets/specs/t158_editor_shell.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-158_plan.md)): one settings entry point and
  the inert buttons wired.
- [T-704 — Command palette over every editor command](/documentation_v2/tickets/specs/t704_command_palette.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-704_plan.md)): a searchable palette of the
  commands.
- [T-1034 — Fix top strip slot census and summary line never showing](/.ai/tickets/T-1034.toml)
  (idea, no plan): the census shows or goes.
- [T-1053 — Decide whether mission upload and save share one duplicate-slot check](/.ai/tickets/T-1053.toml)
  (idea, no plan): one duplicate-id check for Save Version and the library upload.
- [T-083 — Top menu bar](/.ai/tickets/T-083.toml) (deferred, no plan): Eden-style menus.

No open ticket covers version history in the editor, the merge surface or the export envelope's
version field.

## Decisions

- Versions are numbered by the mission maker and immutable: a taken number is refused.
- Two exports answer two questions: "Export JSON" re-imports into the editor, "Export Compiled"
  is what the game server loads; both take the live title.
- A setting with no reader gets no control: the dialog offers no view distance or thermals,
  because no compiled mission and no mod code reads either value.
