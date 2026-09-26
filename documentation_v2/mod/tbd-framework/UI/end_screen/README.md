**Status:** live

# End screen documentation

The documentation of the END banner, the overlay that names the winning faction and the reason a
round of an [event](/documentation_v2/glossary/a_to_f.md#event) ended.

## Contents

```text
documentation_v2/mod/tbd-framework/UI/end_screen/
├── end_screen_specification.md  the banner as built, its data, design target, open work and decisions
└── visual_references/           the Stitch mockup set of the banner
```

## Code

- [Post-game overlay scripts](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/PostGame/UI/) —
  `TBD_EndScreen`, the banner
- [Round orchestrator](/apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Orchestrator/) — the
  stage machine that records the winner and the reason and opens the banner
- [Post-game layouts](/apps/mod/tbd-framework/UI/layouts/Session/PostGame/) — `TBD_EndScreen.layout`

## Boundaries

- Depends on: the [feature doc template](/documentation_v2/standards/templates/feature_doc.md).
- Used by: the in-code READMEs of the folders above and of the stage scripts, which link the
  specification; the [mod UI index](/documentation_v2/mod/tbd-framework/UI/README.md).
- Rules: the specification keeps its path, since the in-code READMEs link it.

## Related documentation

- [Debrief specification](/documentation_v2/mod/tbd-framework/UI/debrief_after_action_review/debrief_after_action_review_specification.md)
  — the scoreboard the next stage opens
