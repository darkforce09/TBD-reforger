# Lobby and slotting

The pre-game lobby: the stage watcher that raises the pre-game screens, the
[ORBAT](/documentation_v2/glossary.md#orbat) roster wire through which a player claims, releases and
deploys into a [slot](/documentation_v2/glossary.md#slot) under one life, the catalog behind the
Lobby screen, and the overlook camera a player without a body sees.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Session/Lobby/
├── TBD_LobbyCatalog.c                         the Lobby screen's data and intents: factions, seats, kits
├── TBD_LobbyClient.c                          client roster cache, optimistic claims, reconciliation
├── TBD_LobbyComponent.c                       game mode component: wire self-check, stage watcher
├── TBD_LobbyController.c                      modded `SCR_PlayerController`: the roster RPCs
├── TBD_LobbyData.c                            the roster models: sides, groups, seats and the verdict
├── TBD_LobbyService.c                         server roster builder, seat actions, wire format
├── TBD_LobbyServiceDeploymentAuthorization.c  the deploy reply while the platform decides a seat
├── TBD_LobbyStage.c                           client stage watcher for the pre-game screens
├── TBD_PreSlotCamera.c                        `TBD_PreSlotCamera`: an overlook for a player with no body
├── TBD_PreSlotComponent.c                     game mode component that arms the pre-slot camera
└── UI/                                        the Lobby screen: factions, roles, kit inspector
```

## How it works

### The stage watcher

`TBD_LobbyComponent`, on `apps/mod/tbd-framework/Prefabs/Systems/TBD_GameMode.et`, runs
`TBD_LobbyService.SelfCheckWire` at boot on every machine and starts `TBD_LobbyStage` 2 s after
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
  <--RPC-- TBD_RpcDo_LobbyRoster(wire): the whole roster plus a V verdict record
client TBD_LobbyClient.Accept: replaces the cached roster wholesale
```

Every request is answered with one message: the whole roster as the server sees it after the action,
with an optional `V` record naming the action, whether it succeeded, and why not. The client shows a
claim at once and reconciles by replacing its roster with each reply, so a refused claim reverts in
the same message that explains it. The roster is not re-derived: `BuildForPlayer` parses the six
tab-separated columns of `TBD_SpawnManager.BuildSlotRoster` (slot key, faction, group, role, state
`OPEN`, `HELD` or `DEAD`, holder) and adds display names. The wire is one line per record (`M`
mission, `V` verdict, `L` life spent, `D` in the world, `X` roster unavailable, `F` faction, `G`
group, `S` seat), tab-separated with every field prefixed by `.`, at most 600 lines. On a listen
host the request builds the payload in place and still goes through `Serialise`, `Accept` and
`Parse`. `TBD_LobbyServiceDeploymentAuthorization` rewords a deploy the platform is still deciding
(`AUTHORIZING`) or cannot authorize now (`UNAUTHORIZED`) and does not mark it accepted.

No script calls `TBD_LobbyClient.Request`, `Claim`, `Release` or `Deploy`: the Lobby screen reads
`TBD_LobbyCatalog`, which `Get()` builds from `TBD_LobbyMock` in
`apps/mod/tbd-framework/Scripts/Game/TBD/UI/Mock/` until something calls `Set()`, and no script
does. The catalog's `Claim` and `Release` change that local copy only.

### The pre-slot camera

`TBD_PreSlotComponent`, on the same game mode prefab, arms `TBD_PreSlotCameraArm` 2 s after init
when its `m_bPreSlotCamera` attribute is on (the default); off, it logs a WARNING and a player with
no body sees black. The arm polls every 250 ms: once the local player has controlled nothing for 3 s
it spawns `TBD_PreSlotCamera`, which orbits the centre of the world's bound box at 800 m radius and
300 m height at 1.5 degrees per second; it steps aside as soon as the player controls a body or
`TBD_SpectatorController` takes the view.

## Authority

- Server: `TBD_LobbyService` (`ApplyClaim`, `ApplyRelease`, `ApplyDeploy`, `BuildForPlayer`) and
  the server halves of the RPCs (`@authority server`); the caller is always `GetPlayerId()` of the
  controller the request arrived on.
- Client: `TBD_LobbyClient`, `TBD_LobbyStage`, `TBD_LobbyCatalog`, the pre-slot camera and the
  screen (`@authority client` on the watcher and the camera). `TBD_LobbyComponent` runs its wire
  self-check on every machine (`@authority any`).
- Owner: `TBD_RpcDo_LobbyRoster`, the one reply, runs on the requesting client only.
- RPCs, on the modded `SCR_PlayerController`, as their `@rpc` tags state:
  `TBD_RpcAsk_LobbyRoster`, `TBD_RpcAsk_ClaimSlot(string)`, `TBD_RpcAsk_ReleaseSlot` and
  `TBD_RpcAsk_Deploy` (Reliable, Server); `TBD_RpcDo_LobbyRoster(string)` (Reliable, Owner).
- Replicated properties: none here; the watcher reads `TBD_FrameworkManager`'s replicated stage.

## Boundaries

- Depends on: `TBD_SpawnManager` (slots, lives and deployment) in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Spawning/`; `TBD_FrameworkManager` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Orchestrator/`; `TBD_MissionLoader` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Loaders/`; `TBD_SpectatorController` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Session/Spectator/`; `TBD_MenuStack` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/`; `TBD_LobbyMock` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Mock/`; `TBD_SessionSelection` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Session/MissionSelector/`.
- Used by: `apps/mod/tbd-framework/Prefabs/Systems/TBD_GameMode.et`, which attaches
  `TBD_LobbyComponent` and `TBD_PreSlotComponent`; `TBD_BriefingScreen`, `TBD_BriefingOrbatPage`
  and `TBD_PlayersPanel`, which read `TBD_LobbyCatalog`.
- Rules: the server never takes a player id from the client, and `TBD_SpawnManager.ClaimSlot` judges
  every claimed seat; the roster is only ever the
  parse of `TBD_SpawnManager.BuildSlotRoster`, never a second opinion; a reply replaces the client
  roster, never merges into it; the wire self-check passes at boot; lines added stay ASCII and
  `cargo xtask mod compile` checks that the scripts compile.

## Related documentation

- [Lobby specification](/documentation_v2/mod/tbd-framework/UI/lobby/lobby_specification.md) — the
  lobby's design target
- [Spawning](/apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Spawning/README.md) — the slot map,
  one-life bookkeeping and deployment the lobby wire calls into
