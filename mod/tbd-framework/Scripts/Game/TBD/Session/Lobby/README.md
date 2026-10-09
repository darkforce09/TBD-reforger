# Lobby and slotting

The pre-game lobby: the stage watcher that raises the pre-game screens, the
[ORBAT](/documentation/glossary/n_to_z.md#orbat) roster wire through which a player claims, releases and
deploys into a [slot](/documentation/glossary/n_to_z.md#slot) under one life, the catalog behind the
Lobby screen, and the overlook camera a player without a body sees.

## Contents

```text
mod/tbd-framework/Scripts/Game/TBD/Session/Lobby/
├── Catalog/                 the Lobby screen's data and intents: factions, squads, seats, kits
├── PreSlot/                 the overlook camera for a player with no body, and its arm
├── Service/                 server roster builder, seat actions, the roster models and wire
├── UI/                      the Lobby screen: factions, roles, kit inspector, pause-menu hook
├── SCR_PlayerController.c   modded `SCR_PlayerController`: the roster RPCs
├── TBD_LobbyClient.c        client roster cache, optimistic claims, reconciliation
├── TBD_LobbyComponent.c     game mode component: wire self-check, stage watcher
└── TBD_LobbyStage.c         client stage watcher for the pre-game screens
```

## How it works

### The stage watcher

`TBD_LobbyComponent`, on `mod/tbd-framework/Prefabs/Systems/TBD_GameMode.et`, runs
`TBD_LobbyRosterWireSelfCheck.Run` at boot on every machine and starts `TBD_LobbyStage` 2 s after
init; `Start` logs a refusal and stops on a dedicated server. `TBD_LobbyStage` polls the replicated
stage every 500 ms: on entering `LOBBY` it resets the roster cache and raises the Mission Selector
(`TBD_UIMissionSelector`), from whose top bar the Lobby and Briefing tabs are reached; while the
stage stays in `LOBBY` it raises the selector again when no pre-game screen is open, the player has
no body and the server has not accepted a deploy; on any other stage it closes the selector and the
lobby. Arming waits for a framework world, retrying every 250 ms for up to 60 attempts (15 s), and
then logs the failure.

### The roster wire

```text
client TBD_LobbyClient.Request / Claim / Release / Deploy (optimistic edit first)
  -> TBD_RequestLobbyRoster / ClaimSlot / ReleaseSlot / Deploy   --RPC-->  server
       TBD_LobbyService.ApplyClaim -> TBD_SpawnManager.ClaimSlot
       TBD_LobbyService.ApplyRelease -> TBD_SpawnManager.ReleaseSlot
       TBD_LobbyService.ApplyDeploy -> TBD_SpawnManager.DeployPlayerEx
       TBD_LobbyService.BuildForPlayer: parses TBD_SpawnManager.BuildSlotRoster
       TBD_LobbyRosterWire.Serialise
  <--RPC-- TBD_RpcDo_LobbyRoster(wire): the whole roster plus a V verdict record
client TBD_LobbyClient.Accept -> TBD_LobbyRosterWire.Parse: replaces the cached roster wholesale
```

Every request is answered with one message: the whole roster as the server sees it after the action,
with an optional `V` record naming the action, whether it succeeded, and why not. The client shows a
claim at once and reconciles by replacing its roster with each reply, so a refused claim reverts in
the same message that explains it. [Service/](Service/README.md) holds the builder, the models and
the wire format. A deploy the platform is still deciding (`AUTHORIZING`) or cannot authorize now
(`UNAUTHORIZED`) gets its own sentence and is not marked accepted.

No script calls `TBD_LobbyClient.Request`, `Claim`, `Release` or `Deploy`: the Lobby screen reads
`TBD_LobbyCatalog` in [Catalog/](Catalog/README.md), which `Get()` builds from `TBD_LobbyMock` in
`mod/tbd-framework/Scripts/Game/TBD/UI/Mock/` until something calls `Set()`, and no script
does. The catalog's `Claim` and `Release` change that local copy only.

### The pre-slot camera

[PreSlot/](PreSlot/README.md) shows a player who has controlled nothing for 3 s a slow overlook of
the terrain instead of a black screen, and steps aside for a body or the spectator.

## Authority

- Server: `TBD_LobbyService` and the server halves of the RPCs (`@authority server`); the caller is
  always `GetPlayerId()` of the controller the request arrived on.
- Client: `TBD_LobbyClient`, `TBD_LobbyStage`, the catalog, the pre-slot camera and the screen
  (`@authority client` on the watcher and the camera). `TBD_LobbyComponent` runs its wire
  self-check on every machine.
- Owner: the `TBD_Request*` calls and `TBD_RpcDo_LobbyRoster`, the one reply, which runs on the
  requesting client only.
- RPCs, on the modded `SCR_PlayerController`, as their `@rpc` tags state:
  `TBD_RpcAsk_LobbyRoster`, `TBD_RpcAsk_ClaimSlot(string)`, `TBD_RpcAsk_ReleaseSlot` and
  `TBD_RpcAsk_Deploy` (Reliable, Server); `TBD_RpcDo_LobbyRoster(string)` (Reliable, Owner).
- Replicated properties: none here; the watcher reads `TBD_FrameworkManager`'s replicated stage.

## Boundaries

- Depends on: `TBD_SpawnManager` (slots, lives and deployment) in
  `mod/tbd-framework/Scripts/Game/TBD/Systems/Spawning/`; `TBD_FrameworkManager` in
  `mod/tbd-framework/Scripts/Game/TBD/Gamemode/Orchestrator/`; `TBD_MissionLoader` in
  `mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Loaders/`; `TBD_SpectatorController` in
  `mod/tbd-framework/Scripts/Game/TBD/Session/Spectator/`; `TBD_MenuStack` in
  `mod/tbd-framework/Scripts/Game/TBD/UI/Core/`; `TBD_LobbyMock` in
  `mod/tbd-framework/Scripts/Game/TBD/UI/Mock/`; `TBD_SessionSelection` in
  `mod/tbd-framework/Scripts/Game/TBD/Session/MissionSelector/`.
- Used by: `mod/tbd-framework/Prefabs/Systems/TBD_GameMode.et`, which attaches
  `TBD_LobbyComponent` and `TBD_PreSlotComponent`; `TBD_BriefingScreen`, `TBD_BriefingOrbatPage`
  and `TBD_PlayersPanel`, which read `TBD_LobbyCatalog`.
- Rules: the server never takes a player id from the client, and `TBD_SpawnManager.ClaimSlot` judges
  every claimed seat; the roster is only ever the parse of `TBD_SpawnManager.BuildSlotRoster`, never
  a second opinion; a reply replaces the client roster, never merges into it; the wire self-check
  passes at boot; lines added stay ASCII and `cargo xtask mod compile` checks that the scripts
  compile.

## Related documentation

- [Lobby specification](/documentation/mod/tbd-framework/UI/lobby/lobby_specification.md) — the screen as built, its
  data, design target, open work and decisions
- [Lobby design references](/documentation/mod/tbd-framework/UI/lobby/visual_references/README.md)
  — the Stitch mockup sets and the Arma 3 captures the screen started from
- [Spawning](/mod/tbd-framework/Scripts/Game/TBD/Systems/Spawning/README.md) — the slot map,
  one-life bookkeeping and deployment the lobby wire calls into
