# Round stage helpers

The work the framework manager delegates: carrying the round out of `LOADING`, the checks and the
clock that end a live round, the END banner, and the
[mission](/documentation/glossary/g_to_m.md#mission)'s wind and night-vision rules.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Orchestrator/Stage/
├── TBD_EndBanner.c           TBD_EndBanner: END winner and reason, scoreboard, END and DEBRIEF overlays
├── TBD_FactionElimination.c  TBD_FactionElimination: the 2 s objective-trigger and faction_eliminated check
├── TBD_LoadingGate.c         TBD_LoadingGate: LOADING to LOBBY once the mission, roster and loadouts settle
├── TBD_RoundClock.c          TBD_RoundClock: the authored round clock armed at LIVE
└── TBD_StageEnvironment.c    TBD_StageEnvironment: authored wind direction and the night-vision strip
```

## How it works

Each helper is a `Managed` object owned by `TBD_FrameworkManager`, created in its constructor with
the manager passed in, and cancelled from its `OnDelete` through `CancelCallbacks`.

- `TBD_LoadingGate` polls the mission load once a second. Once it is loaded and valid it puts the
  document into force: `TBD_MissionWorldApplier.Apply` first (the `entities[]` bodies, the
  spectator policy, the authored environment and the gadget-flag spawn hook), then
  `TBD_MissionFlowReport.Apply`, `TBD_StageEnvironment.ApplyAuthoredWeather` (whose wind direction
  overrides the environment's) and the manager's `LatchAuthoredSettings`; it loads the registry,
  materializes the [slot](/documentation/glossary/n_to_z.md#slot) bodies (which claim the placed
  vehicles) and starts the [event](/documentation/glossary/a_to_f.md#event) roster fetch; every
  500 ms it then waits for the roster (force-settled after 2 s) and the loadout settle (up to 24
  polls) and asks for `LOBBY`. The poll runs on the main thread from the call queue, after the
  world has created its entities, and it is the only place the loaded document reaches the world:
  the parse and the platform's answers never spawn or apply anything.
- `TBD_FactionElimination.Arm` runs at `LIVE` when `winConditions.endOn` declares
  `faction_eliminated` or an objective trigger. Every 2 s it ends the round on the first
  `TBD_ObjectiveRegistry.EvaluateEndTriggers` answer, else when at least two sides fielded players
  and at most one still has a living one. `CountSurvivors` is the survivor count the banner and the
  clock also use.
- `TBD_RoundClock.Arm` runs at `LIVE` and counts `flow.timeLimitSeconds` down once a second, only
  when `endOn` also declares `time_limit` (`0` means no limit); it warns in chat at the
  `TBD_ClockText` milestones and ends the round with reason `time_limit`.
- `TBD_EndBanner` holds the reason and winner an end rule named. On the way into `END` it gives
  them to the manager, or infers them: the objective trigger, else `faction_eliminated` when one
  of two or more sides survives, else `admin`. It packs the scoreboard from
  `TBD_DebriefScoreboard.Fill`, whose kills come from `TBD_MatchTelemetryTally` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/MatchEvents/`, and opens the local END and
  DEBRIEF overlays.
- `TBD_StageEnvironment` applies `environment.windDirDeg` through the weather manager and, when
  `settings.nightVision` is false, deletes night-vision gadgets from each spawned body 1.5 s after
  it spawns.

## Authority

- Server: every poll, the end checks, the clock and the night-vision strip
  (`@authority server`); the manager arms them only on the server.
- Client: `TBD_EndBanner.ApplyEndScreens`, called from the manager's local UI hook.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none; the manager holds the replicated results.

## Boundaries

- Depends on: `TBD_FrameworkManager` (the stage, `SetStage`, `EndRound`, the night-vision latch);
  `TBD_MissionFlow` and `TBD_MissionFlowReport` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Orchestrator/Flow/`; `TBD_ObjectiveRegistry`
  under `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Objectives/`; `TBD_MissionLoader`,
  `TBD_MissionWorldApplier`, `TBD_RosterLoader` and `TBD_SpawnManager` under
  `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/`;
  `TBD_EndScreen`, `TBD_DebriefScreen` and `TBD_DebriefScoreboard` under
  `apps/mod/tbd-framework/Scripts/Game/TBD/Session/`.
- Used by: `TBD_FrameworkManager` only.
- Rules: a round ends only through `TBD_FrameworkManager.EndRound` or `SetStage(END)`; every poll
  stops itself when the stage leaves the stage it serves; every `[TBD][Win]` line keeps its prefix;
  lines added to a script stay ASCII, and `cargo xtask mod compile` checks that the scripts compile.
