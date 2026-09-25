**Status:** live

# Debrief screen documentation

The documentation of the DEBRIEF scoreboard, the last screen of a round of an
[event](/documentation_v2/glossary.md#event): the winner and each player's kills and deaths.

## Contents

```text
documentation_v2/mod/tbd-framework/UI/debrief_after_action_review/
├── debrief_after_action_review_specification.md  the scoreboard as built, its design target, open work
└── visual_references/                            the Stitch mockup set of a personal after-action card
```

## Code

- [Post-game overlay scripts](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/PostGame/UI/) —
  `TBD_DebriefScreen` and the scoreboard snapshot
- [Round orchestrator](/apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Orchestrator/) — the kill
  count and the packed board
- [Post-game layouts](/apps/mod/tbd-framework/UI/layouts/Session/PostGame/) —
  `TBD_DebriefScreen.layout`

## Boundaries

- Depends on: the [feature doc template](/documentation_v2/standards/templates/feature_doc.md).
- Used by: the in-code READMEs of the folders above, which link the specification; the
  [mod UI index](/documentation_v2/mod/tbd-framework/UI/README.md).
- Rules: the specification keeps its path, since the in-code READMEs link it.

## Related documentation

- [End screen specification](/documentation_v2/mod/tbd-framework/UI/end_screen/end_screen_specification.md)
  — the banner that precedes the scoreboard and decides its winner
