**Status:** live

# Admin server panel mockup

Design-phase reference for the admin menu's Server module: the server process and the round's lifecycle. It gives layout and colour context and is not an implementation source; the built UI is the
[EnfScript](/documentation_v2/glossary.md#enfscript) code the Code section links.

## Contents

```text
documentation_v2/mod/tbd-framework/UI/in_game_menu/visual_references/admin_server_panel_mockup/
├── admin_server_panel_mockup.html  the Stitch export
└── admin_server_panel_mockup.png   its screenshot
```

## How it works

The set shows tabs for "SERVER OPERATIONS", "NETWORK & TELEMETRY" and "LOGS & SCRIPT ENGINE": access control ("UNLOCKED", "Lock Server"), the active [mission](/documentation_v2/glossary.md#mission) (`TBD_Montignac_Counterattack_v1.et` on Everon) with "Open Missions List", auto-restart after the match, "Reboot Dedicated Daemon" and "Admin Session Logout"; network ingress and egress, entity count and desync; a console command field; and CPU, memory and ping.

The built admin screen has no server module; `#tbd missions` and `#tbd mission <n>` list and deploy missions in chat. The [in-game menu specification](/documentation_v2/mod/tbd-framework/UI/in_game_menu/in_game_menu_specification.md) feature doc holds the full comparison.

## Code

- [Admin screen scripts](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Admin/UI/) — the built
  admin screen this set was drawn for.

## Boundaries

- Depends on: the styles, fonts and images the html loads from the network when opened; the png
  needs nothing.
- Used by: the feature doc's Design section and the visual references README.
- Rules: the set is kept as captured; the html and the png stay a pair named after the set.
