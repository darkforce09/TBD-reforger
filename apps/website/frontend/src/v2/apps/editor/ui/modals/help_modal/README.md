# Controls hint and shortcut catalog

The [Mission Creator](/documentation_v2/glossary.md#mission-creator)'s floating "Controls —
keyboard shortcuts" card and the catalog of every editor key binding it lists. The parent module,
`apps/website/frontend/src/v2/apps/editor/ui/modals/help_modal.rs`, declares both modules,
re-exports their public items and mounts the keymap census and its tests.

## Contents

```text
apps/website/frontend/src/v2/apps/editor/ui/modals/help_modal/
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

- Depends on: `shell::layout::HOVER_FILL` in `apps/website/frontend/src/v2/apps/editor/shell/`;
  `cn` and `MaterialIcon` from `crate::v2::core::ui`.
- Used by: the top strip in `apps/website/frontend/src/v2/apps/editor/ui/docks/top_strip/`
  (`view.rs` for the shown state, `view/overlays.rs` for the mount), through the parent's
  re-exports; the help tests in `apps/website/frontend/src/v2/apps/editor/ui/modals/tests/help_modal/`.
- Rules: every key code that a window-level editor listener binds has a `SHORTCUTS` row, and no row
  names a code nothing binds (`every_binding_has_a_help_entry` and
  `no_help_entry_invents_a_binding` in
  `apps/website/frontend/src/v2/apps/editor/ui/modals/tests/help_modal/shortcut_coverage.rs`, which
  reads the bindings through the keymap census beside it).

## Related documentation

- [Mission Creator UX specification](/documentation_v2/website/frontend/apps/editor/ux_spec.md)
  — the editor's layout, interaction contract and shipped keyboard shortcuts.
- [Mission Creator feature inventory: keyboard shortcuts](/documentation_v2/website/frontend/apps/editor/feature_inventory/keyboard_shortcuts.md) — the bindings the shortcut list documents.
