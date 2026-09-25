**Status:** live

# Staging phase panel mockup

Design-phase reference for the pause menu during safe start: team readiness. It gives layout and colour context and is not an implementation source; the built UI is the
[EnfScript](/documentation_v2/glossary.md#enfscript) code the Code section links.

## Contents

```text
documentation_v2/mod/tbd-framework/UI/in_game_menu/visual_references/staging_phase_panel_mockup/
├── staging_phase_panel_mockup.html  the Stitch export
└── staging_phase_panel_mockup.png   its screenshot
```

## How it works

The set shows "Staging Phase" with a `SAFE-START` chip, "Safe-start boundary drops in 01:33", "Alpha 1-1: READY (6/6 Synced)" and the buttons "My Team Is Ready!" and "My Team Is Not Ready!".

The [mod](/documentation_v2/glossary.md#mod) has no readiness vote: safe start ends when its countdown runs out or an admin types `#tbd safestart go`, and its countdown shows only as vanilla pop-ups and chat. The [safe start HUD](/documentation_v2/mod/tbd-framework/UI/safe_start_hud/safe_start_hud_specification.md) and the [in-game menu specification](/documentation_v2/mod/tbd-framework/UI/in_game_menu/in_game_menu_specification.md) feature docs hold the comparison.

## Code

- [Game stages](/apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Stages/) — the safe start this
  set was drawn for.

## Boundaries

- Depends on: the styles, fonts and images the html loads from the network when opened; the png
  needs nothing.
- Used by: the feature doc's Design section and the visual references README.
- Rules: the set is kept as captured; the html and the png stay a pair named after the set.
