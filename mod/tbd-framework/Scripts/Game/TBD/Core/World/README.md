# World queries

Synchronous box queries over the game world, behind calls that return their answer.

## Contents

```text
mod/tbd-framework/Scripts/Game/TBD/Core/World/
└── TBD_EntityQuery.c  the first vehicle near a point, and every entity of a prefab in a box or zone
```

## How it works

`BaseWorld.QueryEntitiesByAABB` reports each entity to a plain callback function, so
`TBD_EntityQuery` keeps static scratch that its callbacks write into and that each call sets and
clears. `FirstVehicleNear(x, z, halfWidthM, halfHeightM)` returns the first `Vehicle` that is
not a character in the box around a point. `CollectPrefabInBox(prefab, mins, maxs, outHits)`
appends every entity spawned from a prefab. `CollectPrefabInZone(prefab, zone, halfHeightM,
checkHeightBand, outHits)` queries the zone's XZ bounds and keeps the origins inside its shape,
through `TBD_ZoneVolume.ContainsOrigin` when `checkHeightBand` is true, else
`TBD_Zone.Contains`.

## Authority

- Server: nothing of its own; its callers query the authoritative world.
- Client: nothing.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_Zone` and `TBD_ZoneVolume` in `mod/tbd-framework/Scripts/Game/TBD/Systems/Zones/`; the engine's
  `BaseWorld`, `Vehicle` and `ChimeraCharacter`.
- Used by: `TBD_ObjectiveDestroyTargets` in `mod/tbd-framework/Scripts/Game/TBD/Gamemode/Objectives/Types/Destroy/`;
  `TBD_VehicleState` and `TBD_MissionVehicleRoster` in `mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Data/`;
  `TBD_VehicleSpawnDefaults`, `TBD_WaypointFactory` and `TBD_TriggerWorldEffects` under
  `mod/tbd-framework/Scripts/Game/TBD/Systems/`.
- Rules: queries are synchronous and never nest; the scratch is empty between calls; lines added
  stay ASCII; `cargo xtask mod compile` checks that the scripts compile.

## Related documentation

- [Framework core utilities](/mod/tbd-framework/Scripts/Game/TBD/Core/README.md) — the rest of the core
