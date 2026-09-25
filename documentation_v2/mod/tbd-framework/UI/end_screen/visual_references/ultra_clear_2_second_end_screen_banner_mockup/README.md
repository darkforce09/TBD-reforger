**Status:** live

# End screen banner mockup

Design-phase reference for the END banner: a verdict card read at a glance. It gives layout and
colour context and is not an implementation source; the built banner is the [EnfScript](/documentation_v2/glossary.md#enfscript) and layout
code the Code section links.

## Contents

```text
documentation_v2/mod/tbd-framework/UI/end_screen/visual_references/ultra_clear_2_second_end_screen_banner_mockup/
├── ultra_clear_2_second_end_screen_banner_mockup.html  the Stitch export
└── ultra_clear_2_second_end_screen_banner_mockup.png   its screenshot
```

## How it works

The set shows a glass card with a blue top glow on a dark field: a pill "BLUFOR // US MARINE
CORPS", a large "VICTORY", the headline "ALL OPPOSING FORCES ELIMINATED" and the sentence
"Hostile combatants sustained 100% casualties. Sector secured.". Below the card, "SIMULATE
REASON:" buttons switch the text between "Faction Wiped", "Objective Secured", "Time Limit
Expired" and "Admin Halt"; they are mockup controls, not part of the design.

The built banner, `TBD_EndScreen` in `TBD_EndScreen.layout`, shows "MISSION ENDED", the winning
faction's key and a one-line reason on a plain panel, with no faction colour, no pill and no
headline per reason. The [end screen specification](/documentation_v2/mod/tbd-framework/UI/end_screen/end_screen_specification.md)
feature doc holds the full comparison.

## Code

- [Post-game overlay scripts](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/PostGame/UI/) — the
  built banner this set was drawn for.

## Boundaries

- Depends on: the styles, fonts and images the html loads from the network when opened; the png
  needs nothing.
- Used by: the feature doc's Design section and the visual references README.
- Rules: the set is kept as captured; the html and the png stay a pair named after the set.
