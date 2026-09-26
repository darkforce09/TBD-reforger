# Spawn manager component

The game-mode component that owns every player spawn in a framework world, the result every deploy
answers with, and the connection epochs that tell a player apart from whoever is later handed the
same numeric id.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Spawning/Manager/
├── TBD_ConnectionEpochs.c  one stamp per sitting of a numeric player id; deferred callbacks quote it
├── TBD_EDeployResult.c     the answer of one deploy attempt, NOT_MINE the only one vanilla may act on
└── TBD_SpawnManager.c      the component: engine hooks, editor attributes, public surface, helpers
```

## How it works

`TBD_SpawnManager` is attached by `apps/mod/tbd-framework/Prefabs/Systems/TBD_GameMode.et` and found
through `GetInstance`, which resolves the component on the current game mode at every call. Its
constructor creates one instance of each helper in `Slots/`, `Identity/`, `Deploy/` and `Lives/` and
passes itself to them; the helpers reach one another through its getters. The engine hooks stay on
the component: `OnPostInit` subscribes `TBD_DeployWatchdog.OnPlayerSpawnedHook` to the game mode's
spawn invoker, `OnPlayerAuditSuccess` runs `TBD_SpawnJoinAudit`, `OnPlayerKilled` ends the platform
life and runs `TBD_DeathRespawnFlow`, `OnPlayerDisconnected` runs `TBD_SpawnDeparture` and ends the
platform life, `OnGameEnd` ends every platform life, and `OnDelete` cancels every helper's pending
callbacks. `OnStageChanged`, called by `TBD_FrameworkManager.SetStage`, caches the stage the helpers
read.

`TBD_ConnectionEpochs` gives each join a fresh positive epoch and drops it at disconnect.
`ScriptCallQueue.Remove` cancels by function, not per player, so every per-player deferred callback
carries the epoch it was scheduled under and does nothing once it moved on; `TBD_SpectatorHost` and
`TBD_DeploymentAuthorization` stamp their records the same way through `ConnectionEpochFor` and
`IsConnectionCurrent`.

## Authority

- Server: the whole component (`@authority server` on the class and on each hook); every hook
  returns on a client.
- Client: nothing.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: the helpers in the sibling folders, `TBD_DeploymentAuthorization` (life ends),
  `TBD_SpawnDeploymentGate`, and the engine's `SCR_BaseGameModeComponent` hooks.
- Used by: `TBD_FrameworkManager`, the objectives, safe-start and win-condition scripts, the admin,
  briefing, lobby, players, post-game and spectator code, `TBD_ResultsReporter`, and the AI,
  Markers, Mission, Radio and Zones scripts, all through `TBD_SpawnManager.GetInstance`.
- Rules: `TBD_SpawnManager` is a frozen class name (a prefab references it) and its public members
  keep their signatures; the three attributes keep their names and defaults; `cargo xtask mod
  compile` checks the scripts compile.
