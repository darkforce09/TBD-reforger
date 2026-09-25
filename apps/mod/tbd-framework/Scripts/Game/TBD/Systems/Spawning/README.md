# Slot bodies, deployment and one life

Owns every player spawn in a framework world: stands one body per [mission](/documentation_v2/glossary.md#mission)
[slot](/documentation_v2/glossary.md#slot) in the world, hands each player onto the body of the
slot they hold, keeps one life per player, asks the platform to authorize each deployment into an
[event](/documentation_v2/glossary.md#event) seat, and stands the vanilla respawn flow down. It
also runs the mission's AI spawn modules.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Spawning/
├── TBD_DeploymentAuthorization.c              platform authorization of event-seat deploys; open lives
├── TBD_DeploymentEndQueue.c                   bounded retry queue reporting ended lives
├── TBD_DeploymentRequest.c                    a deployment request record and the decision struct
├── TBD_DeploymentRequestQueue.c               deployment requests, one at a time, in order, retried
├── TBD_DynamicSpawner.c                       spawnModules[]: AI group waves and garrisons
├── TBD_SCR_MenuSpawnLogic.c                   vanilla menu spawn logic routed to the spawn manager
├── TBD_SCR_PossessSpawnHandlerComponent.c     possess requests only for the authorized body
├── TBD_SCR_RespawnSystemComponent.c           vanilla registration, audit and spawn requests stood down
├── TBD_SpawnClient.c                          Ready & Continue on the client: request and answer
├── TBD_SpawnController.c                      Ready & Continue request and reply RPCs
├── TBD_SpawnManager.c                         slot bodies, claims, deploys, one life, vehicle defaults
└── TBD_SpawnManagerDeploymentAuthorization.c  the spawn manager's authorization gate and life ends
```

## How it works

### Slot bodies and deploys

`TBD_SpawnManager` is a `SCR_BaseGameModeComponent` on
`apps/mod/tbd-framework/Prefabs/Systems/TBD_GameMode.et`. Once the mission is valid,
`TBD_FrameworkManager` calls `MaterializeSlotBodies`: one body per compiled slot at its authored
position (`TBD_PlacementScatter` offset, the JSON `y` or the terrain surface), with the kit prefab
from `TBD_Registry`, its AI disabled unless `TBD_WaypointRuntime.ShouldEnableAIAtSpawn` says
otherwise, and a `TBD_LoadoutApplication` dressing it. The same pass seats vehicle crews, adds
default cargo and full fuel to vehicles (`TBD_VehicleSpawnDefaults`, authored values win), and
applies the vehicle and entity states. The lobby does not open until every loadout has settled
(`IsLoadoutSettlePending`), and a blocking loadout failure refuses the lobby
(`IsLoadoutDeliveryRefused`).

A player gets a slot from the event roster (`TBD_RosterLoader.GetSlotForIdentity`), from a claim
in the lobby (`ClaimSlot`), or round-robin (`AssignSlotForPlayer`). `DeployPlayerEx(playerId)` is
the one door into the world:

```text
DeployPlayerEx ──> slot assigned? ──no──> RETRY
      │ yes
      v
one life spent? ──yes──> DENIED (only AdminRespawn passes)
      │ no
      v
TBD_SpawnDeploymentGate.Admits ──wait──> AUTHORIZING   ──cannot authorize──> UNAUTHORIZED (seat kept)
      │ admitted
      v
AuthorizeSpawn: a ticket for this player and this body ──> vanilla POSSESS request ──> DEPLOYED
```

The deploy takes over the existing body through vanilla's possess spawn request, so the client runs
vanilla's finalize and no second body is created. `TBD_EDeployResult` also has `ALREADY`, `FAILED`
and `NOT_MINE`; only `NOT_MINE` (a client, or no framework mission) lets vanilla spawn anyone.
Claimed slot holders deploy once when the stage moves from `LOBBY` to `BRIEFING`, and with
`m_bAutoDeploy` on, unclaimed players are seated at the same moment. `DeployOnReady` serves the
briefing's Ready & Continue button: the slot path first (in local play it also advances a `LOBBY`
round to `BRIEFING`), then, in local play only, a walk-on vanilla rifleman on dry ground; never past
a spent life.

### One life and identity

With `m_bOneLife` on, death is terminal: the slot stays held, `IsPlayerDead` answers true, and only
`AdminRespawn` (called by `TBD_AdminService`, which checks permission) puts the player back on a
freshly materialized body. Lives, seats and reclaims are keyed on the player's durable key: the
backend identity on a correctly configured dedicated server, a name-derived identity on a listen
host, or a `player:<id>` fallback that is not durable. `StageRefusalFor` refuses the stages that need
one life enforced while any connected player has the fallback key, until an admin signs a waiver
(`AcceptNonDurableIdentity`, revoked by `RequireDurableIdentity`). A player who leaves after their
death keeps the seat (`TBD_DepartedSeat`), and `CountAliveForFaction` and `CountClaimedForFaction`
feed the win conditions.

### Vanilla stood down

In a framework world (`TBD_FrameworkManager.IsFrameworkWorld()`), the modded
`SCR_RespawnSystemComponent` swallows vanilla player registration and audit, which otherwise
re-roll the player's faction looking for spawn points, and refuses every spawn request the spawn
manager did not authorize. The modded `SCR_PossessSpawnHandlerComponent` gates `CanHandleRequest_S`
and `HandleRequest_S` on the spawn manager's ticket for that exact player and entity, and spends it
when vanilla reports success; without the spawn manager it refuses. The modded
`SCR_MenuSpawnLogic` never waits for spawn points and routes `DoSpawn_S` through `DeployPlayerEx`.
On a vanilla world every override falls through to vanilla.

### Platform deployment authorization

When the running deployment names an event, `TBD_SpawnDeploymentGate.Admits` asks
`TBD_DeploymentAuthorization.Check` before any body is prepared:

- until `TBD_RosterLoader.IsSlotTableLoaded()`, every deployment is refused ("the event roster has
  not loaded on this server yet");
- a slot the roster does not list is not an event seat and deploys without asking (one WARNING per
  slot);
- a listed slot goes to `TBD_DeploymentRequestQueue` with a fresh `player_life_id` per attempt:
  `POST /api/v1/game-runtime/sessions/{sessionId}/deployments` with
  `{event_mission_id, orbat_slot_id, arma_id, player_life_id}`, addressed to
  `TBD_RuntimeSession`'s session, one request at a time, each player's in order, retried with the
  same id and backoff from 2 s to 30 s, and held while an ended life of the same player or slot
  is still being reported;
- `allowed` remembers the life and its `occupancy_id` and continues the deploy
  (`OnDeploymentAuthorized`); `denied` tells the player the reason in private chat and gives the
  seat back (`OnDeploymentRefused`), except `SLOT_NOT_IN_LOADED_MISSION`, a server fault, where the
  player keeps the seat; a refusal that is not about the seat keeps it too. A dead player waiting
  on an admin respawn stays dead.

Every refusal reaches the admin audit trail once per cause per round (a denial once per player,
seat and reason). A life ends on death, disconnect, a seat change, the round's end or the world's
end, and `TBD_DeploymentEndQueue` reports it: `POST /api/v1/game-runtime/sessions/{sessionId}/deployments/{occupancyId}/end`,
in order, with backoff from 2 s to 60 s, at most 256 waiting (the oldest is dropped with an ERROR),
and a rejected report dropped with an ERROR. A deployment without an event never asks.

### AI spawn modules

`TBD_DynamicSpawner` adds a `modded class SCR_BaseGameMode` whose one-second tick, on the server in
a framework world, reads `spawnModules[]` once per mission id. A `wave` spawns its `count` groups
at `intervalSeconds`, or once its `triggerId` has fired, while fewer than `maxAlive` (at most 32)
groups live; a `garrison` spawns once and is not restocked. A module names either `x` and `z` or a
`zoneId` from `TBD_ZoneRegistry`. Every spawned group is deleted when the round reaches `END`.

### Attributes

| Attribute on `TBD_SpawnManager` | Default | Meaning |
|---|---|---|
| `m_bAutoDeploy` | `1` (`TBD_GameMode.et` sets `0`) | also seat unclaimed players when the briefing starts |
| `m_iRedeployDelayMs` | `5000` | delay before an automatic redeploy after death, with one life off and `m_bAutoDeploy` on |
| `m_bOneLife` | `1` | death is terminal; only an admin respawn puts a player back |

## Authority

- Server: everything in `TBD_SpawnManager` (`@authority server` on the class and its entry points),
  the deployment files (`@authority server` on each header), the vanilla overrides (each override
  tagged `@authority server`) and `TBD_DynamicSpawner` (`@authority server` on `OnGameStart`, which
  returns on `RplMode.Client`).
- Client: `TBD_SpawnClient`, which the briefing screen calls and binds to.
- Owner: `TBD_RpcDo_ReadyDeployResult` runs on the requesting client (`@authority owner`). In local
  play or on a listen host, `TBD_RequestReadyDeploy` runs the deploy in place without an RPC.
- RPCs, on the modded `SCR_PlayerController`:
  - `TBD_RpcAsk_ReadyDeploy`: Reliable, Server (`@rpc Reliable Server`); calls `DeployOnReady`.
  - `TBD_RpcDo_ReadyDeployResult`: Reliable, Owner (`@rpc Reliable Owner`); `ok` and `why`.
- Replicated properties: none; the slot assignment is a server-side map, and clients learn it from
  the lobby and briefing services.

## Boundaries

- Depends on:
  - `TBD_MissionLoader`, `TBD_RosterLoader` and `TBD_DeployedMission` in
    `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Loaders/`, the structs and readers in
    `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Data/`, `TBD_PlacementScatter`,
    `TBD_LoadoutApplication`, `TBD_WaypointRuntime` and `TBD_ZoneRegistry`;
  - `TBD_FrameworkManager` (the stage and the framework-world test), `TBD_Registry`, `TBD_Log`;
  - `TBD_GameRuntimeHttp`, `TBD_RuntimeSession` and `TBD_PlayerIdentity` in
    `apps/mod/tbd-framework/Scripts/Game/TBD/API/`;
  - over HTTP with the `mod_runtime` [machine credential](/documentation_v2/glossary.md#machine-credential),
    `/api/v1/game-runtime/sessions/{sessionId}/deployments` and its `/{occupancyId}/end` route
    (`apps/website/api_v2/src/operations/routes.rs`), shaped by
    `contracts_v2/definitions/game-runtime-deployment.schema.json`;
  - the engine's spawn classes (`SCR_RespawnSystemComponent`, `SCR_MenuSpawnLogic`,
    `SCR_PossessSpawnHandlerComponent`, `SCR_PossessSpawnData`) and `SCR_AIGroup`.
- Used by: `TBD_FrameworkManager` in `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Orchestrator/`;
  the objectives, safe-start and win-condition scripts under
  `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/`; the admin, briefing, lobby, players,
  post-game and spectator code under `apps/mod/tbd-framework/Scripts/Game/TBD/Session/`
  (`TBD_BriefingScreen` binds `TBD_SpawnClient`); `TBD_ResultsReporter`, `TBD_IdentityLink` and
  `TBD_PlayerIdentity` in `apps/mod/tbd-framework/Scripts/Game/TBD/API/`; the AI, Loadouts,
  Markers, Mission, Radio and Zones folders beside this one; and
  `apps/mod/tbd-framework/Prefabs/Systems/TBD_GameMode.et`, which attaches `TBD_SpawnManager` and
  `SCR_RespawnSystemComponent`.
- Rules: in a framework world only `TBD_SpawnManager` spawns a player, and only `NOT_MINE` falls
  through to vanilla; a spawn ticket names one player and one body and is spent once; one life is
  keyed on the durable player key, never the numeric id; deployment authorization fails closed until
  the slot table loads, and a lost request is repeated with the same `player_life_id`; every `Rpc()`
  stays within eight parameters; lines added stay ASCII, and `cargo xtask mod compile` checks that
  the scripts compile, while a deploy on a dedicated server is checked by hand.

## Related documentation

- [Mod design](/documentation_v2/mod/tbd-framework/mod_design.md) — one life, the possess deploy
  and the vanilla respawn stand-down among the framework's non-negotiables
- [Spawn determinism](/documentation_v2/runbooks/spawn_determinism.md) — the Workbench gate that
  checks spawn and equip give the same outcome across fresh Workbench processes
