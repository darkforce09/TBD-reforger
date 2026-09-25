# AI group runtime

Moves the AI groups a [mission](/documentation_v2/glossary.md#mission) authors: reads each group's
waypoints and AI defaults from the loaded mission, puts the unclaimed seats of waypointed groups
under AI control when the round goes live, and issues their waypoints in order.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Systems/AI/
├── TBD_GroupState.c       group AI defaults: combat mode, formation, speed and behaviour
└── TBD_WaypointRuntime.c  waypoint chains: the AI spawn gate, arming groups, issuing waypoints
```

## How it works

Both files follow one pattern. Each adds a `modded class SCR_BaseGameMode` whose `OnGameStart`
arms a one-second self-re-arming call-queue tick, on the server and only in a framework world
(`TBD_FrameworkManager.IsFrameworkWorld()`). The tick parses the loaded mission once per mission id
with its own `JsonLoadContext` pass over `TBD_MissionLoader.GetRawJson()`, into wire structs that
declare only the keys it reads, because Enfusion maps JSON keys onto named class fields only:
`orbat.*.groups[].waypoints` for `TBD_WaypointRuntime`, and each group's `combatMode`, `behaviour`,
`formation` and `speedMode` for `TBD_GroupState`. Both act only once the stage is `LIVE` and
`TBD_SpawnManager` has materialized the slot bodies.

`TBD_WaypointRuntime` adds each waypointed group's unclaimed bodies to a new `SCR_AIGroup`
(`Group_Base.et`) and issues the group's waypoints in document order as ScenarioFramework waypoint
prefabs, with a completion radius from `radiusM` and a speed from `speedMode`, else from
`behaviour`. Later unclaimed bodies of an armed group, such as a respawned AI seat, join the same
group. At spawn, `TBD_SpawnManager` asks `TBD_WaypointRuntime.ShouldEnableAIAtSpawn` whether a
body keeps its AI, which holds only for a waypointed group's seat once the round is live; every
other body stays parked.

| Waypoint `type` | Prefab |
|---|---|
| `move` | `AIWaypoint_Move.et` |
| `attack` | `AIWaypoint_Attack.et` |
| `defend`, `hold`, `sentry` | `AIWaypoint_Defend.et` |
| `patrol` | `AIWaypoint_Patrol.et` |
| `seek_and_destroy` | `AIWaypoint_SearchAndDestroy.et` |
| `get_in`, `get_out` | `AIWaypoint_GetIn.et`, `AIWaypoint_GetOut.et`, bound to the authored vehicle through its `vehicleUid` |
| `cycle` | `AIWaypoint_Cycle.et`, which reruns the other waypoints without end |

`TBD_GroupState` finds each group's live `SCR_AIGroup` through its slot bodies' AI agents and
applies its defaults once: combat mode through `SCR_AIGroupUtilityComponent`, formation through
`AIFormationComponent`, and a movement-speed setting with the `DEFAULT` origin, so a waypoint's own
speed setting wins. Only groups that `TBD_WaypointRuntime` armed have a live group, so the defaults
of a group without waypoints take no effect.

## Authority

- Server: everything. Each tick is armed only when `RplSession.Mode()` is not `RplMode.Client`,
  and every other method runs from a tick or from `TBD_SpawnManager` on the server. The scripts
  carry no `@authority` tag.
- Client: nothing; `OnGameStart` clears the parsed state and returns.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_MissionLoader` (the raw mission JSON, the mission id, slots and vehicles),
  `TBD_FrameworkManager` (the game stage), `TBD_SpawnManager` (slot bodies and player claims),
  `TBD_Log`, and the engine's AI classes (`SCR_AIGroup`, `AIWaypoint`, `AIWaypointCycle`,
  `SCR_EntityWaypoint`, the ScenarioFramework waypoint prefabs); the `waypoint` and `group`
  definitions in `contracts_v2/definitions/mission.schema.json`.
- Used by: `TBD_SpawnManager` in `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Spawning/`, which
  calls `TBD_WaypointRuntime.ShouldEnableAIAtSpawn`; the game mode, through the modded
  `SCR_BaseGameMode`.
- Rules: the scripts run on the server only; each reader declares its own wire structs beside the
  code that interprets them instead of adding fields to the mission loader's structs; an absent
  string attribute is tested with `IsEmpty()`, because `JsonLoadContext` allocates a nested class
  field even when its key is absent; sources stay ASCII; `cargo xtask mod compile` checks that they
  compile, and whether a group walks its path on a dedicated server is checked by hand.
