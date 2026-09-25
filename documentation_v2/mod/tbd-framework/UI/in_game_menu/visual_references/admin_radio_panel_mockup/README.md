**Status:** live

# Admin radio panel mockup

Design-phase reference for the admin menu's Radio module: handing out radios. It gives layout and colour context and is not an implementation source; the built UI is the
[EnfScript](/documentation_v2/glossary.md#enfscript) code the Code section links.

## Contents

```text
documentation_v2/mod/tbd-framework/UI/in_game_menu/visual_references/admin_radio_panel_mockup/
├── admin_radio_panel_mockup.html  the Stitch export
└── admin_radio_panel_mockup.png   its screenshot
```

## How it works

The set shows "Connected Roster (6)" with each player's side, role and whether they carry a long-range and a short-range radio, "Selected Player Actions" ("Give SR Radio", "Give LR Radio", "Give Both (SR + LR)") and "Global Bulk Distribution" ("Give SR to All", "Give LR to leaders").

The built admin screen, `TBD_AdminScreen`, is one list of [mission](/documentation_v2/glossary.md#mission), stage, players and audit with a single respawn or deploy action; it has no such module. The [in-game menu specification](/documentation_v2/mod/tbd-framework/UI/in_game_menu/in_game_menu_specification.md) feature doc holds the full comparison.

## Code

- [Admin screen scripts](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Admin/UI/) — the built
  admin screen this set was drawn for.

## Boundaries

- Depends on: the styles, fonts and images the html loads from the network when opened; the png
  needs nothing.
- Used by: the feature doc's Design section and the visual references README.
- Rules: the set is kept as captured; the html and the png stay a pair named after the set.
