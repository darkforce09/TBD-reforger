**Status:** live

# README template: mod scripts

**When to use:** a folder at or under an Enfusion addon's `Scripts/`, such as
`apps/mod/tbd-framework/Scripts/Game/TBD/Systems/AI/` or a Workbench plugin folder under
`Scripts/WorkbenchGame/`. The [README standard](/documentation_v2/standards/readme_standard.md)
defines every rule this template follows; the mod scripts kind adds Authority.

## Skeleton

Copy the block and replace every `<…>` placeholder; each one says what goes there. Authority
matches the `@authority`, `@rpc` and `@replicated` tags in the scripts; where the scripts carry
none, it says where the code runs as the code decides it, and names no RPC or property that does
not exist.

````markdown
# <What the scripts do, in plain words: no path, no backticks>

<One to three sentences: the gameplay or tooling behaviour the scripts provide, and when it runs.>

## Contents

```text
<repository path of the folder>/
├── <child folder>/  <what it holds: a lowercase phrase, no closing period>
└── <TBD_Class.c>    <the class it declares and what it does>
```

## How it works

<The classes and how they meet: what arms them (a component on a prefab, a modded class, a
Workbench plugin), what they read and write, the order of events, and the engine behaviour they
rely on.>

## Authority

- Server: <what runs only on the server, and the guard that keeps it there>
- Client: <what runs on each client>
- Owner: <what runs only on the owning client>
- RPCs: <each RPC with its reliability and receivers, as its @rpc tag states; or "none">
- Replicated properties: <each property and its replication hook, as its @replicated tag states;
  or "none">

## Boundaries

- Depends on: <the framework classes, engine classes and data the scripts use>
- Used by: <the scripts, prefabs and configs outside the folder that call or attach these classes,
  found with git grep>
- Rules: <the invariants a change must keep, and the gate that checks them>

## Related documentation

- [<document title>](/documentation_v2/mod/<path to the document>) — <what it covers>
````

## Worked sample

Written from `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/AI/`. Its scripts hold no RPC, no
replicated property and no authority tag; the server-only guard is in their code, and Authority
says so. No document covers these scripts, so the sample has no Related documentation. The sample
sits in a fenced block, so no gate reads it as a README; the folder's own README.md is written from
the same code and may differ.

````markdown
# AI group runtime

Moves the AI groups a mission authors: reads each group's waypoints and AI defaults from the loaded
mission, puts the unclaimed seats of waypointed groups under AI control when the round goes live,
and issues their waypoints in order.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Systems/AI/
├── TBD_GroupState.c       group AI defaults: combat mode, formation, speed and behaviour
└── TBD_WaypointRuntime.c  waypoint chains: the AI spawn gate, arming groups, issuing waypoints
```

## How it works

Both files follow one pattern. Each adds a `modded class SCR_BaseGameMode` whose `OnGameStart`
arms a one-second call-queue tick, on the server and only in a framework world
(`TBD_FrameworkManager.IsFrameworkWorld()`). The tick parses the loaded mission once per mission id
with its own `JsonLoadContext` pass over `TBD_MissionLoader.GetRawJson()`, into wire structs that
declare only the keys it reads, because Enfusion maps JSON keys onto named class fields only:
`orbat.*.groups[].waypoints` for `TBD_WaypointRuntime`, and each group's `combatMode`, `behaviour`,
`formation` and `speedMode` for `TBD_GroupState`.

`TBD_WaypointRuntime` waits for the `LIVE` stage and for `TBD_SpawnManager` to materialize the slot
bodies, adds each waypointed group's unclaimed bodies to an `SCR_AIGroup`, and issues the group's
waypoints in document order as ScenarioFramework waypoint prefabs; a `get_in` or `get_out`
waypoint binds the authored vehicle through its `vehicleUid`. At spawn, `TBD_SpawnManager` asks
`TBD_WaypointRuntime.ShouldEnableAIAtSpawn` whether a body keeps its AI, which holds only for a
waypointed group's seat once the round is live. `TBD_GroupState` finds each group's live
`SCR_AIGroup` and applies its defaults through `SCR_AIGroupUtilityComponent`,
`AIFormationComponent` and a movement-speed setting with the `DEFAULT` origin, so a waypoint's own
speed setting wins.

## Authority

- Server: everything. Each tick is armed only when `RplSession.Mode()` is not `RplMode.Client`,
  and every other method runs from a tick or from `TBD_SpawnManager` on the server.
- Client: nothing; `OnGameStart` clears the parsed state and returns.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_MissionLoader` (the raw mission JSON, the mission id, slots and vehicles),
  `TBD_FrameworkManager` (the game stage), `TBD_SpawnManager` (slot bodies and player claims),
  `TBD_Log`, and the engine's AI classes (`SCR_AIGroup`, `AIWaypoint`, `AIWaypointCycle`, the
  ScenarioFramework waypoint prefabs).
- Used by: `TBD_SpawnManager` in `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Spawning/`, which
  calls `TBD_WaypointRuntime.ShouldEnableAIAtSpawn`; the game mode, through the modded
  `SCR_BaseGameMode`.
- Rules: the scripts run on the server only; each reader declares its own wire structs beside the
  code that interprets them instead of adding fields to the mission loader's structs; an absent
  string attribute is tested with `IsEmpty()`, because `JsonLoadContext` allocates a nested class
  field even when its key is absent; sources stay ASCII; `cargo xtask mod compile` checks that they
  compile, and whether a group walks its path on a dedicated server is checked by hand.
````
