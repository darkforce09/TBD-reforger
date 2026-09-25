# Mission data structs and state readers

The typed shape of a loaded [mission](/documentation_v2/glossary.md#mission)'s slots and vehicles,
and the readers that apply the per-row states the primary parse does not bind: entity and vehicle
state, player gadget flags and launch parameters. The server fills and applies them while a mission
loads and while its slot bodies materialize.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Data/
├── TBD_EntityState.c           entities[] health, allowDamage, showModel and size, applied to placed bodies
├── TBD_GadgetFlags.c           slots[].gadgets: map, compass, watch, GPS and radio added or removed at spawn
├── TBD_MissionParams.c         missionParams[] rows and the launch value chosen for each symbol
├── TBD_MissionSlotStruct.c     one compiled slot: position, kit, loadout gear and cargo, seat identity
├── TBD_MissionVehicleStruct.c  vehicles[] rows, their crew seats, and the roster that seats crews
└── TBD_VehicleState.c          vehicles[] lock, fuel and ammo, applied to the placed vehicles
```

## How it works

`TBD_MissionSlotStruct` (with `TBD_SlotLoadoutStruct`, `TBD_SlotGearStruct`, `TBD_SlotCargoStruct`)
and `TBD_MissionVehicleStruct` are fields of `TBD_MissionDocumentStruct`, which `TBD_MissionLoader`
binds with `JsonLoadContext` in the primary parse. `JsonLoadContext` binds JSON keys onto class
fields by exact name, so every field name is its JSON key.

The other files each run a second `JsonLoadContext` pass over `TBD_MissionLoader.GetRawJson()`
into wire structs that declare only the keys they read, so the primary structs do not grow:

| Reader | Keys | Armed by | Applies |
|---|---|---|---|
| `TBD_EntityState` | `entities[]` `health`, `allowDamage`, `showModel`, `size`, `stamina` | `TBD_MissionLoader.SpawnMissionEntities` records each placed body (`RecordSpawn`) | `ApplySpawned`: `SetHealthScaled`, `EnableDamageHandling`, the `VISIBLE` flag, `SetScale`; `stamina` is logged and not applied (the engine has no stamina toggle) |
| `TBD_VehicleState` | `vehicles[]` `lock`, `fuel`, `ammo` | the vehicles already in the world | `ApplySpawned`: `LockPilotControls`, fuel nodes (slotted tanks included), turret and cargo magazines |
| `TBD_GadgetFlags` | `slots[].gadgets` as `map<string, bool>` | `Bind()` after a valid parse, which hooks `SCR_BaseGameMode.GetOnPlayerSpawned` | 800 ms after a player spawns (after the loadout's cargo lands), adds or removes each authored gadget |
| `TBD_MissionParams` | `missionParams[]` and each row's `default` | `Resolve()` after a valid parse | the value per symbol: the selection in `$profile:TBD_MissionParams.json` when it is in the row's `values[]`, else the authored default; `Get(symbol)` returns `EMPTY` (0) for an unknown or unusable symbol, and `Has(symbol)` tells it from an authored 0 |

One authored vehicle arrives as two rows: an `entities[]` row that
`TBD_MissionLoader.SpawnMissionEntities` places, and a `vehicles[]` row that adds its `seats[]`.
`TBD_MissionVehicleRoster` indexes the placed bodies (`RecordEntitySpawn`), and
`SeatAuthoredCrews` claims each roster row's twin by `uid` (else by an alias, x and z fingerprint)
before it spawns anything, so a crewed vehicle exists once. It then seats each crew slot's body in
the compartment its role names, and `VerifySeatedCrews` checks a second later which bodies ended up
inside. `TBD_SpawnManager.MaterializeSlotBodies` calls `SeatAuthoredCrews`, then
`TBD_VehicleState.ApplySpawned`, then `TBD_EntityState.ApplySpawned`.

`TBD_MissionSlotStruct.Key()` is the slot's durable key: its `uid`, else its derived `id`. Spawn
points, rosters and logs use it.

Presence follows one rule across the folder, because `JsonLoadContext` allocates a nested `ref`
field even when its key is absent and leaves an absent scalar at its initializer:

- a number that may be authored as 0 starts at a sentinel (`Y_ABSENT`, `ABSENT`, `INDEX_ABSENT`,
  all -1e6 or below);
- a string is present when it is not empty, a container when its `Count()` is not 0;
- a bool cannot tell absent from `false`, so `lock`, `allowDamage` and `showModel` apply only when
  true, and gadget flags use a map whose `Find` is per-flag presence.

## Authority

- Server: everything that reads or applies. `TBD_MissionLoader` binds and resolves on the server
  load path only, `TBD_GadgetFlags` returns on `RplMode.Client`, and the state readers run from
  `TBD_SpawnManager` on the server.
- Client: the struct classes only, as data: the lobby's `TBD_KitInfo` carries a
  `TBD_SlotLoadoutStruct` that the client's kit preview reads.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none; the engine replicates the entity changes the server makes.

## Boundaries

- Depends on: `TBD_MissionLoader` in `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Loaders/`
  (the raw JSON, the parsed document, the mission id); `TBD_SpawnManager` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Spawning/` (slot bodies and seating);
  `TBD_Log`; the engine's `DamageManagerComponent`, `VehicleControllerComponent`,
  `FuelManagerComponent`, `BaseWeaponManagerComponent`, `SCR_GadgetManagerComponent` and
  compartment classes; the wire shape in `contracts_v2/definitions/mission.schema.json`.
- Used by: `TBD_MissionLoader` (document fields, `ResetIndex`, `RecordSpawn`, `Bind`, `Resolve`);
  `TBD_SpawnManager` (`MaterializeSlotBodies`, `GetAssignedSlot`); the loadout scripts in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Loadouts/`; `TBD_WaypointRuntime` (vehicle rows);
  and, through `TBD_MissionSlotStruct`, `apps/mod/tbd-framework/Scripts/Game/TBD/API/TBD_ResultsReporter.c`,
  the objective, safe-start and win-condition scripts under
  `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/`, and the admin, briefing and lobby services
  under `apps/mod/tbd-framework/Scripts/Game/TBD/Session/`.
- Rules: every wire field keeps its JSON key's exact spelling, and each struct carries its
  `//! @contract mission.schema.json#…` tag (`cargo xtask schema citations`); presence uses the
  sentinel, empty-string and `Count()` tests above, never a null test; a new per-row key gets its
  own second-pass reader instead of a field on a primary struct; the Enforce keyword `default` is
  never a field name, so `TBD_MissionParams` reads that key by string; lines added stay ASCII, and
  `cargo xtask mod compile` checks that the scripts compile, while whether a state shows in a round
  is checked by hand.
