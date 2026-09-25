# Admin screen layouts

The place for layouts that only the in-game admin menu uses. It holds none: the admin screen,
`TBD_AdminScreen`, draws on the shared list shell
`apps/mod/tbd-framework/UI/layouts/Common/TBD_ScreenShell.layout`.

## Contents

```text
apps/mod/tbd-framework/UI/layouts/Session/Admin/
```

## Format

- File type: none here; a layout added for the admin menu alone is an
  [Enfusion](/documentation_v2/glossary.md#enfusion) widget layout with its `.layout.meta`, named
  `TBD_Admin<Element>.layout`, with a block from the ledger in
  `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/TBD_UILayouts.c` and a `TBD_UILayouts`
  constant.

## Referenced by

None: no resource refers to this folder. The `TBD_UIAdmin` preset in
`apps/mod/tbd-framework/Configs/System/chimeraMenus.conf` opens `TBD_ScreenShell.layout` with the
`TBD_AdminScreen` class from `apps/mod/tbd-framework/Scripts/Game/TBD/Session/Admin/UI/`.

## Boundaries

- Depends on: nothing.
- Used by: nothing.
- Rules: a layout goes here only when the admin menu alone uses it; one another screen shares goes
  to `apps/mod/tbd-framework/UI/layouts/Common/`.

## Related documentation

- [In-game menu specification](/documentation_v2/mod/tbd-framework/UI/in_game_menu/in_game_menu_specification.md)
  — the in-game menus' design target
- [Admin help ticket specification](/documentation_v2/mod/tbd-framework/UI/admin_help_ticket/admin_help_ticket_specification.md)
  — the admin tickets panel's design target
