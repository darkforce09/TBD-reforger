**Status:** live

# In-game menu reference screenshots

Design-phase reference for the [in-game menu](/documentation_v2/mod/tbd-framework/UI/in_game_menu/in_game_menu_specification.md):
fifteen Arma 3 captures of a community's pause menu and its "WOG Admin Menu", the in-game tools
the TBD menu design started from. They are not an implementation source.

## Contents

```text
documentation_v2/mod/tbd-framework/UI/in_game_menu/visual_references/reference_screenshots/
├── admin_chat.png              Chat: recipient list, history, composer
├── admin_heal_repair.png       Heal and Repair: vehicle roster, schematic, per-target actions
├── admin_home.png              Mission: announcement, timers, safe start, sides, end mission
├── admin_kick_ban.png          Kick and Ban: player list, notification, KICK and BAN
├── admin_radio.png             Radio: player list and four voice routes
├── admin_server_and_other.png  Server: lifecycle buttons and weather controls
├── admin_spawner.png           Spawn: catalogue, class name spawn, quick tools
├── admin_spawner_1.png         Spawn, the medkit tool's tooltip
├── admin_spawner_2.png         Spawn, the spawn-at-cursor tool's tooltip
├── admin_spawner_3.png         Spawn, the delete-at-cursor tool's tooltip
├── admin_teleport.png          Teleport: source and destination lists, map, vehicle toggle
├── home_menu.png               the pause menu during live play, OPTIONS focused
├── home_menu_after_start.png   the pause menu once the round is live
├── home_menu_before_start.png  the pause menu during freeze time, with team readiness
└── player_options.png          the client options dialog
```

## How it works

### Pause menu

The pause menu, titled "Main menu", is a stack on the left over the live world: `OPTIONS`,
`ADMIN PANEL` (for admins), `CONSOLE` and `CLOSE`, the focused button white on black and the rest
olive text on dark slate. A separate button on the right, `FIX UNIFORM BUG` in red, re-equips the
uniform and vest when their models fail to show, without a reconnect. During freeze time
(`home_menu_before_start.png`) `MY TEAM IS READY!` and `MY TEAM IS NOT READY!` sit above it,
signalling a side's readiness to the admin; once the round is live (`home_menu_after_start.png`)
they are gone.

"Options" (`player_options.png`), a compact dialog with an amber title: "View distance" with
steppers for `Terrain` and `Preset 1` to `Preset 3` (read as on foot, ground vehicles and
aircraft); "Sound settings" with `Muting`, an earplug level of `0.01`; and the checks "Save
terrain settings" (off), "Highlight nickname" (on) and "Beep after freeze time" (off); `CLOSE`.

### Admin menu shell

Every admin capture shares a gold header, "WOG Admin Menu: <module>", with the in-game time in the
middle, the server's frame rate on the right and a close cross, and a left rail of eight icons:
a flag (Mission), a radio handset (Radio), a figure in rings (Teleport), speech bubbles (Chat), an
open crate (Spawn), a figure with plus signs (Heal and Repair), a prohibition sign (Kick and Ban)
and a gear and wrench (Server and Other). The active icon is green, with a tooltip.

### Admin modules

- Mission (`admin_home.png`): "Announcement Text" and "Make Announcement"; the hold-fire (safe
  start) timer `01:29` and the round limit `120:00`, each with `-5 min`, `-1 min`, `+1 min` and
  `+5 min`; `HFT ON` and `HFT OFF`; an icon to return to the admin's own body and one for a
  click-to-teleport camera; living counts with a button per side (`EAST`, `WEST`, `GUER`, `CIV`)
  that declares that side the winner; "End Mission Reason" and `End Mission`, disabled until a
  reason and winner are set.
- Radio (`admin_radio.png`): a searchable player list ("Mission Maker ★", the star marking an admin
  or mission maker) and `SR to Player`, `LR to Player`, `SR to All` and `LR to Leaders`, which put
  the admin's voice on the chosen short- or long-range nets.
- Teleport (`admin_teleport.png`): a source list and a destination list, each searchable;
  between them `Become mortal` (toggling invulnerability), `>` and `<` to move one to the other,
  `Map` to teleport by map click, and a toggle that brings the occupied vehicle along.
- Chat (`admin_chat.png`): a recipient list, the conversation history, a composer and `Send`.
- Spawn (`admin_spawner.png` and the three tooltip captures): `VEHICLES` and `ITEMS`; a preset list
  ("Mission Maker ★"); a class name field and `Spawn by class name`; three quick tools that act on a
  double click ("Double click to spawn default personal medicine kit", "Double click to SPAWN
  vehicle or item at cursor target", "Double click to DELETE vehicle at cursor target"); "Clear
  vehicle cargo"; `SPAWN`, disabled until an item is chosen; and a searchable catalogue coloured by
  side: blue for BLUFOR (`M113A3 (MEV)`, `M60A1`, `M923 (Fuel)`, `Mortar 120mm`), red for OPFOR
  (`MT-12 Rapira`, `MT-LB (ZU-23)`, `DShKM`), white for civilian items and crates.
- Heal and Repair (`admin_heal_repair.png`): a vehicle roster with each driver ("M60A1 | No
  driver"), a damage schematic, and two ways to act: a rectangular button on the roster selection
  or a crosshair button on whatever the admin aims at. Actions: `Fuel level` and `Set fuel level`,
  `Hitpoint damage` and `Set hitpoint damage`, `Full heal/repair` (a player is healed and revived),
  `Flip vehicle`, `Detach vehicle`, `Lock/Unlock vehicle` and `Respawn player`.
- Kick and Ban (`admin_kick_ban.png`): a player list, "Notification text", a button that sends it
  as a warning, `KICK` (olive) and `BAN` (maroon, adding the player's platform id to the ban list),
  both with the text as the reason.
- Server and Other (`admin_server_and_other.png`): `Lock/Unlock server`, `Missions list`, `Restart
  server after mission ends`, `Restart server` and `Logout`; and weather with a commit button each:
  time of day (`12:00`, `Set time of day`), overcast, rain and lightning intensity, and fog
  intensity, density and altitude with one `Set fog`.

### Patterns the captures show

Destructive quick tools act on a double click; most actions offer both a list selection and an aim
at the world; sides keep one colour everywhere (blue, red, green, white); the pause menu adds and
removes the readiness buttons with the phase; and the uniform repair is self-service.

The TBD admin screen keeps a player list, respawn and a stage control; the
[in-game menu specification](/documentation_v2/mod/tbd-framework/UI/in_game_menu/in_game_menu_specification.md)
lists what it drops.

## Code

- [Admin screen scripts](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Admin/UI/) — the screen
  the captures informed.

## Boundaries

- Depends on: nothing.
- Used by: the in-game menu specification's Design section and the visual references README.
- Rules: captures are kept as taken; no screenshot of the built UI belongs here.
