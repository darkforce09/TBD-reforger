# Mission data structs and state readers

The typed shape of a loaded [mission](/documentation/glossary/g_to_m.md#mission)'s slots and vehicles,
and the readers that apply the per-row states the primary parse does not bind: entity and vehicle
state, player gadget flags and launch parameters. The server fills and applies them while a mission
loads and while its slot bodies materialize.

## Contents

```text
mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Data/
├── Document/                   the typed mission document: root, zones, ORBAT, briefings, entities, flow, settings, variants
├── TBD_EntityState.c           entities[] health, allowDamage, showModel and size, applied to placed bodies
├── TBD_GadgetFlags.c           slots[].gadgets: map, compass, watch, GPS and radio added or removed at spawn
├── TBD_MissionFactionNames.c   a faction key's authored display name, falling back to the key
├── TBD_MissionParams.c         missionParams[] rows and the launch value chosen for each symbol
├── TBD_MissionSlotStruct.c     one compiled slot: position, kit, loadout gear and cargo, seat identity
├── TBD_MissionVariants.c       whether a variantId-gated row is in the active variant selection
└── Vehicles/                   vehicles[] rows and crew seats, the roster that seats crews, and vehicle lock, fuel and ammo
```

## How it works

`TBD_MissionSlotStruct` (with `TBD_SlotLoadoutStruct`, `TBD_SlotGearStruct`, `TBD_SlotCargoStruct`)
and `TBD_MissionVehicleStruct` (in `Vehicles/`) are fields of `TBD_MissionDocumentStruct` (in `Document/`), which
`TBD_MissionLoader` binds with `JsonLoadContext` in the primary parse. `JsonLoadContext` binds JSON
keys onto class fields by exact name, so every field name is its JSON key.

The readers each run a second `JsonLoadContext` pass, opened by `TBD_MissionJsonPass.LoadRoot`,
into wire structs that declare only the keys they read, so the primary structs do not grow:

| Reader | Keys | Armed by | Applies |
|---|---|---|---|
| `TBD_EntityState` | `entities[]` `health`, `allowDamage`, `showModel`, `size`, `stamina` | `TBD_MissionWorldApplier.SpawnMissionEntities` records each placed body (`RecordSpawn`) | `ApplySpawned`: `SetHealthScaled`, `EnableDamageHandling`, the `VISIBLE` flag, `SetScale`; `stamina` is logged and not applied (the engine has no stamina toggle) |
| `TBD_VehicleState` (in `Vehicles/`) | `vehicles[]` `lock`, `fuel`, `ammo` | the vehicles already in the world | `ApplySpawned`: `LockPilotControls`, fuel nodes (slotted tanks included), turret and cargo magazines |
| `TBD_GadgetFlags` | `slots[].gadgets` as `map<string, bool>` | `Bind()` from `TBD_MissionWorldApplier.Apply` once the document is valid, which hooks `SCR_BaseGameMode.GetOnPlayerSpawned` | 800 ms after a player spawns (after the loadout's cargo lands), adds or removes each authored gadget |
| `TBD_MissionParams` | `missionParams[]` and each row's `default` | `Resolve()` after a valid parse | the value per symbol: the selection in `$profile:TBD_MissionParams.json` when it is in the row's `values[]`, else the authored default; `Get(symbol)` returns `EMPTY` (0) for an unknown or unusable symbol, and `Has(symbol)` tells it from an authored 0 |

One authored vehicle arrives as two rows, an `entities[]` row and a `vehicles[]` row with its
`seats[]`; [`Vehicles/`](Vehicles/README.md) joins them so a crewed vehicle exists once, seats its
crew, and applies its lock, fuel and ammo. `TBD_SlotBodyMaterializer` calls
`TBD_MissionVehicleRoster.SeatAuthoredCrews`, then `TBD_VehicleState.ApplySpawned`, then
`TBD_EntityState.ApplySpawned`.

`TBD_MissionSlotStruct.Key()` is the slot's durable key: its `uid`, else its derived `id`. Spawn
points, rosters and logs use it.

Presence follows one rule across the folder, because `JsonLoadContext` allocates a nested `ref`
field even when its key is absent and leaves an absent scalar at its initializer:

- a number that may be authored as 0 starts at a sentinel (`Y_ABSENT`, `ABSENT`, `INDEX_ABSENT`,
  all -1e6 or below);
- a string is present when it is not empty, a container when its `Count()` is not 0;
- a bool cannot tell absent from `false`, so `lock`, `allowDamage` and `showModel` apply only when
  true, and gadget flags use a map whose `Find` is per-flag presence.

`TBD_MissionFactionNames.DisplayName(doc, factionKey)` returns a faction's authored display name,
or the key when the faction has none or is not declared; wire builders sanitise it.
`TBD_MissionVariants.IsActive` gates a second-pass row on a `variantId`: an empty id always runs,
and the caller says whether selection is in force and what a missing active set means.
`IsRowIncluded` is the loader's filter over its declared and active sets, which drops a dangling
id with one WARNING.

## Authority

- Server: everything that reads or applies. `TBD_MissionLoader` binds and resolves on the server
  load path only, `TBD_MissionWorldApplier` records the placed bodies and binds the gadget flags from
  `TBD_LoadingGate` on the server's main thread, `TBD_GadgetFlags` returns on a client
  (`TBD_Authority.IsClient`), and the state readers run from `TBD_SlotBodyMaterializer` on the
  server.
- Client: the struct classes only, as data: the lobby's `TBD_KitInfo` carries a
  `TBD_SlotLoadoutStruct` that the client's kit preview reads.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none; the engine replicates the entity changes the server makes.

## Boundaries

- Depends on: `TBD_MissionLoader` in `mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Loaders/`
  (the raw JSON, the parsed document, the mission id); `TBD_MissionJsonPass` in
  `mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Ingestion/` (the second-pass context);
  `TBD_EntityQuery`, `TBD_LoadoutInventoryUtil.PrefabOf`; `TBD_SpawnManager` in
  `mod/tbd-framework/Scripts/Game/TBD/Systems/Spawning/` (slot bodies and seating);
  `TBD_Log`; the engine's `DamageManagerComponent`, `VehicleControllerComponent`,
  `FuelManagerComponent`, `BaseWeaponManagerComponent`, `SCR_GadgetManagerComponent` and
  compartment classes; the wire shape in `contracts/definitions/mission.schema.json`.
- Used by: `TBD_MissionLoader` (document fields, `Resolve`) and `TBD_MissionWorldApplier`
  (`ResetIndex`, `RecordSpawn`, `Bind`); `TBD_SlotBodyMaterializer` (`ApplySpawned`); the loadout scripts in
  `mod/tbd-framework/Scripts/Game/TBD/Systems/Loadouts/`; `TBD_WaypointRuntime` (vehicle rows);
  and, through `TBD_MissionSlotStruct`, `mod/tbd-framework/Scripts/Game/TBD/API/Results/TBD_ResultsReporter.c`,
  the objective, safe-start and win-condition scripts under
  `mod/tbd-framework/Scripts/Game/TBD/Gamemode/`, and the admin, briefing and lobby services
  under `mod/tbd-framework/Scripts/Game/TBD/Session/`.
- Rules: every wire field keeps its JSON key's exact spelling, and each struct carries its
  `//! @contract mission.schema.json#…` tag; presence uses the
  sentinel, empty-string and `Count()` tests above, never a null test; a new per-row key gets its
  own second-pass reader instead of a field on a primary struct; the Enforce keyword `default` is
  never a field name, so `TBD_MissionParams` reads that key by string; lines added stay ASCII, and
  `cargo xtask mod compile` checks that the scripts compile, while whether a state shows in a round
  is checked by hand.
