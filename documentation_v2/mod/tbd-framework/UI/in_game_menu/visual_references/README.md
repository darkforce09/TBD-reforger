**Status:** live

# In-game menu design references

The design references of the [in-game menu](/documentation_v2/mod/tbd-framework/UI/in_game_menu/in_game_menu_specification.md):
thirteen design-phase Stitch sets, one per pause menu panel or admin module, and the Arma 3
captures of a community admin menu the design started from.

## Contents

```text
documentation_v2/mod/tbd-framework/UI/in_game_menu/visual_references/
├── admin_chat_panel_mockup/          Chat: direct whispers with a log and macros
├── admin_environment_panel_mockup/   Environment: time of day, acceleration and weather
├── admin_heal_repair_panel_mockup/   Heal and Repair: casualties and damaged vehicles
├── admin_home_panel_mockup/          Home: announcement, timers, end mission, pause
├── admin_kick_ban_panel_mockup/      Kick and Ban: flagged players, warnings, bans
├── admin_panel_sidebar_mockup/       the admin sidebar: ten modules
├── admin_radio_panel_mockup/         Radio: giving short- and long-range radios
├── admin_server_panel_mockup/        Server: access, mission, restart, telemetry, console
├── admin_spawn_panel_mockup/         Spawn: the asset catalogue by side
├── admin_teleport_panel_mockup/      Teleport: source, destination and transfer
├── pause_menu_left_sidebar_mockup/   the pause sidebar: options, admin, contact, lobby, link
├── player_options_mockup/            Player Options: view distance, muting, HUD preferences
├── reference_screenshots/            fifteen Arma 3 captures of the pause and admin menus
└── staging_phase_panel_mockup/       the safe start readiness panel
```

## How it works

A mockup set is a folder named `<panel>_mockup` holding the Stitch export as an html file and its
screenshot as a png, both named after the set, and a README that says what the set shows and how
the built menu differs. `reference_screenshots/` holds in-game captures of another game. The built
menu is the code the Code section links; the in-game menu specification's Design section lists the
differences.

## Code

- [Admin screen scripts](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Admin/UI/) and the
  [pause menu extension](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Lobby/UI/) — the built
  menus.

## Boundaries

- Depends on: nothing in the repository; each set is self-contained apart from what its html loads
  from the network.
- Used by: the in-game menu specification and the in-game menu folder README.
- Rules: a set is kept as captured and never edited to match the built screen; a new set gets its
  own folder and README; no screenshot of the built UI belongs here.
