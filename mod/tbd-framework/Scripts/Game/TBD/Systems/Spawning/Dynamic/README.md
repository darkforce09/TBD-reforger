# AI spawn modules

Runs the mission's authored `spawnModules[]` during the live round: restocking modules spawn groups
on an interval up to a living cap, garrisons spawn once and hold, and every spawned group is
deleted when the round ends.

## Contents

```text
mod/tbd-framework/Scripts/Game/TBD/Systems/Spawning/Dynamic/
├── TBD_DynamicSpawner.c      prepares the modules per mission and ticks them during LIVE; cleanup
├── TBD_DynamicSpawnVolley.c  one module's tick: trigger gate, interval, living cap, origin, volley
└── TBD_SpawnModuleStruct.c   a spawnModules[] row and the document root of that pass
```

## How it works

[`TBD_RuntimeHeartbeat`](../../../Gamemode/Orchestrator/Heartbeat/README.md) calls
`TBD_DynamicSpawner.Tick` every `TICK_MS` (1 s) and `Clear` at a world start. `Build` reads
`spawnModules[]` once per mission id through `TBD_MissionJsonPass` and prepares each valid row
(an id, a known kind, a group template, a positive count, and x and z or a zone, never both). During
`LIVE`, `TBD_DynamicSpawnVolley.TickModule` prunes dead groups, waits for the module's trigger
(`TBD_TriggerRuntime.HasFired`), and spawns up to `count` groups below `maxAlive` (at most 32) at the
module's point or zone centre (`TBD_ZoneRegistry.FindById`) through `TBD_AIGroupFactory`, with the
faction's engine key. At `END` or `DEBRIEF` every spawned group is deleted once.

## Authority

- Server: everything (`@authority server` on `Tick` and the volley); the mission JSON is held on
  the server only.
- Client: nothing.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none; the groups replicate as world entities.

## Boundaries

- Depends on: `TBD_MissionLoader`, `TBD_MissionJsonPass`, `TBD_FrameworkManager`,
  `TBD_ZoneRegistry`, `TBD_TriggerRuntime`, `TBD_AIGroupFactory`, `TBD_AnnounceOnce`, `TBD_Log`,
  and `TBD_SlotBodyMaterializer.EngineFactionKey`.
- Used by: `TBD_RuntimeHeartbeat` in `mod/tbd-framework/Scripts/Game/TBD/Gamemode/Orchestrator/Heartbeat/`.
- Rules: `Tick` keeps its static signature; `spawnModules` presence is a count; no module keeps more
  than 32 living groups.
