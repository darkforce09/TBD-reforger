# Ready and Continue on the client

The client end of the briefing's Ready and Continue button: the request to the authority, its
answer, and the invoker the briefing screen binds to.

## Contents

```text
mod/tbd-framework/Scripts/Game/TBD/Systems/Spawning/Client/
├── SCR_PlayerController.c  the request and reply RPC pair on the player controller
└── TBD_SpawnClient.c       the last answer and its invoker; the request entry point
```

## How it works

The briefing screen calls `TBD_SpawnClient.Request`, which calls `TBD_RequestReadyDeploy` on the
local player controller. On a client it sends `TBD_RpcAsk_ReadyDeploy`; the server runs
`TBD_SpawnManager.DeployOnReady` and answers the owner with `TBD_RpcDo_ReadyDeployResult`. On a
listen host or in local play the deploy runs in place. Either way `TBD_SpawnClient.AcceptResult`
stores the answer and fires `GetOnDeployResult`.

## Authority

- Server: `TBD_RpcAsk_ReadyDeploy` and `TBD_ReadyDeployHere` (`@authority server`).
- Client: `TBD_SpawnClient`, which the briefing screen calls and binds to.
- Owner: `TBD_RequestReadyDeploy` and `TBD_RpcDo_ReadyDeployResult` (`@authority owner`).
- RPCs:
  - `TBD_RpcAsk_ReadyDeploy`: Reliable, Server (`@rpc Reliable Server`).
  - `TBD_RpcDo_ReadyDeployResult`: Reliable, Owner (`@rpc Reliable Owner`); `ok` and `why`.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_SpawnManager.DeployOnReady`, `TBD_Authority`.
- Used by: `TBD_BriefingScreen` in
  `mod/tbd-framework/Scripts/Game/TBD/Session/Briefing/UI/`.
- Rules: the RPC names differ from the lobby picker's `TBD_RequestDeploy` pair, because every
  modded block of `SCR_PlayerController` shares one method namespace.
