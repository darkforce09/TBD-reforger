# World queries

Synchronous box queries over the game world, behind calls that return their answer.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Core/World/
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

- Depends on: `TBD_Zone` and `TBD_ZoneVolume` in `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Zones/`; the engine's
  `BaseWorld`, `Vehicle` and `ChimeraCharacter`.
- Used by: none at present; it replaces the `s_Query*` scratch and `OnQuery*` callbacks in
  `TBD_VehicleState`, `TBD_SpawnManager`, `TBD_WaypointRuntime`, `TBD_MissionVehicleStruct`,
  `TBD_ObjectiveRegistry` and `TBD_TriggerRuntime`.
- Rules: queries are synchronous and never nest; the scratch is empty between calls; lines added
  stay ASCII; `cargo xtask mod compile` checks that the scripts compile.

## Related documentation

- [Framework core utilities](/apps/mod/tbd-framework/Scripts/Game/TBD/Core/README.md) — the rest of the core
