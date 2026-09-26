**Status:** live

# In-game menu

What a player reaches in game without leaving the round: the game's own pause menu, to which the
[mod](/documentation_v2/glossary/g_to_m.md#mod) adds one button, and the admin screen, one list through which a listed server admin reads the
[mission](/documentation_v2/glossary/g_to_m.md#mission), the stage and every player, and recovers a
player. The chat commands `#tbd …` cover the admin powers the screen does not.

## Where it lives

- Pause menu: `modded class PauseMenuUI` in `TBD_LobbyScreen.c`, in
  [`apps/mod/tbd-framework/Scripts/Game/TBD/Session/Lobby/UI/`](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Lobby/UI/README.md).
- Admin screen: [`apps/mod/tbd-framework/Scripts/Game/TBD/Session/Admin/UI/`](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Admin/UI/README.md)
  (`TBD_AdminScreen`) over the admin service, client, snapshot and audit in
  [`apps/mod/tbd-framework/Scripts/Game/TBD/Session/Admin/`](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Admin/README.md),
  whose README lists every `#tbd` command.
- Layouts: the game's pause menu layout; the admin screen uses the shared list shell,
  `apps/mod/tbd-framework/UI/layouts/Common/TBD_ScreenShell.layout`.
  [`apps/mod/tbd-framework/UI/layouts/Session/Admin/`](/apps/mod/tbd-framework/UI/layouts/Session/Admin/README.md)
  and [`Session/Pause/`](/apps/mod/tbd-framework/UI/layouts/Session/Pause/README.md) hold none.
- Entry: the `TBD_AdminMenu` action (F8) or `#tbd menu`, both through `TBD_AdminClient.Open`, on
  the `TBD_UIAdmin` menu preset.
- Related features: the [lobby](/documentation_v2/mod/tbd-framework/UI/lobby/lobby_specification.md),
  which "Change slot" opens; the
  [admin help ticket](/documentation_v2/mod/tbd-framework/UI/admin_help_ticket/admin_help_ticket_specification.md),
  designed as a module of this menu; the
  [safe start HUD](/documentation_v2/mod/tbd-framework/UI/safe_start_hud/safe_start_hud_specification.md),
  whose readiness panel is a set here.

## Behaviour

### Pause menu

1. The pause menu is the game's own. During `BRIEFING`, `SAFE_START` and `LIVE` in a framework
   world, the mod relabels its leave-faction button "Change slot"; pressing it closes the pause
   menu and opens the lobby. In every other stage the pause menu is unchanged.

### Admin screen

1. F8 or `#tbd menu` opens the screen, titled "ADMIN", on the caller's machine; F8 again closes it.
   On opening it shows "Asking the server…", asks for a snapshot, and asks again every 3 s while
   open.
2. The server checks the caller against the game's listed admins. A non-admin's snapshot carries
   only the refusal, so the subtitle reads "Not authorised" and the screen shows nothing else.
3. The subtitle reads "<stage> · <n> connected · <n> lives spent".
4. MISSION: the mission and its terrain, or "none loaded", and "Validation" with "PASSED — <n>
   warning(s)", "FAILED — <n> error(s), <n> warning(s)" or "not run yet"; picking it unfolds the
   findings.
5. STAGE: the current stage and "Force stage -> <next>". The first pick arms it ("ARMED — pick again
   to move the whole round"); the second moves the round. It reads "already at the last stage" at
   `DEBRIEF`.
6. PLAYERS: every connected player with an admin tag, faction, group and role (or "no slot"), and
   "in world", "LIFE SPENT" or "NO BODY".
7. The one primary action follows the selected player: "RESPAWN <name>" for a spent life, which
   gives the life back and rebuilds the player on their own [slot](/documentation_v2/glossary/n_to_z.md#slot), and "DEPLOY <name>" for a live
   player with no body. The footer explains the action before it runs and shows the server's answer
   after; nothing opens a modal.
8. ADMIN ACTIONS: "Show the audit trail" unfolds the newest 20 entries with their times.

### Chat commands

Every other power is a `#tbd` command for listed admins: the mission list and [mission deployment](/documentation_v2/glossary/g_to_m.md#mission-deployment), the
backend address, validation, the dead list, respawn and deploy by player id, forcing any stage,
safe start, the one-life identity waiver, the audit replay and `menu`. Everyone else reads "TBD:
admin only."; `#tbd link` alone is open to every player. The Admin README's
[How it works](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Admin/README.md#how-it-works) has
the full table.

### Known discrepancies

None found: every action the screen offers is one the server's `TBD_AdminService.Execute` serves.

## Data

The screen makes no HTTP call. Its wire, on the modded `SCR_PlayerController`
(`TBD_MissionBrowser.c`):

- `TBD_RequestAdminSnapshot` and `TBD_RequestAdminAction` go to the server; the caller is the player
  of the controller the request arrived on, never an argument.
- `TBD_RpcDo_AdminSnapshot` answers with the payload one admin may see: the mission, the stage,
  the validation findings, the players and the newest 20 audit lines, as tab-separated lines of at
  most 400 characters, each field prefixed with `.` so none is empty on the wire.
- `TBD_RpcDo_AdminActionResult` carries the server's verdict, shown verbatim.
- Every action and every refused attempt goes into `TBD_AdminAudit` (at most 60 entries, of which
  refusals may hold 12) and the server log.

## Design

- As built: the game's pause menu with one relabelled button, and a single list over the shared
  shell with four sections and one primary action, painted from `TBD_UITheme`.
- Design target: the thirteen Stitch sets in
  [visual_references](/documentation_v2/mod/tbd-framework/UI/in_game_menu/visual_references/README.md),
  design-phase references, and the Arma 3 captures of a community's admin menu in
  [reference_screenshots](/documentation_v2/mod/tbd-framework/UI/in_game_menu/visual_references/reference_screenshots/README.md)
  the design started from. The target is a pause menu of its own and an admin suite of modules
  behind a sidebar; the built menu has none of the following:
  - a pause sidebar with "Main Menu", "Player Options", "Admin Panel", "Contact Admin", "Lobby",
    "Identity Link" and "Close";
  - player options: terrain and object view distance with three presets, an earplug muting level,
    saving terrain settings to the profile, highlighting the player's own name, and a chime when
    safe start ends;
  - a staging panel during safe start with the countdown and "My Team Is Ready!" and "My Team Is
    Not Ready!", and the captures' "FIX UNIFORM BUG" self-repair;
  - an admin sidebar of Home, Radio, Teleport, Chat, Tickets, Spawn, Heal and Repair, Kick and Ban,
    Server and Environment, under a header with the in-game time and the server's frame rate;
  - Home: a broadcast announcement; safe start and round time adjusted in 1 and 5 minute steps;
    each side's living count; ending the mission with a chosen winner and reason; pausing the
    mission;
  - Radio: giving short- and long-range radios to one player, to everyone or to leaders;
  - Teleport: moving any player or vehicle to another, to an objective or base, or by map click;
  - Chat: direct whispers to one player, with a log and quick macros;
  - Spawn: a vehicle and item catalogue by side, spawning by class name or at the cursor, and
    deleting at the cursor;
  - Heal and Repair: health, fuel and hit-point damage per player or vehicle, full heal, flip,
    detach and lock;
  - Kick and Ban: the player list with warnings and flags, kick and ban with a reason;
  - Server: locking the server, the mission list, restart after the round or now, network and
    resource telemetry, and a console;
  - Environment: time of day and acceleration, overcast, rain, lightning and fog.
- Of the target, the built admin screen covers the player list, respawn (the captures' "Respawn
  player"), deploy, a forced stage in place of "End Mission", and the mission list through
  `#tbd missions`.
- No open ticket covers these differences.

## Open work

None. Checked `.ai/tickets/` for open tickets on the pause menu, the admin menu and its modules;
the website's server control tickets change the platform, not this menu.

## Decisions

- One list over the shared shell rather than a suite of panels: the screen holds no power of its
  own, only a view of what the server sends and one action at a time, so each new power is a
  server-side action first.
- Forcing a stage takes two picks: moving the whole round is irreversible.
- The admin check runs on the server against the game's admin list for every request: the client
  cannot claim to be an admin, and a non-admin's screen has nothing to draw.
- The pause menu is extended, not replaced: the game's pause menu keeps its own options, and the
  mod adds only the action its stages need.
