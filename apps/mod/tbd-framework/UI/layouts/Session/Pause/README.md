# Pause menu layouts

The place for layouts of the in-game pause menu. It holds none: the pause menu is the game's own
`PauseMenuUI`, which `TBD_LobbyScreen.c` in
`apps/mod/tbd-framework/Scripts/Game/TBD/Session/Lobby/UI/` extends with a "Change slot" button
that reopens the lobby.

## Contents

```text
apps/mod/tbd-framework/UI/layouts/Session/Pause/
```

## Format

- File type: none here; a layout added for the pause menu is an
  [Enfusion](/documentation_v2/glossary.md#enfusion) widget layout with its `.layout.meta`, named
  `TBD_Pause<Element>.layout`, with a block from the ledger in
  `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/TBD_UILayouts.c` and a `TBD_UILayouts`
  constant.

## Referenced by

None: no resource refers to this folder.

## Boundaries

- Depends on: nothing.
- Used by: nothing.
- Rules: a layout goes here only when the pause menu alone uses it; one another screen shares goes
  to `apps/mod/tbd-framework/UI/layouts/Common/`.

## Related documentation

- [In-game menu specification](/documentation_v2/mod/tbd-framework/UI/in_game_menu/in_game_menu_specification.md)
  — the in-game menus' design target
