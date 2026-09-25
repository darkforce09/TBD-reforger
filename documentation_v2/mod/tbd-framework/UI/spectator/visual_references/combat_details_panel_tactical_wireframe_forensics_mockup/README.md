**Status:** live

# Spectator combat details panel mockup

Design-phase reference for the spectator's death forensics: a panel that tells a dead player how they died and what they did. It gives layout and colour context and is not an implementation source; the built UI is the
[EnfScript](/documentation_v2/glossary.md#enfscript) code the Code section links.

## Contents

```text
documentation_v2/mod/tbd-framework/UI/spectator/visual_references/combat_details_panel_tactical_wireframe_forensics_mockup/
├── combat_details_panel_tactical_wireframe_forensics_mockup.html  the Stitch export
└── combat_details_panel_tactical_wireframe_forensics_mockup.png   its screenshot
```

## How it works

The set shows the header "[LG] Yuki_Hattori" `K.I.A.`, "KILLS: 3" and "KILLER: [XOF] Ales", the fatal blow (weapon, calibre, range, headshot), "Incoming Damage & Anatomical Trauma" with hits by body part on a wireframe, a "Forensic Trauma Log" of each hit with its shooter or source and distance, "Outgoing Combat (Hits & Eliminations)" with each kill's time, weapon and range, and an "Engagement Summary" of shots fired and accuracy.

The built spectator records no hits, weapons, ranges or kills and draws no panel over the camera; its only screen is the roster. The [spectator specification](/documentation_v2/mod/tbd-framework/UI/spectator/spectator_specification.md) feature doc holds the full comparison.

## Code

- [Spectator roster screen](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Spectator/UI/) — the
  built spectator UI this set was drawn for.

## Boundaries

- Depends on: the styles, fonts and images the html loads from the network when opened; the png
  needs nothing.
- Used by: the feature doc's Design section and the visual references README.
- Rules: the set is kept as captured; the html and the png stay a pair named after the set.
