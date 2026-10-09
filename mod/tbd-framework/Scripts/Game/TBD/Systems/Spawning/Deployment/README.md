# Platform deployment authorization

Asks the TBD platform whether a player may deploy into an event roster seat, applies its decision to
the waiting deploy, and reports every life it allowed when that life ends.

## Contents

```text
mod/tbd-framework/Scripts/Game/TBD/Systems/Spawning/Deployment/
├── TBD_DeploymentAuthorization.c  the event-seat gate, open lives, refusals and life ends
├── TBD_DeploymentEndQueue.c       bounded retry queue reporting ended lives
├── TBD_DeploymentRequest.c        a deployment request record and the decision struct
├── TBD_DeploymentRequestQueue.c   deployment requests, one at a time, in order, retried
└── TBD_SpawnDeploymentGate.c      the deploy path's view of the gate and of each decision
```

## How it works

`TBD_DeployExecutor` asks `TBD_SpawnDeploymentGate.Admits` once a seat is assigned. For a mission
deployed for a platform event, `TBD_DeploymentAuthorization.Check` refuses every deploy until the
event roster's slot table loads, lets a slot the table does not list deploy, and sends a listed
slot to `TBD_DeploymentRequestQueue` (`POST /api/v1/game-runtime/sessions/{sessionId}/deployments`)
with a fresh `player_life_id`; the deploy answers `AUTHORIZING` meanwhile. An allowed decision
continues the deploy through `TBD_SpawnDeploymentGate.OnAuthorized` (the retry step); a denial tells
the player why and returns them to slot selection through `OnRefused`, except
`SLOT_NOT_IN_LOADED_MISSION`, which keeps the seat. `TBD_DeploymentEndQueue` reports each ended
life (`POST .../deployments/{occupancyId}/end`).

## Authority

- Server: everything (`@authority server` on each header and on the gate's entry points).
- Client: nothing.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_GameRuntimeHttp`, `TBD_RuntimeSession`, `TBD_PlayerIdentity`,
  `TBD_DeployedMission`, `TBD_RosterLoader`, `TBD_PlayerChat`, `TBD_AdminAudit`, and
  `TBD_SpawnManager` with its helpers; over HTTP the game-runtime deployment routes, shaped by
  `contracts/definitions/game-runtime-deployment.schema.json`.
- Used by: `TBD_DeployExecutor` (`Admits`, `Refusal`), `TBD_SlotClaimBook` (seat changes and
  releases), `TBD_SpawnManager` (life ends on death, disconnect, round and world end), and the lobby
  and admin services in `mod/tbd-framework/Scripts/Game/TBD/Session/`.
- Rules: the gate fails closed until the slot table loads; a lost request is repeated with the same
  `player_life_id`; a decision for a connection that moved on does nothing.
