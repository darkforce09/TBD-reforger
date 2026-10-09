# Mission Creator dialogs

The dialogs the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator) raises over the
whole workspace instead of framing the map: the Mission Settings dialog for the open
[mission](/documentation/glossary/g_to_m.md#mission), the [ORBAT](/documentation/glossary/n_to_z.md#orbat)
Manager, the Faction Manager for the faction library, and the floating controls hint with its
shortcut catalog.

## Contents

```text
crates/frontend/workspaces/mission_creator_workspace/src/ui/modals/
├── faction_manager.rs  `FactionManagerDialog`: create, edit and delete the faction library's templates
├── help_modal/         the controls hint card and the shortcut catalog
├── help_modal.rs       the help module tree; re-exports the hint and the catalog
├── mod.rs              the module tree
├── orbat_manager/      the ORBAT Manager: side tree, slot inspector, faction templates
├── orbat_manager.rs    the ORBAT Manager module tree; re-exports the dialog and the template helpers
├── settings_modal/     the Mission Settings dialog, the All Settings list, the Editor Preferences
├── settings_modal.rs   the settings module tree; re-exports the dialog and catalog; inner openers
└── tests/              unit tests for the settings dialogs and the ORBAT Manager
```

## How it works

| Dialog | Module | Opened from | Writes |
|---|---|---|---|
| Mission Settings | `settings_modal` | the top strip's "Mission settings" button and menus | the document's environment, the mission's row |
| ORBAT Manager | `orbat_manager` | the top strip's "ORBAT Manager" button | the document's ORBAT, the faction library |
| Faction Manager | `faction_manager` | the right dock's "Manage factions" button | the faction library |
| Controls hint | `help_modal` | the top strip's Help menu | nothing |

The editor page, `crates/frontend/workspaces/mission_creator_workspace/src/mission_editor.rs`, mounts the first
three; the top strip's
overlays mount the controls hint. None of them is routed. Each takes an `open` signal from its
mount site and owns only the draft of what it edits; document changes go through the map engine's
hosted commands.

The Faction Manager edits the signed-in mission maker's own faction library: each faction's side,
name, role templates (role, tag and a character from the item
[registry](/documentation/glossary/n_to_z.md#registry)) and vehicles (a vehicle and an optional label);
a stored role's loadout is kept as it is. It loads
the library with `GET /api/v1/factions`, saves a new faction with `POST /api/v1/factions` and an
existing one with `PUT /api/v1/factions/{id}`, and deletes with `DELETE /api/v1/factions/{id}`
after the "Delete this faction?" confirmation.

The Mission Settings dialog with its two inner dialogs, the ORBAT Manager and the Faction Manager
register with `core::ui::modal_stack` and answer Escape only while topmost, so stacked dialogs
close in order; the ORBAT Manager also takes its overlay z-index from the stack. A dialog that
binds a key listens on the window.

## Public surface

- `settings_modal::MissionSettingsDialog` and `orbat_manager::OrbatManagerDialog`: mounted by the
  editor page.
- `faction_manager::FactionManagerDialog`: mounted by the editor page.
- `help_modal::{ControlsHint, hint_shown, set_hint_shown}`: mounted and toggled by the top strip.

## Boundaries

- Depends on:
  - the sibling folders of `crates/frontend/workspaces/mission_creator_workspace/src/ui/`: the inspectors (the
    environment gate, the win conditions and spawn modules sections, the zone schema, the
    validation router), the outliner (the ORBAT node model, the drag latch) and the top strip's
    row mirror;
  - `mission_editing_commands::hosted_commands` for every document write;
  - the editor's shell in `crates/frontend/workspaces/mission_creator_session/src/` (`layout` classes,
    `document_commands`, `review_mode`, `world_layer_prefs`) and its bridge (`editor_context`,
    `entity_selection`, the document handle);
  - the foundation crates: the [API](/documentation/glossary/a_to_f.md#api) client and DTOs, `AuthStore`,
    `modal_stack`, `Dialog`, toasts, `MaterialIcon`; over HTTP, `/api/v1/missions/{id}` and
    `/api/v1/factions` of the API's [missions](/documentation/glossary/g_to_m.md#missions) domain.
- Used by:
  - `crates/frontend/workspaces/mission_creator_workspace/src/mission_editor.rs`;
  - the top strip in `crates/frontend/workspaces/mission_creator_workspace/src/ui/docks/top_strip/`, for the
    controls hint;
  - the headless editor smoke tests in `tools/browser_testing/browser_gate_suites/src/editor_smoke_tests/`,
    which find the Mission Settings and ORBAT Manager dialogs by their headings;
  - `cargo xtask verify editor-orbat-coherency`
    (`tools/checks/repository_checks/src/architecture/editor_orbat_coherency.rs`), which scans
    `orbat_manager.rs` and every source file in `orbat_manager/` for banned interface text;
  - the test `orbat_manager_overlay_derives_z_from_the_modal_stack` in
    `crates/frontend/foundation/frontend_ui/src/tests/ui.rs`, which reads `orbat_manager/dialog.rs`.
- Rules: a dialog stacked over another answers Escape only while topmost; every key a
  window-level editor listener binds has a catalog row.

## Related documentation

- [Mission Creator UX specification](/documentation/crates/frontend/workspaces/mission_creator_workspace/ux_spec.md)
  — the editor's layout, interaction contract and keyboard shortcuts.
- [Missions domain](/crates/api/api_missions/src/README.md) — the mission row and faction
  library routes the dialogs call.
- [Mission Creator feature inventory: asset palette](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/right_asset_palette.md) — the Faction Manager behind "Manage factions".
- [Mission Creator feature inventory: top command strip](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/top_command_strip.md) — the Mission Settings dialog.
