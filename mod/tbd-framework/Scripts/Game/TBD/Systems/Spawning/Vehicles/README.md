# Roster vehicle defaults

Default cargo and full fuel for the vehicles a mission's `vehicles[]` roster places, applied once
when the slot lineup is materialized.

## Contents

```text
mod/tbd-framework/Scripts/Game/TBD/Systems/Spawning/Vehicles/
├── TBD_VehicleSpawnDefaults.c  class default cargo or authored inventory; full fuel when none is authored
└── TBD_VehicleSpawnStruct.c    the vehicles[] rows as this pass reads them, with inventory rows
```

## How it works

`TBD_SlotBodyMaterializer.MaterializeSlotBodies` calls `ApplyCargo` before
`TBD_VehicleState.ApplySpawned`, so authored ammo scales the magazines just inserted, and
`ApplyDefaultFuel` after it, so authored fuel wins. Each pass reads `vehicles[]` from the held
mission JSON (`TBD_MissionJsonPass`) and finds each placed vehicle within 3 m of its roster position
(`TBD_EntityQuery.FirstVehicleNear`). A row with inventory gets exactly that; otherwise the alias's
coarse class (helo, apc, truck, jeep or default) picks the cargo. Items that do not fit are deleted;
a missing prefab is warned about once.

## Authority

- Server: everything (`@authority server` on both entry points); the mission JSON is held on the
  server only.
- Client: nothing.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_MissionJsonPass`, `TBD_EntityQuery`, `TBD_VehicleState`, and the engine's
  `SCR_InventoryStorageManagerComponent`.
- Used by: `TBD_SlotBodyMaterializer` in `Slots/`.
- Rules: an omitted `fuel` differs from an authored 0; inventory presence is a count, never a null
  test; a vehicle not found is logged and left alone.
