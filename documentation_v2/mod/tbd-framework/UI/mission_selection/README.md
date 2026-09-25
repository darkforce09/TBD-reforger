**Status:** live

# Mission selection documentation

The documentation of the Mission Selector, the "Scenario Browser" tab where a player browses
terrains and [missions](/documentation_v2/glossary.md#mission) and picks one for the lobby and
briefing.

## Contents

```text
documentation_v2/mod/tbd-framework/UI/mission_selection/
├── mission_selection_specification.md  the screen as built, the admin deployment path, design
└── visual_references/                  the Stitch mockup sets and the Arma 3 capture
```

## Code

- [Mission Selector screen](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/MissionSelector/UI/)
  — the screen and its panels
- [Mission selection and in-game deployment](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/MissionSelector/)
  — the models, the catalog and the admin deployment relay
- [Mission Selector layouts](/apps/mod/tbd-framework/UI/layouts/Session/MissionSelector/) — the
  shell and panel layouts

## Boundaries

- Depends on: the [feature doc template](/documentation_v2/standards/templates/feature_doc.md).
- Used by: the in-code READMEs of the folders above, which link the specification; the
  [mod UI index](/documentation_v2/mod/tbd-framework/UI/README.md).
- Rules: the specification keeps its path, since the in-code READMEs link it.

## Related documentation

- [Lobby specification](/documentation_v2/mod/tbd-framework/UI/lobby/lobby_specification.md) — the
  screen the top bar's next tab opens
