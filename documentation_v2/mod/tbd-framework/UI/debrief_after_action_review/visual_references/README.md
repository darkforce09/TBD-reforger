**Status:** live

# Debrief design references

The design references of the [debrief](/documentation_v2/mod/tbd-framework/UI/debrief_after_action_review/debrief_after_action_review_specification.md):
one design-phase Stitch set of a personal after-action card.

## Contents

```text
documentation_v2/mod/tbd-framework/UI/debrief_after_action_review/visual_references/
└── aar_variant_1_tactical_node_assault_chevrons_mockup/  one player's outcome, tallies and kills
```

## Code

- [Post-game overlay scripts](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/PostGame/UI/) and
  [layouts](/apps/mod/tbd-framework/UI/layouts/Session/PostGame/) — the built scoreboard.

## Boundaries

- Depends on: nothing in the repository; the set is self-contained apart from what its html loads
  from the network.
- Used by: the debrief specification and the debrief folder README.
- Rules: a set is kept as captured and never edited to match the built screen; a new set gets its
  own folder and README; no screenshot of the built UI belongs here.
