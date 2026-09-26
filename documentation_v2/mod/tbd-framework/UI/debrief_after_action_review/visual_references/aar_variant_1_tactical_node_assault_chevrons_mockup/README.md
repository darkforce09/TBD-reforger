**Status:** live

# After-action review card mockup

Design-phase reference for the debrief: a personal after-action card for one player. It gives
layout and colour context and is not an implementation source; the built scoreboard is the
[EnfScript](/documentation_v2/glossary/a_to_f.md#enfscript) and layout code the Code section links.

## Contents

```text
documentation_v2/mod/tbd-framework/UI/debrief_after_action_review/visual_references/aar_variant_1_tactical_node_assault_chevrons_mockup/
├── aar_variant_1_tactical_node_assault_chevrons_mockup.html  the Stitch export
└── aar_variant_1_tactical_node_assault_chevrons_mockup.png   its screenshot
```

## How it works

The set shows a banner "VICTORY" over "OPERATION RED DAWN" with the player's chips (`BLUFOR`,
`ASSAULT`, "ALPHA 1-1" and `SL`) and the side's totals ("OBJECTIVES 2 / 2", "ENEMIES KILLED
10 / 10"); tiles for "LIFESPAN" (`38m 24s`), "OBJECTIVES CAPTURED", "ENEMY KILLS", "FRIENDLY
KILLS", "ENEMY VEHICLES" and "FRIENDLY VEHICLES"; and four lists: "Enemy Infantry Kills" (each
kill's name, weapon and distance), "Enemy Vehicles Destroyed", "Friendly Infantry Kills" ("No
Friendly Fire Recorded") and "Friendly Vehicles Destroyed" ("No Friendly Armor Destroyed"). A
"SIMULATE STATE:" toggle between "SURVIVED" and "K.I.A." is a mockup control.

The built screen, `TBD_DebriefScreen`, is a server-wide list of every player's name, faction,
role and kill and death counts; it records no lifespan, objective credit, vehicle kill, weapon or
distance. The [debrief specification](/documentation_v2/mod/tbd-framework/UI/debrief_after_action_review/debrief_after_action_review_specification.md)
feature doc holds the full comparison.

## Code

- [Post-game overlay scripts](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/PostGame/UI/) — the
  built scoreboard this set was drawn for.

## Boundaries

- Depends on: the styles, fonts and images the html loads from the network when opened; the png
  needs nothing.
- Used by: the feature doc's Design section and the visual references README.
- Rules: the set is kept as captured; the html and the png stay a pair named after the set.
