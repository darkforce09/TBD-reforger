# Right dock compositions

The right dock's Compositions tab: reusable multi-entity stamps saved from the current selection
into the [mission](/documentation/glossary/g_to_m.md#mission) document, listed by category, armed from
their row and stamped onto the map.

## Contents

```text
crates/frontend/workspaces/mission_creator_workspace/src/ui/docks/dock_right/compositions/
└── mod.rs  `compositions_panel`: save the selection; list, arm, edit and delete the saved rows
```

## How it works

With entities selected, "Save composition… (N selected)" opens "Title" and "Category" fields, and
"Save" calls the map engine's `save_composition` with the title (else "Untitled"), the category
(else "Uncategorized") and the signed-in user's name as the author (else "You"). A save takes the
whole selection: placed entities keep the elevation they were saved at, and a comment in the
selection is captured but stays editor-only, never reaching the compiled mission.

The saved rows (`composition_rows`, in category then title order) sit under their category
headings, each with its title and "by <author> · N item(s)". Pressing a row arms its placement
(`begin_place_composition`); the next map click stamps it at the cursor, and Esc or a right-click
cancels. A row's hover actions edit its title, category and author inline, or delete it, which also
cancels an armed placement of that row. Every change is an undoable document edit through
`mission_editing_commands::hosted_commands`, and bumps `doc_tick`. The panel renders only in the
browser build; the native `compositions_panel` draws nothing.

## Boundaries

- Depends on: `mission_editing_commands::hosted_commands` (`save_composition`,
  `composition_rows`, `composition_count`, `rename_composition`, `recategorize_composition`,
  `set_composition_author`, `delete_composition`) and `mission_editing_session::host`
  (`selection_len`); `bridge::host_state::armed_placement` and `editor_context` for arming and
  cancelling; the outliner's `ROW` style; `AuthStore` from `frontend_session` for the author.
- Used by: the Compositions tab of `DockRight` in
  `crates/frontend/workspaces/mission_creator_workspace/src/ui/docks/dock_right/shell/layout.rs`; the tests in
  `crates/frontend/workspaces/mission_creator_workspace/src/ui/docks/tests/dock_right/`.
- Rules: the Compositions tab maps to its own sub-mode
  (`compositions_tab_maps_to_its_own_submode` in that folder's `compositions.rs`); a capture keeps
  comments and each entity's authored elevation; arming uses the editor's shared
  pending-placement state.

## Related documentation

- [Mission Creator feature inventory: asset palette](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/right_asset_palette.md) — the Compositions tab.
- [Mission Creator feature inventory: placement](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/placement.md) — stamping a composition.
