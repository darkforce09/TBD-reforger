# Seats and slot bodies

Who holds which mission slot, and the dressed body that stands on each slot: the lineup spawned at
mission load, the fresh body of every new life, the loadout check that opens spawning, and the
roster line the lobby reads.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Spawning/Slots/
├── TBD_SlotBodyDressing.c      authored stance and rank applied to a freshly spawned body
├── TBD_SlotBodyMaterializer.c  the slot lineup and every rematerialized body; body bookkeeping
├── TBD_SlotClaimBook.c         seats: claims, releases, automatic seating, retained departed seats
├── TBD_SlotLoadoutSettle.c     the spawn boundary: waits for every loadout pass, opens or refuses
└── TBD_SlotRosterWire.c        the lobby's tab-separated roster line per slot
```

## How it works

`TBD_SlotBodyMaterializer.MaterializeSlotBodies` spawns one body per compiled slot at its authored
transform (`TBD_PlacementScatter`, the JSON `y` or the terrain surface), parks its AI unless
`TBD_WaypointRuntime.ShouldEnableAIAtSpawn` keeps it, applies the slot identity through
`TBD_SlotBodyDressing`, and starts a `TBD_LoadoutApplication` that `TBD_SlotLoadoutSettle` tracks.
The same pass seats vehicle crews (`TBD_MissionVehicleRoster`), applies vehicle cargo and fuel
defaults and the vehicle and entity states, then arms the settle. The settle polls every 250 ms,
for at most 40 polls: when every application is done and none is unplayable it opens spawning;
a timeout or an unplayable body refuses the lobby and every deploy.

`TBD_SlotClaimBook` holds the seats. A claim from the lobby is refused for a spent life and for a
seat another player holds; after `LOBBY` a claim deploys at once. Automatic seating tries, in
order, a departed seat of the same identity, the identity's remembered slot, its event roster slot
and the next free slot. A spent life that leaves keeps its seat as a `TBD_DepartedSeat`, keyed on the
slot. `TBD_SlotRosterWire` renders the seats as `OPEN`, `HELD` or `DEAD` lines.

## Authority

- Server: everything; the helpers are owned by `TBD_SpawnManager` and called from its server-side
  paths (`@authority server` on the entry points).
- Client: nothing.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none; clients learn the roster from the lobby service.

## Boundaries

- Depends on: `TBD_MissionLoader`, `TBD_RosterLoader`, `TBD_Registry`, `TBD_PlacementScatter`,
  `TBD_LoadoutApplication`, `TBD_WaypointRuntime`, `TBD_MissionVehicleRoster`,
  `TBD_VehicleSpawnDefaults`, `TBD_VehicleState`, `TBD_EntityState`, `TBD_AdminAudit`,
  `TBD_DeploymentAuthorization`, and `TBD_SpawnManager` with its other helpers.
- Used by: `TBD_SpawnManager` (`GetSlots`, `GetBodies`, `GetLoadoutSettle` and the forwarders
  `ClaimSlot`, `ReleaseSlot`, `BuildSlotRoster`, `MaterializeSlotBodies`, `GetSlotBody`), the deploy,
  identity and lives helpers, and `TBD_DynamicSpawnVolley` (`EngineFactionKey`).
- Rules: a slot has at most one holder; a body is reused only alive and by the identity it was
  handed to; spawning opens only through the settle; roster fields never carry a separator.
