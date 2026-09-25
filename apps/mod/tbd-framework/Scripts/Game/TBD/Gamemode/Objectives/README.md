# Mission objectives and tasks

Runs a [mission](/documentation_v2/glossary.md#mission)'s objectives on the server while the round
is live: capture, hold and destroy objectives on the mission's zones, with per-side task text, the
objective end triggers the round can end on, and the mission's `tasks[]` that follow its editor
triggers.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Objectives/
├── TBD_Objective.c            TBD_Objective: one objective's rules, progress, owner and board text
├── TBD_ObjectiveRegistry.c    builds objectives from zones and objectives[]; answers the end triggers
├── TBD_ObjectiveRules.c       zoneRules objective keys, read by a second JSON pass per objective zone
├── TBD_ObjectivesComponent.c  game mode component: the 1 Hz live tick, presence, progress, HUD push
└── TBD_TaskStateMachine.c     tasks[]: assigned, succeeded or failed from triggers and schedules
```

## How it works

### Objectives

`TBD_ObjectiveRegistry.Build()` runs once per world, after the mission has loaded and validated.
It takes every prepared zone of type `objective_capture`, `objective_destroy` or
`objective_hold_until` from `TBD_ZoneRegistry` and makes a `TBD_Objective` of it, with the rules
`TBD_ObjectiveRulesReader` reads for that zone and the height, count and starting-owner rules
`TBD_ZoneVolume` reads. `TBD_ObjectiveEntityReader`, in the same file as the registry, joins each
top-level `objectives[]` row onto its zone by `zoneId`: its label and per-side `framing` give each
viewer attacker or defender text, and where a row and its zone disagree the zone wins and the load
log says so. `lock` and `autoLose` are parsed, validated and reported, and change nothing.

`TBD_ObjectivesComponent`, a `SCR_BaseGameModeComponent` on
`apps/mod/tbd-framework/Prefabs/Systems/TBD_GameMode.et`, ticks once a second on the server and
acts only while the stage is `LIVE`:

```text
tick ──▶ Build (until the mission is ready) ──▶ stage LIVE? ──no──▶ hide every HUD
                                                   │ yes
   first LIVE tick: arm destroy targets, seed hold announcements
                                                   ▼
   sample presence ──▶ advance capture | hold | destroy ──▶ chat on completion, HUD on change
```

- Presence counts each connected player with a living body, a life left and a side, at the body's
  origin in the zone's 2D footprint.
- Capture: uninterrupted presence for `captureSeconds` (default 120) takes a neutral objective,
  `neutralizeSeconds` tears a held one back; an enemy inside pauses it when `contestable`, and with
  nobody inside partial progress holds (`onEmpty: "hold"`, the default) or decays.
- Hold: the zone's side holds for `holdSeconds`, paused or reset by an enemy and, with
  `requireHolderPresent`, only while a friendly is inside.
- Destroy: on the first live tick the objective resolves `targetAlias` through `TBD_Registry` and
  queries its zone for that prefab; it completes when `targetCount` (all, when absent) of them are
  destroyed, and goes inert, with the reason logged, when nothing is found.
- Progress survives a stage change away from `LIVE` and back; only a new world clears it.

Each tick sends a player the `TBD_ObjectiveHud` board and capture bar only when that player's
rendered snapshot differs from the last one sent, and puts only `CAPTURED`, `DESTROYED` and `HELD`
lines in chat. `TBD_ObjectiveRegistry.EvaluateEndTriggers` answers `all_objectives_captured`,
`objective_destroyed` and `hold_expired`, each only when `winConditions.endOn` declares it, with
the winning side; `TBD_FrameworkManager` asks it every 2 s and ends the round.

### Tasks

`TBD_TaskStateMachine` reads `tasks[]` with its own `JsonLoadContext` pass. A task starts
`assigned` and moves once, to `succeeded` when its `triggerId` names an editor trigger that fired,
or to `failed` when that trigger is inert or missing, or when its `schedule` window closes while it
is still assigned. A `modded class SCR_BaseGameMode` arms a self-re-arming 1 s tick in a framework
world: on the server it runs the machine and, on a change, pushes the assigned tasks that have a
position to every player through `TBD_TaskHud`; on a client it asks for the snapshot each second.

## Authority

- Server: everything here. `TBD_ObjectivesComponent.OnPostInit` arms its tick only off
  `RplMode.Client` (`@authority server`), and the task tick is armed only on the server; clients
  hold no mission document, so the registry refuses to build there.
- Client: the task HUD request tick, armed by the task machine's `OnGameStart` on
  `RplMode.Client`; the HUD itself lives in `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Hud/`.
- Owner: each player receives only the board and capture bar built for their own side.
- RPCs: none declared here; the objective board goes out through
  `SCR_PlayerController.TBD_PushObjectiveHud` and the tasks through `TBD_PushTaskHud`, whose
  Reliable Server and Owner RPCs `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Hud/` declares.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_ZoneRegistry`, `TBD_Zone`, `TBD_ZoneVolume` and `TBD_TriggerRuntime` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Zones/`; `TBD_MissionLoader` (the raw JSON,
  factions and `HasEndTrigger`) and `TBD_SpawnManager` (a player's
  [slot](/documentation_v2/glossary.md#slot), side and life) under
  `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/`; `TBD_FrameworkManager` (the stage);
  `TBD_Registry` and `TBD_Log` in `apps/mod/tbd-framework/Scripts/Game/TBD/Core/`;
  `TBD_ObjectiveHud` and `TBD_TaskHud` in `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Hud/`; the
  `zone`, `zoneRules`, `objective`, `task` and `winConditions` definitions in
  `contracts_v2/definitions/mission.schema.json`.
- Used by: `TBD_FrameworkManager`, which calls `EvaluateEndTriggers` to end the round;
  `TBD_TriggerRuntime` (the `objective_complete` condition and `set_objective` effect) and
  `TBD_ZoneVolume`; `TBD_MissionValidator` and `TBD_MissionLoader`, which read the objective
  vocabulary; the HUD scripts in `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Hud/`;
  `TBD_AudioEmitter`, which follows task states; and
  `apps/mod/tbd-framework/Prefabs/Systems/TBD_GameMode.et`, which attaches
  `TBD_ObjectivesComponent`.
- Rules: objectives advance only while `LIVE`; containment comes only from `TBD_Zone`, never a
  second test here; the component's `OnDelete` clears the registry, the rules reader and the
  entity reader, and the task machine clears in `OnGameStart`, because statics outlive a world; a
  nested JSON block's presence is tested with a sentinel, never a null test; lines added to a
  script stay ASCII, and `cargo xtask mod compile` checks that the scripts compile, while capture,
  hold and destroy behaviour is checked in a round with players.

## Related documentation

- [Objective capture HUD specification](/documentation_v2/mod/tbd-framework/UI/objective_capture_hud/objective_capture_hud_specification.md)
  — the design of the objective board and capture bar
