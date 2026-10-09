# End and debrief overlays

The two screens that close a round: the END banner with the winning faction and the reason, and
the DEBRIEF scoreboard of kills and deaths per player. Both are workspace overlays, never menus, so
neither can refuse a stage change.

## Contents

```text
mod/tbd-framework/Scripts/Game/TBD/Session/PostGame/UI/
├── TBD_DebriefScreen.c  `TBD_DebriefScreen`: the scoreboard and its row packing
└── TBD_EndScreen.c      `TBD_EndScreen`: the END banner with the winner and the reason
```

## How it works

`TBD_EndBanner.ApplyEndScreens` in
`mod/tbd-framework/Scripts/Game/TBD/Gamemode/Orchestrator/Stage/` runs on every stage change
on a machine with a workspace: it opens `TBD_EndScreen` on `END` and `TBD_DebriefScreen` on
`DEBRIEF`, and closes each on every other stage. `Open` and `Close` are local widget operations
(`TBD_UILayouts.END_SCREEN` and `DEBRIEF_SCREEN`, the layouts in
`mod/tbd-framework/UI/layouts/Session/PostGame/`), so they cannot feed back into the stage.
The banner reads `GetEndWinner` and `GetEndReason` from `TBD_FrameworkManager` under the title
"MISSION ENDED".

The scoreboard reads the board as one packed string. On the authority,
`TBD_EndBanner.PackDebriefBoard` calls `TBD_DebriefScoreboard.Fill` in
`mod/tbd-framework/Scripts/Game/TBD/Session/PostGame/`: one `TBD_DebriefRow` per connected
player with name, faction and role from the assigned slot, deaths 0 or 1 from the one-life record,
and kills from `TBD_MatchTelemetryTally.GetKills` (enemy kills; team kills are counted apart). `PackRows` joins the rows into the replicated
board, one row per `TBD_WireCodec.LINE_SEP` with kills, deaths, faction, role and name separated by
`TBD_WireCodec.FIELD_SEP`, and `UnpackRows` splits it on each client. The list sorts by kills, then
deaths, then name, and its one button, "SORT BY KILLS", and a click on the header row flip the
order.

## Authority

- Server: `TBD_DebriefScoreboard.Fill` returns nothing on a remote client
  (`TBD_Authority.IsClient()`); only the authority builds the board.
- Client: both overlays, opened on each machine with a workspace.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none here; the winner, the reason and the packed board are
  `TBD_FrameworkManager` properties (`m_sEndWinner`, `m_sEndReason`, `m_sDebriefBoard`).

## Boundaries

- Depends on: `TBD_FrameworkManager` in
  `mod/tbd-framework/Scripts/Game/TBD/Gamemode/Orchestrator/`; `TBD_SpawnManager` (slots and
  the one-life record); `TBD_MatchTelemetryTally` in
  `mod/tbd-framework/Scripts/Game/TBD/Systems/MatchEvents/` (the kills); `TBD_UILayouts`, `TBD_UITheme` and `TBD_ListBox` in
  `mod/tbd-framework/Scripts/Game/TBD/UI/Core/`; `TBD_WireCodec` in
  `mod/tbd-framework/Scripts/Game/TBD/Core/Wire/`; the two layouts in
  `mod/tbd-framework/UI/layouts/Session/PostGame/`.
- Used by: `TBD_EndBanner`, which opens and closes both screens and packs the board for
  `TBD_FrameworkManager`.
- Rules: the overlays never block or refuse a stage change; a packed field never holds the row or
  field separator (`SanitizeField`); lines added stay ASCII and `cargo xtask mod compile` checks
  that the scripts compile.

## Related documentation

- [End screen specification](/documentation/mod/tbd-framework/UI/end_screen/end_screen_specification.md)
  — the END banner as built, how a round ends and the design target
- [Debrief specification](/documentation/mod/tbd-framework/UI/debrief_after_action_review/debrief_after_action_review_specification.md)
  — the DEBRIEF scoreboard as built, kill counting and the design target
