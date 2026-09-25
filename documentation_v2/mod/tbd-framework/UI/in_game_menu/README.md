**Status:** live

# In-game menu documentation

The documentation of what a player reaches in game during a round of an
[event](/documentation_v2/glossary.md#event): the pause menu's added action and the admin screen,
with the design references of the fuller menu and admin suite.

## Contents

```text
documentation_v2/mod/tbd-framework/UI/in_game_menu/
├── in_game_menu_specification.md  the menus as built, their wire, design target and decisions
└── visual_references/             the Stitch mockup sets and the Arma 3 captures
```

## Code

- [Admin screen scripts](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Admin/UI/) —
  `TBD_AdminScreen`
- [Admin](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Admin/) — the admin service, client,
  snapshot, audit and `#tbd` commands
- [Lobby screen scripts](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Lobby/UI/) — the pause
  menu's "Change slot"
- [Admin layouts](/apps/mod/tbd-framework/UI/layouts/Session/Admin/) and
  [pause menu layouts](/apps/mod/tbd-framework/UI/layouts/Session/Pause/) — none yet; the admin
  screen uses the shared shell

## Boundaries

- Depends on: the [feature doc template](/documentation_v2/standards/templates/feature_doc.md).
- Used by: the in-code READMEs of the admin and pause folders above, which link the specification;
  the [mod UI index](/documentation_v2/mod/tbd-framework/UI/README.md).
- Rules: the specification keeps its path, since the in-code READMEs link it.

## Related documentation

- [Admin help ticket specification](/documentation_v2/mod/tbd-framework/UI/admin_help_ticket/admin_help_ticket_specification.md)
  — the tickets module designed for the admin suite
