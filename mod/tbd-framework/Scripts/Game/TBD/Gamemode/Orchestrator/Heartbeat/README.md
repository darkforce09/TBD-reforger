# Runtime heartbeat

The one game-mode loop that clears the mission runtimes when a world starts and ticks them each
beat in a fixed, explicit order.

## Contents

```text
mod/tbd-framework/Scripts/Game/TBD/Gamemode/Orchestrator/Heartbeat/
└── TBD_RuntimeHeartbeat.c  TBD_RuntimeHeartbeat: clear and tick lists; the modded SCR_BaseGameMode that arms the loop
```

## How it works

A `modded class SCR_BaseGameMode` overrides `OnGameStart`. It calls `super.OnGameStart()`, then
`TBD_RuntimeHeartbeat.ClearRuntimes()` on every machine and in every world, because statics
outlive a world inside one process. In a framework world
(`TBD_FrameworkManager.IsFrameworkWorld()`) it arms one self-re-arming one-shot `CallLater` of
`BEAT_MS` (1000 ms) per game mode instance and logs the order it will run; on the server it also
loads the durable telemetry queue (`TBD_TelemetryQueue.EnsureLoaded`), which reads
`$profile:TBD/Telemetry/` once per process:

```text
[TBD][Heartbeat] armed side=server beatMs=1000 order=WinCondition/2,Task,GroupState,Waypoint,Audio,Weather,DynamicSpawner,Trigger,MatchEvents/10,TelemetryDelivery
```

Each beat counts up from 1 and calls `TBD_RuntimeHeartbeat.Beat(beat)`. A runtime ticks on a beat
when `beat * BEAT_MS` is a multiple of its own `TICK_MS`, so every runtime keeps its period:

| Order | Runtime | Period | Side |
| --- | --- | --- | --- |
| 1 | `TBD_WinConditionEvaluator.Tick` | 2000 ms, until `HasEnded()` | server |
| 2 | `TBD_TaskStateMachine.Tick` | 1000 ms | server |
| 3 | `TBD_GroupState.Tick` | 1000 ms | server |
| 4 | `TBD_WaypointRuntime.Tick` | 1000 ms | server |
| 5 | `TBD_AudioEmitter.Tick` | 1000 ms | server |
| 6 | `TBD_WeatherRuntime.Tick` | 1000 ms | server |
| 7 | `TBD_DynamicSpawner.Tick` | 1000 ms | server |
| 8 | `TBD_TriggerRuntime.Tick` | 1000 ms | server |
| 9 | `TBD_MatchEventRecorder.Tick` | 10000 ms; queues the buffered detailed match events | server |
| 10 | `TBD_TelemetryDelivery.Tick` | 1000 ms; its own backoff decides when a request is sent | server |
| 1 | `TBD_TaskHud.RequestLocal` | 1000 ms | remote client |

`ClearRuntimes` clears `TBD_TaskStateMachine`, `TBD_TaskHud`, `TBD_WinConditionEvaluator`,
`TBD_GroupState`, `TBD_WaypointRuntime`, `TBD_AudioEmitter`, `TBD_WeatherRuntime`,
`TBD_DynamicSpawner`, `TBD_TriggerRuntime` and `TBD_MatchEventRecorder`, in that order; the
recorder first queues what the previous world left buffered.

The loop re-arms only while its game mode is still `GetGame().GetGameMode()`:
`ScriptCallQueue.Remove` cancels by function and a game mode has no teardown hook, so a stale
timer from a replaced world runs once more and stops. The armed flag keeps a second
`OnGameStart` on the same instance from arming a second loop.

## Authority

- Server: every runtime in the order above.
- Client: only `TBD_TaskHud.RequestLocal`, which asks the server for the task snapshot.
- Owner: nothing.
- RPCs: none of its own.
- Replicated properties: none.

## Boundaries

- Depends on: the static `Clear` and `Tick` of the runtimes above, `TBD_TelemetryQueue` and
  `TBD_TelemetryDelivery` in `mod/tbd-framework/Scripts/Game/TBD/API/MatchTelemetry/`,
  `TBD_MatchEventRecorder` in `mod/tbd-framework/Scripts/Game/TBD/Systems/MatchEvents/`, `TBD_Authority`,
  `TBD_FrameworkManager.IsFrameworkWorld()` and `TBD_Log`.
- Used by: the engine, through `SCR_BaseGameMode.OnGameStart`.
- Rules: a runtime's `TICK_MS` is a whole multiple of `BEAT_MS`; adding a runtime means adding it
  to `ClearRuntimes`, `TickServer` or `TickClient`, and to `SERVER_ORDER` or `CLIENT_ORDER`;
  `cargo xtask mod world-boot` shows the arm-time order line.

## Related documentation

- [Round orchestrator](/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Orchestrator/README.md) — the framework manager and `IsFrameworkWorld()`
- [Mod design](/documentation/mod/tbd-framework/mod_design.md) — the event loop
