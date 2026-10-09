# Controls hint and shortcut catalog

The [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s floating "Controls —
keyboard shortcuts" card and the catalog of every editor key binding it lists. The parent module,
`crates/frontend/workspaces/mission_creator_workspace/src/ui/modals/help_modal.rs`, declares both modules,
re-exports their public items.

## Contents

```text
crates/frontend/workspaces/mission_creator_workspace/src/ui/modals/help_modal/
├── controls_hint.rs     `ControlsHint`: the floating shortcut card, and whether it shows
└── shortcut_catalog.rs  `SHORTCUTS` and `GROUPS`: each binding's key codes, chord, action and group
```

## How it works

The top strip's Help menu toggles the card, and the top strip's overlays mount it. `hint_shown`
and `set_hint_shown` keep its visibility in a thread-local, so the card stays open across a remount
of the chrome, and its close button hides it. The card lists `SHORTCUTS` under the seven `GROUPS`
headings ("Selection", "View", "Transform & snapping", "Arrange", "History", "Tools", "Context
menu"), each row carrying its `KeyboardEvent` codes in `data-codes`.

## Boundaries

- Depends on: `HOVER_FILL` from `frontend_ui::tokens`;
  `cn` and `MaterialIcon` from `frontend_ui`.
- Used by: the top strip in `crates/frontend/workspaces/mission_creator_workspace/src/ui/docks/top_strip/`
  (`view.rs` for the shown state, `view/overlays.rs` for the mount), through the parent's
  re-exports.
- Rules: every key code that a window-level editor listener binds has a `SHORTCUTS` row, and no row
  names a code nothing binds.

## Related documentation

- [Mission Creator UX specification](/documentation/crates/frontend/workspaces/mission_creator_workspace/ux_spec.md)
  — the editor's layout, interaction contract and shipped keyboard shortcuts.
- [Mission Creator feature inventory: keyboard shortcuts](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/keyboard_shortcuts.md) — the bindings the shortcut list documents.
