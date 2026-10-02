# AI group runtime

Moves the AI groups a [mission](/documentation/glossary/g_to_m.md#mission) authors: reads each group's
waypoints and AI defaults from the loaded mission, puts the unclaimed seats of waypointed groups
under AI control when the round goes live, and issues their waypoints in order.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Systems/AI/
├── GroupState/            group AI defaults: combat mode, formation, speed and behaviour
├── Waypoints/             waypoint chains: the AI spawn gate, arming groups, issuing waypoints
├── TBD_AIGroupFactory.c   spawns an empty SCR_AIGroup from a prefab and adopts a member's faction
└── TBD_AIWireEnums.c      the speedMode and behaviour tokens and the movement speed they select
```

## How it works

Both runtimes follow one pattern.
[`TBD_RuntimeHeartbeat`](../../Gamemode/Orchestrator/Heartbeat/README.md) clears them when a game
mode starts and ticks each once a second on the server, only in a framework world
(`TBD_FrameworkManager.IsFrameworkWorld()`). The tick parses the loaded mission once per mission id
with its own pass through `TBD_MissionJsonPass.LoadRoot`, into wire structs that declare only the
keys it reads, because Enfusion maps JSON keys onto named class fields only:
`orbat.*.groups[].waypoints` for [Waypoints](Waypoints/README.md), and each group's `combatMode`,
`behaviour`, `formation` and `speedMode` for [GroupState](GroupState/README.md). Both act only once
the stage is `LIVE` and `TBD_SpawnManager` has materialized the slot bodies. Only groups the
waypoint runtime armed have a live group, so the defaults of a group without waypoints take no
effect.

`TBD_AIGroupFactory.SpawnGroup(prefab, origin, failure)` spawns an empty `SCR_AIGroup`, deletes a
spawned entity that is not a group, and reports `PREFAB_UNLOADABLE` or `NOT_A_GROUP`;
`AdoptMemberFaction` gives the group a member's affiliated or default faction.
`TBD_AIWireEnums` holds the `speedMode` and `behaviour` tokens: `SpeedFromSpeedMode` maps
limited, normal and full to walk, run and sprint, `SpeedCeilingFromBehaviour` maps careless, safe
and stealth to walk, aware to run and combat to sprint, and `SpeedFromWire` lets an explicit
`speedMode` win.

## Authority

- Server: everything. The runtimes tick from `TBD_RuntimeHeartbeat`'s server tick, and
  `ShouldEnableAIAtSpawn` is asked by `TBD_SlotBodyMaterializer` on the server; the methods that
  spawn or command entities carry `@authority server`.
- Client: nothing.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_MissionLoader` (the mission id, slots and vehicles), `TBD_MissionJsonPass` (the
  raw mission JSON), `TBD_FrameworkManager` (the game stage), `TBD_SpawnManager` (slot bodies and
  player claims), `TBD_EntityQuery`, `TBD_Log` and `TBD_AnnounceOnce`, and the engine's AI classes
  (`SCR_AIGroup`, `AIWaypoint`, `AIWaypointCycle`, `SCR_EntityWaypoint`, the ScenarioFramework
  waypoint prefabs); the `waypoint` and `group` definitions in
  `contracts/definitions/mission.schema.json`.
- Used by: `TBD_RuntimeHeartbeat` in `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Orchestrator/Heartbeat/`
  (`Clear`, `Tick`, `TICK_MS`); `TBD_SlotBodyMaterializer` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Spawning/Slots/`, which calls
  `TBD_WaypointRuntime.ShouldEnableAIAtSpawn`; `TBD_DynamicSpawnVolley` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Spawning/Dynamic/` (`TBD_AIGroupFactory`).
- Rules: the scripts run on the server only; each reader declares its own wire structs beside the
  code that interprets them instead of adding fields to the mission loader's structs; an absent
  string attribute is tested with `IsEmpty()`, because `JsonLoadContext` allocates a nested class
  field even when its key is absent; lines added stay ASCII; `cargo xtask mod compile` checks that
  the scripts compile, and whether a group walks its path on a dedicated server is checked by hand.
