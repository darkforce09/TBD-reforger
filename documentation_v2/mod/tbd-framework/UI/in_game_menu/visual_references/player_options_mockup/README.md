**Status:** live

# Player options mockup

Design-phase reference for the pause menu's Player Options: client preferences. It gives layout and colour context and is not an implementation source; the built UI is the
[EnfScript](/documentation_v2/glossary.md#enfscript) code the Code section links.

## Contents

```text
documentation_v2/mod/tbd-framework/UI/in_game_menu/visual_references/player_options_mockup/
├── player_options_mockup.html  the Stitch export
└── player_options_mockup.png   its screenshot
```

## How it works

The set shows "View Distance" (terrain render distance `3 500 m`, object view distance with presets 1 to 3 at `1 200 m`, `2 000 m` and `3 500 m`), "Audio" (an earplug muting level `0.01 (-99 dB)` on `SHIFT+N`), "Client HUD Preferences" (save terrain settings to profile, highlight nickname on HUD, beep chime after freeze time expires), and "Reset Defaults" and "Apply".

The [mod](/documentation_v2/glossary.md#mod) builds no player options; the game's own settings apply. The [in-game menu specification](/documentation_v2/mod/tbd-framework/UI/in_game_menu/in_game_menu_specification.md) feature doc holds the full comparison.

## Code

- [Lobby screen scripts](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Lobby/UI/) — the
  pause menu extension, the only built part of the menu this set was drawn for.

## Boundaries

- Depends on: the styles, fonts and images the html loads from the network when opened; the png
  needs nothing.
- Used by: the feature doc's Design section and the visual references README.
- Rules: the set is kept as captured; the html and the png stay a pair named after the set.
