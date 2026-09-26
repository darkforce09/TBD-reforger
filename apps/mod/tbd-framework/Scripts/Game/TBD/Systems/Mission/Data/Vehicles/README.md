# Mission vehicle roster and vehicle state

The mission-placed vehicles of a loaded [mission](/documentation_v2/glossary/g_to_m.md#mission):
the `vehicles[]` rows with their crew plans, the roster that puts exactly one world vehicle behind
each row and seats its crew, and the reader that applies each vehicle's authored lock, fuel and
ammo. The server runs all of it while a mission's slot bodies materialize.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Data/Vehicles/
├── TBD_MissionVehicleCrewSeating.c  seats each crew slot's body in its compartment and verifies it a second later
├── TBD_MissionVehicleRoster.c       entities[] placement index, the twin claim, fallback spawn and world census
├── TBD_MissionVehicleSeatStruct.c   one seats[] entry: slot uid, crew role and optional station index
├── TBD_MissionVehicleStruct.c       one vehicles[] row: alias, uid, position, heading, faction and seats
└── TBD_VehicleState.c               vehicles[] lock, fuel and ammo, applied to the placed vehicles
```

## How it works

One authored vehicle arrives as two rows: an `entities[]` row that
`TBD_MissionWorldApplier.SpawnMissionEntities` places, and a `vehicles[]` row
(`TBD_MissionVehicleStruct`, bound onto `TBD_MissionDocumentStruct.vehicles`) that adds its
`seats[]`.

1. `TBD_MissionWorldApplier` calls `TBD_MissionVehicleRoster.ResetIndex` before it places
   anything and `RecordEntitySpawn` for each placed row.
2. `TBD_SlotBodyMaterializer` calls `TBD_MissionVehicleRoster.SeatAuthoredCrews` once every slot
   body exists. Each roster row claims its placed twin by `uid`, else by an alias, x and z
   fingerprint, and each twin is claimed once; a row with nothing to claim spawns its own vehicle.
   A world census then counts the entities of that prefab at the row's position and logs an ERROR
   for any count other than 1.
3. `TBD_MissionVehicleCrewSeating.SeatCrew` resolves each seat's `slotId` through
   `TBD_MissionLoader.GetSlotById`, maps its role to a compartment type (driver, pilot and copilot
   to PILOT; commander, gunner and turret to TURRET; cargo to CARGO), picks the station (an
   authored `index` is exact; without one the role's default station, else the next free one), and
   force-teleports the body in with `GetInVehicle`. `VerifySeatedCrews` runs
   `SEAT_VERIFY_DELAY_MS` later and asks the engine which bodies are in the vehicle they were given.
4. `TBD_SlotBodyMaterializer` then calls `TBD_VehicleState.ApplySpawned`, a second
   `JsonLoadContext` pass (`TBD_MissionJsonPass.LoadRoot`) over `vehicles[]` `lock`, `fuel` and
   `ammo`. It finds the vehicle at each row's position with `TBD_EntityQuery.FirstVehicleNear` and
   locks the pilot controls, sets every fuel node, slotted tanks included, and scales turret and
   cargo magazines. `TBD_VehicleSpawnDefaults` calls `TBD_VehicleState.Apply` for its own fuel.

Presence: `seats` is always allocated after a parse, so it is tested with `Count()`; an absent
seat `index` reads `INDEX_ABSENT` and absent `fuel` or `ammo` reads `ABSENT`; `lock` applies only
when true, because an absent bool and an authored false bind the same.

## Authority

- Server: everything. The roster, seating and state readers run from the server's slot-body
  materialization and mission placement.
- Client: nothing.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none; the engine replicates the vehicles, their occupants and their state.

## Boundaries

- Depends on: `TBD_MissionLoader` in `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Loaders/`
  (`GetVehicles`, `GetSlotById`); `TBD_SpawnManager` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Spawning/` (`GetSlotBody`); `TBD_Registry`
  (alias to prefab); `TBD_EntityQuery`; `TBD_MissionJsonPass`; the engine's compartment,
  `VehicleControllerComponent`, `FuelManagerComponent` and weapon and magazine classes; the
  `$defs/vehicle` definition in `contracts_v2/definitions/mission.schema.json`.
- Used by: `TBD_MissionDocumentStruct.vehicles`; `TBD_MissionWorldApplier` (`ResetIndex`,
  `RecordEntitySpawn`); `TBD_SlotBodyMaterializer` (`SeatAuthoredCrews`, `ApplySpawned`);
  `TBD_VehicleSpawnDefaults` (`Apply`, `TBD_VehicleStateWireStruct.ABSENT`).
- Rules: every wire field keeps its JSON key's exact spelling and each struct carries its
  `//! @contract` tag; a seat is reported accepted, never seated, until the engine confirms it; a
  seat that cannot be filled skips only itself and logs why; lines stay ASCII, and
  `cargo xtask mod compile` checks that the scripts compile, while whether a crew sits in its
  vehicle in a round is checked by hand.
