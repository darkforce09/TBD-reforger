# Objective and task HUD

The live-round displays a player sees: the objective board with its capture bar, drawn over the
game, and the [mission](/documentation_v2/glossary.md#mission)'s assigned tasks, drawn as markers
on the map. The server composes both and sends each client its own snapshot.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/UI/Hud/
├── TBD_ObjectiveHud.c  TBD_ObjectiveHud: the objective board and capture bar, and its RPC pair
└── TBD_TaskHud.c       TBD_TaskHud: assigned tasks as map markers, and its RPC pair
```

## How it works

`TBD_ObjectiveHud` is the `ScriptedWidgetComponent` on the root of
`apps/mod/tbd-framework/UI/layouts/Hud/TBD_ObjectiveHud.layout` (`TBD_UILayouts.OBJECTIVE_HUD`).
`TBD_ObjectivesComponent` in `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Objectives/` calls
`SCR_PlayerController.TBD_PushObjectiveHud` with one player's icons, titles, details and bar; on a
listen host the host's own snapshot is applied directly, anyone else's goes out as an owner RPC.
`Accept` stores the snapshot, opens the layout when it is closed and repaints only when the
snapshot's signature changed: a `TBD_ListBox` row per objective, `[icon] title` with the detail
line, and the capture bar filled to its percentage. A snapshot with `show` 0 closes the HUD, which
the server sends when the round leaves `LIVE`. Colours come from `TBD_UITheme` (a glass panel over
the world).

`TBD_TaskHud` draws each assigned task that has a position as an `SCR_MapMarkerBase` static marker
(`PLACED_CUSTOM`, the task's icon through `TBD_MarkerIcons`, its title as the marker text) through
the map's `SCR_MapMarkerManagerComponent`. `TBD_TaskStateMachine` pushes the snapshot to every
player when a task changes, and each client also asks for it once a second; an unchanged snapshot
is ignored, and a task that leaves `assigned` disappears because the snapshot omits it. Tasks are
not side-scoped: every player gets the same snapshot.

## Authority

- Server: the snapshots, built by `TBD_ObjectivesComponent` and `TBD_TaskHud.BuildSnapshot`
  from server-owned state; `TBD_PushObjectiveHud`, `TBD_PushTaskHud` and `TBD_TaskHud.PushToPlayers`
  return on `RplMode.Client`. These methods carry no `@authority` tag.
- Client: `Accept` in both classes paints what arrives; `TBD_RequestTaskHud` asks the server for
  the task snapshot.
- Owner: each RPC reply goes to the requesting or addressed player's controller only.
- RPCs, on the modded `SCR_PlayerController`, as their `@rpc` tags state:
  - `TBD_RpcAsk_ObjectiveHud`: Reliable, Server; answers with that player's board;
  - `TBD_RpcDo_ObjectiveHud`: Reliable, Owner; the board, the bar and `show`;
  - `TBD_RpcAsk_TaskHud`: Reliable, Server; answers with the task snapshot;
  - `TBD_RpcDo_TaskHud`: Reliable, Owner; positions, icons, titles, ids and states.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_ObjectivesComponent` and `TBD_TaskStateMachine` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Objectives/`; `TBD_MarkerClient` and
  `TBD_MarkerIcons` in `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Markers/`; `TBD_UILayouts`,
  `TBD_UITheme` and `TBD_ListBox` in `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/`; `TBD_Log`;
  the engine's `SCR_MapMarkerManagerComponent`; the `task` definition in
  `contracts_v2/definitions/mission.schema.json`.
- Used by: `TBD_ObjectivesComponent`, which pushes the objective board; `TBD_TaskStateMachine`,
  which pushes task changes, clears the markers when a world starts and runs the client request
  tick; `apps/mod/tbd-framework/UI/layouts/Hud/TBD_ObjectiveHud.layout`, which attaches
  `TBD_ObjectiveHud` by class.
- Rules: a client paints only what the server sent and computes nothing from the mission, which
  it does not hold; an RPC carries at most eight parameters, the `Rpc()` limit; the HUD layout is
  named once, in `TBD_UILayouts`; lines added to a script stay ASCII, and
  `cargo xtask mod compile` checks that the scripts compile.

## Related documentation

- [Objective capture HUD specification](/documentation_v2/mod/tbd-framework/UI/objective_capture_hud/objective_capture_hud_specification.md)
  — the design of the objective board and capture bar
- [Tactical marker palette specification](/documentation_v2/mod/tbd-framework/UI/tactical_marker_palette/tactical_marker_palette_specification.md)
  — the marker icons the task markers draw from
