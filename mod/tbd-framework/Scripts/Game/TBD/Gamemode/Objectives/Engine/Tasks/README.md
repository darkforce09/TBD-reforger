# Mission tasks

Runs the mission's `tasks[]` on the server: each task starts assigned and moves once, to succeeded
when its linked editor trigger fires or to failed when that trigger is inert or missing or its
schedule window closes, and players see the assigned tasks on the map HUD.

## Contents

```text
mod/tbd-framework/Scripts/Game/TBD/Gamemode/Objectives/Engine/Tasks/
├── TBD_Task.c              TBD_Task: one prepared task, with its state and tier enums
├── TBD_TaskSchedule.c      the mission clock since LIVE and each task's schedule window
├── TBD_TaskStateMachine.c  reads tasks[], applies the transitions, resolves marker positions
└── TBD_TaskStruct.c        the tasks[], schedule and document-root wire structs
```

## How it works

[`TBD_RuntimeHeartbeat`](/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Orchestrator/Heartbeat/README.md)
calls `TBD_TaskStateMachine.Clear` at the start of each world and `Tick` every `TICK_MS` (1000 ms)
on the server. A tick:

1. Rebuilds when the loaded mission's `meta.id` changed, and otherwise waits until a mission is
   loaded. `Build` binds `tasks[]` into `TBD_TaskStruct` through `TBD_MissionJsonPass` and prepares
   each row with an id and a title; a mission with no tasks logs `idle` once.
2. `TBD_TaskSchedule.Evaluate` opens each timed task's window once the mission clock (seconds since
   the stage went LIVE) reaches `startAfterS`, logging `id=<n> t=<s> -> assigned`, and fails a task
   still assigned when `startAfterS + windowS` passes.
3. `SyncFromTriggers`, once `TBD_TriggerRuntime` is built, fails an evaluating task whose
   `triggerId` names no trigger or an inert one, and succeeds it when the trigger fired.
4. `ResolvePositions` places each task with a linked trigger's zone at that zone's centre.
5. When any task changed, `TBD_TaskHud.PushToPlayers` sends the assigned tasks that have a position.

`TryTransition` is the one mutation: only ASSIGNED moves, to SUCCEEDED or FAILED; any other pair is
logged and ignored. Every transition logs `[TBD][Task] id=<n> t=<s> -> <state>`.

## Authority

- Server: the whole machine; `TBD_RuntimeHeartbeat` ticks it only on the server, and a client holds
  no mission text to build from.
- Client: nothing here; the heartbeat's client tick asks `TBD_TaskHud` for the snapshot.
- Owner: nothing.
- RPCs: none declared here; `TBD_TaskHud` in `mod/tbd-framework/Scripts/Game/TBD/UI/Hud/`
  declares the task HUD RPCs.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_MissionLoader` (the mission id), `TBD_MissionJsonPass`, `TBD_TriggerRuntime` and
  `TBD_Zone` (trigger state and zone centres), `TBD_FrameworkManager` (the stage),
  `TBD_TaskHud`, `TBD_AnnounceOnce`, `TBD_Rounding` and `TBD_Log`; the `task` and `taskSchedule`
  definitions in `contracts/definitions/mission.schema.json`.
- Used by: `TBD_RuntimeHeartbeat` in
  `mod/tbd-framework/Scripts/Game/TBD/Gamemode/Orchestrator/Heartbeat/` (`Clear`, `Tick`,
  `TICK_MS`); `TBD_TaskHud` (`GetAll`, `IconFor`, `StateName`, `TBD_Task`, `TBD_ETaskState`);
  `TBD_AudioEmitter` in `mod/tbd-framework/Scripts/Game/TBD/Systems/Audio/`, which follows
  task states.
- Rules: tasks observe triggers and never fire `winConditions.endOn`; a state moves at most once;
  the nested `schedule` is always allocated, so its ABSENT sentinels are the presence test; the
  static `Tick` signature is what the heartbeat calls.
