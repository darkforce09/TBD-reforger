# Objectives engine

The machinery every [mission](/documentation/glossary/g_to_m.md#mission) objective and task shares,
run on the server while the round is live: the objective record, the registry that builds the
objectives from the mission's zones and `objectives[]` and answers the objective end triggers, the
game mode component that advances them once a second, and the task machine that follows the
mission's `tasks[]`.

## Contents

```text
mod/tbd-framework/Scripts/Game/TBD/Gamemode/Objectives/Engine/
├── Model/     the prepared objective record, its kind, empty-zone and viewer-role enums, board text
├── Registry/  builds objectives from zones and objectives[], answers the end triggers
├── Runtime/   the game mode component: the 1 Hz tick, presence, progress, chat and HUD delivery
└── Tasks/     tasks[]: assigned, succeeded or failed from triggers and schedule windows
```

## How it works

The four parts depend in one direction: `Model/` depends on none of the others, `Registry/` builds
`Model/` records, `Runtime/` drives `Registry/` and advances those records, and `Tasks/` stands
apart, sharing no class with the other three.

```text
Model/     TBD_Objective, its enums, TBD_ObjectiveText  ◀── built, read and advanced by the two below
Registry/  zones + objectives[] ──▶ TBD_Objective list, end triggers
Runtime/   1 Hz tick ──▶ TBD_ObjectiveRegistry.Build ──▶ each kind behaviour advances its TBD_Objective ──▶ chat, HUD
Tasks/     tasks[] ──▶ TBD_Task states, ticked by TBD_RuntimeHeartbeat
```

### Objectives

[`Registry/`](Registry/README.md) builds the objectives once per world, after the mission has
loaded and validated: `TBD_ObjectiveRegistry.Build()` makes a `TBD_Objective`
([`Model/`](Model/README.md)) of every prepared zone in `TBD_ZoneRegistry` whose type a capture,
destroy or hold kind behaviour in
[`Types/`](/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Objectives/Types/README.md) claims,
each run by that behaviour, with the rules
`TBD_ObjectiveRulesReader` reads for that zone and the height, count and starting-owner rules
`TBD_ZoneVolume` reads. `TBD_ObjectiveEntityReader` joins each top-level `objectives[]` row onto
its zone by `zoneId`: its label and per-side `framing` give each viewer attacker or defender text,
and where a row and its zone disagree the zone wins and the load log says so. `lock` and
`autoLose` are parsed, validated and reported, and change nothing.

[`Runtime/`](Runtime/README.md) holds `TBD_ObjectivesComponent`, a `SCR_BaseGameModeComponent` on
`mod/tbd-framework/Prefabs/Systems/TBD_GameMode.et`, which ticks once a second on the server
and acts only while the stage is `LIVE`:

```text
tick ──▶ Build (until the mission is ready) ──▶ stage LIVE? ──no──▶ hide every HUD
                                                   │ yes
   first LIVE tick: each behaviour's OnEnterLive (arm destroy targets, seed hold announcements)
                                                   ▼
   sample presence ──▶ behaviour Advance per objective ──▶ chat on completion, HUD on change
```

- Presence counts each connected player with a living body, a life left and a side, at the body's
  origin in the zone's 2D footprint and height band.
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
lines in chat. `TBD_ObjectiveRegistry.EvaluateEndTriggers` answers each kind behaviour's end
trigger, each only when `winConditions.endOn` declares it, with the winning side;
`TBD_FactionElimination` asks it every 2 s and ends the round.

### Tasks

[`Tasks/`](Tasks/README.md): `TBD_TaskStateMachine` reads `tasks[]` with its own typed pass. A
task starts `assigned` and moves once, to `succeeded` when its `triggerId` names an editor trigger
that fired, or to `failed` when that trigger is inert or missing, or when its `schedule` window
closes while it is still assigned.
[`TBD_RuntimeHeartbeat`](/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Orchestrator/Heartbeat/README.md)
ticks it each second in a framework world: on the server it runs the machine and, on a change,
pushes the assigned tasks that have a position to every player through `TBD_TaskHud`; on a client
it asks for the snapshot each second.

## Authority

- Server: everything here. `TBD_ObjectivesComponent.OnPostInit` arms its tick only off a client
  (`@authority server`), and the task tick is armed only on the server; clients hold no mission
  document, so no reader or registry builds there.
- Client: the task HUD request tick, armed by `TBD_RuntimeHeartbeat` on a client; the HUD itself
  lives in `mod/tbd-framework/Scripts/Game/TBD/UI/Hud/`.
- Owner: each player receives only the board and capture bar built for their own side.
- RPCs: none declared here; the objective board goes out through
  `SCR_PlayerController.TBD_PushObjectiveHud` and the tasks through `TBD_PushTaskHud`, whose
  Reliable Server and Owner RPCs `mod/tbd-framework/Scripts/Game/TBD/UI/Hud/` declares.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_ZoneRegistry`, `TBD_Zone`, `TBD_ZoneVolume` and `TBD_TriggerRuntime` in
  `mod/tbd-framework/Scripts/Game/TBD/Systems/Zones/`; `TBD_MissionLoader` (the raw JSON,
  factions, entities and `HasEndTrigger`), `TBD_MissionJsonPass` and `TBD_MissionVariants` under
  `mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/`; `TBD_SpawnManager` (a player's
  [slot](/documentation/glossary/n_to_z.md#slot), side and life); `TBD_FrameworkManager` (the
  stage); `TBD_Registry`, `TBD_Log`, `TBD_AnnounceOnce`, `TBD_EntityQuery`, `TBD_CharacterUtil`,
  `TBD_PlayerFaction`, `TBD_DeclaredFactions`, `TBD_PlayerChat` and `TBD_Rounding` in
  `mod/tbd-framework/Scripts/Game/TBD/Core/`; `TBD_ObjectiveHud` and `TBD_TaskHud` in
  `mod/tbd-framework/Scripts/Game/TBD/UI/Hud/`; the `zone`, `zoneRules`, `objective`, `task`
  and `winConditions` definitions in `contracts/definitions/mission.schema.json`; the kind
  behaviours (`TBD_ObjectiveKindBehaviour` and its subclasses) in
  `mod/tbd-framework/Scripts/Game/TBD/Gamemode/Objectives/Types/`, which `Runtime/` calls to
  advance, prepare and render each objective and `Model/` calls for the status text.
- Used by: `TBD_FactionElimination` in `mod/tbd-framework/Scripts/Game/TBD/Gamemode/Orchestrator/Stage/`,
  which calls `EvaluateEndTriggers` to end the round, and `TBD_EndBanner`, which infers an admin end;
  `TBD_TriggerRuntime` (the `objective_complete` condition and `set_objective` effect) and
  `TBD_ZoneVolume`; the HUD scripts in
  `mod/tbd-framework/Scripts/Game/TBD/UI/Hud/`; `TBD_AudioEmitter`, which follows task states;
  `TBD_RuntimeHeartbeat`, which ticks and clears the task machine; and
  `mod/tbd-framework/Prefabs/Systems/TBD_GameMode.et`, which attaches
  `TBD_ObjectivesComponent`.
- Rules: objectives advance only while `LIVE`; containment comes only from `TBD_Zone`, never a
  second test here; the component's `OnDelete` clears the registry, its readers and the shared
  kind behaviours, and
  `TBD_RuntimeHeartbeat` clears the task machine, because statics outlive a world; a nested JSON
  block's presence is tested with a sentinel, never a null test; sources stay ASCII, and
  `cargo xtask mod compile` checks that the scripts compile, while capture, hold and destroy
  behaviour is checked in a round with players.

## Related documentation

- [Objective capture HUD specification](/documentation/mod/tbd-framework/UI/objective_capture_hud/objective_capture_hud_specification.md)
  — the objective board and capture bar as built, their delivery and design target
