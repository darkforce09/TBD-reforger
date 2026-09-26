# Vanilla spawn flow stood down

Modded vanilla classes that keep vanilla's own spawn flow out of a framework world and route what
remains through the spawn manager; on a plain vanilla world each behaves as vanilla.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Spawning/VanillaBridge/
├── SCR_MenuSpawnLogic.c                no spawn-point wait; the pull spawn routed to DeployPlayerEx
├── SCR_PossessSpawnHandlerComponent.c  possess requests only for the body a spawn ticket names
└── SCR_RespawnSystemComponent.c        registration and audit swallowed; every other request gated
```

## How it works

In a framework world (`TBD_FrameworkManager.IsFrameworkWorld()`) the modded
`SCR_RespawnSystemComponent` swallows vanilla player registration and audit, which otherwise re-roll
a player's faction looking for spawn points, tears down vanilla's loading placeholder, and refuses
in `CanRequestSpawn_S` every spawn request no spawn ticket covers. The modded
`SCR_PossessSpawnHandlerComponent` gates `CanHandleRequest_S` and `HandleRequest_S` on a ticket for
that exact player and body and spends it when vanilla reports success; vanilla's own
`CanRequestSpawn_S` for possess is skipped while `m_bIgnoreConditions` is set, so the gate sits where
it cannot be skipped. The modded `SCR_MenuSpawnLogic` never waits for spawn points and sends
`DoSpawn_S` to `TBD_SpawnManager.DeployPlayerEx`, falling through to vanilla only on `NOT_MINE`.

## Authority

- Server: every override (`@authority server`); vanilla calls them on the server.
- Client: nothing.
- Owner: nothing.
- RPCs: none added; vanilla's possess request RPC reaches the gated handler.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_SpawnManager` and its `TBD_PossessTicketLedger`, `TBD_FrameworkManager`, and
  vanilla's `SCR_RespawnSystemComponent`, `SCR_PossessSpawnHandlerComponent` and
  `SCR_MenuSpawnLogic`.
- Used by: the engine, through `apps/mod/tbd-framework/Prefabs/Systems/TBD_GameMode.et` and the
  player controller prefab.
- Rules: every override checks for a framework world first; the gates fail closed without a
  `TBD_SpawnManager`; `IsRespawnEnabled` stays vanilla, because it would also refuse the framework's
  own possess request.
