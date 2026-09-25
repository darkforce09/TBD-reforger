**Status:** live

# Admin help ticket documentation

The documentation of the admin help ticket, a designed but unbuilt way for a player to ask the
admins for help during a round of an [event](/documentation_v2/glossary.md#event).

## Contents

```text
documentation_v2/mod/tbd-framework/UI/admin_help_ticket/
├── admin_help_ticket_specification.md  the design target, and what exists instead
└── visual_references/                  the Stitch mockup set of the admin tickets module
```

## Code

- [Admin](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Admin/) — the admin screen and `#tbd`
  commands an admin uses today; no ticket code exists

## Boundaries

- Depends on: the [feature doc template](/documentation_v2/standards/templates/feature_doc.md).
- Used by: the in-code READMEs of the admin folders, which link the specification; the
  [mod UI index](/documentation_v2/mod/tbd-framework/UI/README.md).
- Rules: the specification keeps its path, since the in-code READMEs link it.

## Related documentation

- [In-game menu specification](/documentation_v2/mod/tbd-framework/UI/in_game_menu/in_game_menu_specification.md)
  — the admin menu the tickets module belongs to
