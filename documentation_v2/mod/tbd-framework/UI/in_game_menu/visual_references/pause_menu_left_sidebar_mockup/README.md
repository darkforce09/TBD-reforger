**Status:** live

# Pause menu sidebar mockup

Design-phase reference for the pause menu: a sidebar of the player's in-game destinations. It gives layout and colour context and is not an implementation source; the built UI is the
[EnfScript](/documentation_v2/glossary.md#enfscript) code the Code section links.

## Contents

```text
documentation_v2/mod/tbd-framework/UI/in_game_menu/visual_references/pause_menu_left_sidebar_mockup/
├── pause_menu_left_sidebar_mockup.html  the Stitch export
└── pause_menu_left_sidebar_mockup.png   its screenshot
```

## How it works

The set shows "Main Menu", "Player Options", "Admin Panel" with an `AUTH` chip, "Contact Admin", "Lobby", "Identity Link" and "Close".

The built pause menu is the game's own, with its leave-faction button relabelled "Change slot" during briefing, safe start and live play, which opens the lobby; the admin screen opens on F8 or `#tbd menu`, and identity linking is `#tbd link`. The [in-game menu specification](/documentation_v2/mod/tbd-framework/UI/in_game_menu/in_game_menu_specification.md) feature doc holds the full comparison.

## Code

- [Lobby screen scripts](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Lobby/UI/) — the
  pause menu extension, the only built part of the menu this set was drawn for.

## Boundaries

- Depends on: the styles, fonts and images the html loads from the network when opened; the png
  needs nothing.
- Used by: the feature doc's Design section and the visual references README.
- Rules: the set is kept as captured; the html and the png stay a pair named after the set.
