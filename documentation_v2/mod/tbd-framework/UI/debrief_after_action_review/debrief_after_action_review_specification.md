**Status:** live

# Debrief screen

The DEBRIEF scoreboard: after the END banner, every player with a screen sees the winner and one
row per connected player with their faction, role, kills and deaths, sortable by kills. It is the
last stage of a round; it stays up until an admin moves the round on.

## Where it lives

- Code: [`apps/mod/tbd-framework/Scripts/Game/TBD/Session/PostGame/UI/`](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/PostGame/UI/README.md)
  (`TBD_DebriefScreen.c`, which also adds `TBD_ResultsReporter.FillScoreboard`) and the stage
  machine in
  [`apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Orchestrator/`](/apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Orchestrator/README.md)
  (`TBD_FrameworkManager.c`), which counts kills and holds the packed board.
- Layout: [`apps/mod/tbd-framework/UI/layouts/Session/PostGame/`](/apps/mod/tbd-framework/UI/layouts/Session/PostGame/README.md),
  `TBD_DebriefScreen.layout`.
- Entry: `TBD_FrameworkManager.ApplyEndScreens`, which opens `TBD_DebriefScreen` on the `DEBRIEF`
  stage and closes it on every other stage.
- Related features: the [end screen](/documentation_v2/mod/tbd-framework/UI/end_screen/end_screen_specification.md),
  which precedes it and shares its winner; the
  [in-game menu](/documentation_v2/mod/tbd-framework/UI/in_game_menu/in_game_menu_specification.md),
  whose admin screen forces stages.

## Behaviour

### Counting

1. While the stage is `LIVE`, the server credits a kill to the killer each time one player kills
   another (`TBD_FrameworkManager.OnPlayerKilled`). A kill with no killing player (AI, the world)
   and a suicide are not counted; a kill of a player on the killer's own side is counted like any
   other.
2. Deaths come from the one-life record: 1 when `TBD_SpawnManager.IsPlayerDead` holds for the
   player, else 0.

### The board

1. On entering `END`, and again on entering `DEBRIEF`, the server builds one row per connected
   player (`FillScoreboard`): the player name ("Player <id>" when the name is empty), the faction
   and role of the assigned [slot](/documentation_v2/glossary.md#slot), the kills and the deaths. A
   player who disconnected before the snapshot has no row.
2. An admin moves `END` to `DEBRIEF` with `#tbd stage next` or the admin screen's "Force stage";
   nothing does it on a timer.
3. The screen shows the title "DEBRIEF" and the subtitle "Winner: <faction key>", or "Scoreboard"
   when no winner was named.
4. The list opens with a header row, "PLAYER" and "KILLS / DEATHS", then one row per player:
   `<name>  [<faction>]  <role>` with `<kills> / <deaths>`.
5. Rows sort by kills, high first; ties sort by fewer deaths, then by name. The "SORT BY KILLS"
   button and a click on the header row flip the kill order. The footer says "Sorted by kills
   (high first). Next stage closes this screen." or "Sorted by kills (low first). Next stage
   closes this screen.".
6. The layout's "BACK" button closes the scoreboard on that machine only.
7. `#tbd stage next` goes no further than `DEBRIEF`; an admin names the next stage
   (`#tbd stage LOBBY`) or deploys the next [mission](/documentation_v2/glossary.md#mission). Any
   stage other than `DEBRIEF` closes the scoreboard; `LOADING` and `LOBBY` clear the board.

### Known discrepancies

- The kill credit reads "Team-kills and world/AI kills are ignored."
  (`TBD_FrameworkManager.c`, the comment on `OnPlayerKilled`) — but the code skips only a missing
  killer and a suicide, so a team kill adds a kill.
- `m_sDebriefBoard` is documented as `kills\tfaction\trole\tname` per line
  (`TBD_FrameworkManager.c`) — but `TBD_DebriefScreen.PackRows` writes five fields: kills, deaths,
  faction, role and name.

## Data

The screen makes no HTTP call and sends no RPC. It reads replicated `TBD_FrameworkManager`
properties:

- `m_sDebriefBoard` (`GetDebriefBoard`): the rows packed on the authority by
  `TBD_DebriefScreen.PackRows`, one line per player and tab-separated fields; `SanitizeField`
  replaces a tab or line break inside a field with a space, and `UnpackRows` rejoins a name that
  still splits.
- `m_sEndWinner` (`GetEndWinner`): the winner, written at `END`; the
  [end screen](/documentation_v2/mod/tbd-framework/UI/end_screen/end_screen_specification.md#data)
  gives how it is decided.
- Kills live in `m_mKills`, a server-only map from player id to kills, cleared with the board.

The same round's results reach the platform separately: `TBD_ResultsReporter` posts them to
`POST /api/v1/ingest/match-results`, the [match telemetry](/documentation_v2/glossary.md#match-telemetry)
ingest.

## Design

- As built: the list shell's shape (`TBD_ScreenShell.layout`): a `SCRIM` backdrop, a `SURFACE`
  panel, the title and subtitle, a `TBD_ListBox` of plain rows, the footer status and one primary
  action, and a "BACK" button; the empty-list text is "No players recorded.". Texts draw in the
  engine's default font; no faction colour, grouping or column layout is applied.
- The specification's target, a server-wide scoreboard, and the built screen lacks all of it:
  - a header rail with the mission name and the round's elapsed time, and an outcome banner in
    the winner's colour (BLUFOR blue, OPFOR red, INDFOR olive) with the deciding trigger;
  - a view switcher: all factions, one faction, or a split view, each tab with its alive-of-total
    count;
  - a roster grouped by squad, each row with rank and name, role, a "[Linked]" platform-identity
    badge, `ALIVE` or `KIA`, kills, deaths, team kills, objective credits and a score, sortable by
    any column;
  - a footer with "CLOSE / SPECTATE WORLD" (Tab reopens the board), a countdown to the next match
    (120 s by default) and "RETURN TO LOBBY NOW" for admins.
- Design target: the [after-action review mockup](/documentation_v2/mod/tbd-framework/UI/debrief_after_action_review/visual_references/aar_variant_1_tactical_node_assault_chevrons_mockup/README.md),
  a design-phase reference that draws a personal after-action card instead: the player's outcome,
  faction, role and squad, lifespan, kill and vehicle tallies, and each kill with its weapon and
  distance. The built screen records none of the per-kill weapon, distance or vehicle data it
  needs.
- No open ticket covers these differences.

## Open work

None. Checked `.ai/tickets/` for open tickets on the debrief, the scoreboard and kill counting;
the website's replay and combat telemetry tickets change the platform, not this screen.

## Decisions

- Kills are counted on `TBD_FrameworkManager`, not in `TBD_ResultsReporter`: the reporter tracks
  deaths for one life and posts results, and the scoreboard lives as a modded method so the
  reporter file keeps one owner.
- The board replicates as one packed string, built once per stage change on the authority: a
  client holds no mission or slot state, and one property is cheaper than a row per player.
- The scoreboard closes only on a stage change, like the END banner, so no screen can hold the
  round.
