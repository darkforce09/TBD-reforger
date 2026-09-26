**Status:** live

# Lobby screen documentation

The documentation of the Lobby tab, where a player picks a faction and claims a
[slot](/documentation_v2/glossary/n_to_z.md#slot) in the [ORBAT](/documentation_v2/glossary/n_to_z.md#orbat)
before the briefing.

## Contents

```text
documentation_v2/mod/tbd-framework/UI/lobby/
├── lobby_specification.md  the screen as built, its data, design target, open work and decisions
└── visual_references/      the Stitch mockup sets and the Arma 3 capture
```

## Code

- [Lobby screen scripts](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Lobby/UI/) — the screen
  and its panels
- [Lobby and slotting](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Lobby/) — the catalog, the
  stage watcher and the roster wire
- [Lobby layouts](/apps/mod/tbd-framework/UI/layouts/Session/Lobby/) — the shell and panel layouts

## Boundaries

- Depends on: the [feature doc template](/documentation_v2/standards/templates/feature_doc.md).
- Used by: the in-code READMEs of the folders above, which link the specification; the
  [mod UI index](/documentation_v2/mod/tbd-framework/UI/README.md).
- Rules: the specification keeps its path, since the in-code READMEs link it.

## Related documentation

- [Briefing specification](/documentation_v2/mod/tbd-framework/UI/briefing/briefing_specification.md)
  — the ORBAT page that reuses the lobby's roster read only
