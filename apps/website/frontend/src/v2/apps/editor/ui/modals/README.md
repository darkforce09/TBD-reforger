# Mission Creator dialogs

The dialogs the [Mission Creator](/documentation_v2/glossary.md#mission-creator) raises over the
whole workspace instead of framing the map: the Mission Settings dialog for the open
[mission](/documentation_v2/glossary.md#mission), the [ORBAT](/documentation_v2/glossary.md#orbat)
Manager, the Faction Manager for the faction library, and the floating controls hint with its
shortcut catalog.

## Contents

```text
apps/website/frontend/src/v2/apps/editor/ui/modals/
├── faction_manager.rs  `FactionManagerDialog`: create, edit and delete the faction library's templates
├── help_modal/         the controls hint card and the shortcut catalog
├── help_modal.rs       the help module tree; re-exports the hint and the catalog; mounts the census
├── mod.rs              the module tree
├── orbat_manager/      the ORBAT Manager: side tree, slot inspector, faction templates
├── orbat_manager.rs    the ORBAT Manager module tree; re-exports the dialog and the template helpers
├── settings_modal/     the Mission Settings dialog, the All Settings list, the Editor Preferences
├── settings_modal.rs   the settings module tree; re-exports the dialog and catalog; inner openers
└── tests/              unit tests for the four dialogs and the keymap census
```

## How it works

| Dialog | Module | Opened from | Writes |
|---|---|---|---|
| Mission Settings | `settings_modal` | the top strip's "Mission settings" button and menus | the document's environment, the mission's row |
| ORBAT Manager | `orbat_manager` | the top strip's "ORBAT Manager" button | the document's ORBAT, the faction library |
| Faction Manager | `faction_manager` | the right dock's "Manage factions" button | the faction library |
| Controls hint | `help_modal` | the top strip's Help menu | nothing |

The editor page, `apps/website/frontend/src/v2/apps/editor/mission_editor.rs`, mounts the first
three, the Mission Settings and ORBAT Manager dialogs through `shell::eden_chrome`; the top strip's
overlays mount the controls hint. None of them is routed. Each takes an `open` signal from its
mount site and owns only the draft of what it edits; document changes go through the map engine's
hosted commands.

The Faction Manager edits the signed-in mission maker's own faction library: each faction's side,
name, role templates (role, tag and a character from the item
[registry](/documentation_v2/glossary.md#registry)) and vehicles (a vehicle and an optional label);
a stored role's loadout is kept as it is. It loads
the library with `GET /api/v1/factions`, saves a new faction with `POST /api/v1/factions` and an
existing one with `PUT /api/v1/factions/{id}`, and deletes with `DELETE /api/v1/factions/{id}`
after the "Delete this faction?" confirmation.

The Mission Settings dialog with its two inner dialogs, the ORBAT Manager and the Faction Manager
register with `core::ui::modal_stack` and answer Escape only while topmost, so stacked dialogs
close in order; the ORBAT Manager also takes its overlay z-index from the stack. A dialog that
binds a key listens on the window, and the keymap census under `tests/help_modal/keymap_census/`
checks every such editor binding against the others and against the shortcut catalog.

## Public surface

- `settings_modal::MissionSettingsDialog` and `orbat_manager::OrbatManagerDialog`: re-exported by
  `shell::eden_chrome` for the editor page.
- `faction_manager::FactionManagerDialog`: mounted by the editor page.
- `help_modal::{ControlsHint, hint_shown, set_hint_shown}`: mounted and toggled by the top strip.

## Boundaries

- Depends on:
  - the sibling folders of `apps/website/frontend/src/v2/apps/editor/ui/`: the inspectors (the
    environment gate, the win conditions and spawn modules sections, the zone schema, the
    validation router), the outliner (the ORBAT node model, the drag latch) and the top strip's
    row mirror;
  - `website_map_engine::editing::hosted_commands` for every document write;
  - the editor's `shell/` (`layout` classes, `document_commands`, `review_mode`,
    `world_layer_prefs`) and bridge (`editor_context`, `entity_selection`, the document handle);
  - `crate::v2::core`: the [API](/documentation_v2/glossary.md#api) client and DTOs, `AuthStore`,
    `modal_stack`, `Dialog`, toasts, `MaterialIcon`; over HTTP, `/api/v1/missions/{id}` and
    `/api/v1/factions` of the API's [missions](/documentation_v2/glossary.md#missions) domain.
- Used by:
  - `apps/website/frontend/src/v2/apps/editor/mission_editor.rs` and
    `apps/website/frontend/src/v2/apps/editor/shell/eden_chrome.rs`;
  - the top strip in `apps/website/frontend/src/v2/apps/editor/ui/docks/top_strip/`, for the
    controls hint;
  - the headless editor smoke tests in `tools_v2/developer-tools/src/browser_testing/editor_smoke_tests/`,
    which find the Mission Settings and ORBAT Manager dialogs by their headings.
- Rules: a dialog stacked over another answers Escape only while topmost
  (`settings_dialogs_gate_escape_on_modal_stack` in `tests/dialog_escape_stack.rs`,
  `faction_manager_gates_escape_on_modal_stack` in `tests/faction_manager/dialog_contract.rs`);
  every key a window-level editor listener binds has a catalog row
  (`every_binding_has_a_help_entry` in `tests/help_modal/shortcut_coverage.rs`).

## Related documentation

- [Mission Creator UX specification](/documentation_v2/website/frontend/apps/editor/ux_spec.md)
  — the editor's layout, interaction contract and keyboard shortcuts.
- [Missions domain](/apps/website/api_v2/src/missions/README.md) — the mission row and faction
  library routes the dialogs call.
