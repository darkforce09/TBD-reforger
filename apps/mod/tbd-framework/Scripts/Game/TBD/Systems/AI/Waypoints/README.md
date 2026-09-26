# Waypoint runtime

Puts the unclaimed seats of each waypointed group under AI control when the round goes live and
issues the group's authored waypoints in order. It runs on the server once a second.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Systems/AI/Waypoints/
├── TBD_WaypointFactory.c     TBD_WaypointFactory: spawns the waypoint prefabs, applies completion and speed
├── TBD_WaypointRuntime.c     TBD_WaypointRuntime: the waypoint pass, the AI spawn gate, arming squads
└── TBD_WaypointWireStruct.c  the orbat.*.groups[].waypoints wire structs of the waypoint pass
```

## How it works

`TBD_WaypointRuntime.Tick` parses `orbat.*.groups[].waypoints` once per mission id into
`TBD_WaypointDocStruct` and keeps each group with at least one waypoint as a squad. Once the stage
is `LIVE` and the slot bodies exist, it forms one `SCR_AIGroup` (`Group_Base.et`, through
`TBD_AIGroupFactory`) from each squad's unclaimed slot bodies, gives it the first member's faction,
and hands it to `TBD_WaypointFactory.IssueWaypoints`. Later unclaimed bodies of an armed squad,
such as a respawned AI seat, join the same group. At spawn, `TBD_SlotBodyMaterializer` asks
`ShouldEnableAIAtSpawn` whether a body keeps its AI, which holds only for a waypointed group's
seat once the round is live; every other body stays parked.

`TBD_WaypointFactory` spawns the waypoints in document order, with a completion radius from
`radiusM`, a completion type from the type and `behaviour`, and a waypoint-origin speed from
`speedMode`, else from `behaviour`:

| Waypoint `type` | Prefab |
|---|---|
| `move` | `AIWaypoint_Move.et` (also an unknown type) |
| `attack` | `AIWaypoint_Attack.et` |
| `defend`, `hold`, `sentry` | `AIWaypoint_Defend.et` |
| `patrol` | `AIWaypoint_Patrol.et` |
| `seek_and_destroy` | `AIWaypoint_SearchAndDestroy.et` |
| `get_in`, `get_out` | `AIWaypoint_GetIn.et`, `AIWaypoint_GetOut.et`, placed on and bound to the roster vehicle of `vehicleUid` (found with `TBD_EntityQuery.FirstVehicleNear`), else boarding by proximity |
| `cycle` | `AIWaypoint_Cycle.et`, which reruns the other waypoints without end |

## Authority

- Server: everything; `Tick` runs from `TBD_RuntimeHeartbeat`'s server tick and the spawning and
  commanding methods carry `@authority server`.
- Client: nothing.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_MissionJsonPass`, `TBD_MissionLoader` (mission id, slots, vehicles),
  `TBD_FrameworkManager`, `TBD_SpawnManager`, `TBD_AIGroupFactory`, `TBD_AIWireEnums`,
  `TBD_EntityQuery`, `TBD_Log`, `TBD_AnnounceOnce`; `$defs/waypoint` in
  `contracts_v2/definitions/mission.schema.json`.
- Used by: `TBD_RuntimeHeartbeat` (`Clear`, `Tick`, `TICK_MS`); `TBD_SlotBodyMaterializer`
  (`ShouldEnableAIAtSpawn`); `TBD_GroupState` finds the groups armed here.
- Rules: player-claimed seats never join a group; groups without waypoints are never enabled;
  waypoints issue in document order; the prefab ResourceNames are the ScenarioFramework defaults;
  `cargo xtask mod compile` checks that the scripts compile, and a group walking its path is
  checked by hand on a dedicated server.
