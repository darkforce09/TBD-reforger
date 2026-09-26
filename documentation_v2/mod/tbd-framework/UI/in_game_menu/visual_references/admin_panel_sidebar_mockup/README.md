**Status:** live

# Admin panel sidebar mockup

Design-phase reference for the admin menu's navigation: one sidebar entry per module. It gives layout and colour context and is not an implementation source; the built UI is the
[EnfScript](/documentation_v2/glossary/a_to_f.md#enfscript) code the Code section links.

## Contents

```text
documentation_v2/mod/tbd-framework/UI/in_game_menu/visual_references/admin_panel_sidebar_mockup/
├── admin_panel_sidebar_mockup.html  the Stitch export
└── admin_panel_sidebar_mockup.png   its screenshot
```

## How it works

The set shows "Admin Panel" with an `AUTH` chip and entries Home, Radio, Teleport, Chat, Tickets, Spawn, Heal and Repair, Kick and Ban, Server and Environment, each with an icon and a chevron.

The built admin screen has no modules and no sidebar: one list with four sections. The [in-game menu specification](/documentation_v2/mod/tbd-framework/UI/in_game_menu/in_game_menu_specification.md) feature doc holds the full comparison.

## Code

- [Admin screen scripts](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Admin/UI/) — the built
  admin screen this set was drawn for.

## Boundaries

- Depends on: the styles, fonts and images the html loads from the network when opened; the png
  needs nothing.
- Used by: the feature doc's Design section and the visual references README.
- Rules: the set is kept as captured; the html and the png stay a pair named after the set.
