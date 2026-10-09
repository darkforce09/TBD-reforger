# Deploys

The one door into the world and everything that drives it: the deploy decision, the takeover of a
slot body through vanilla's possess request, the scheduled deploys and retries, the spawn tickets,
the post-deploy watchdog, and the briefing's Ready and Continue button.

## Contents

```text
mod/tbd-framework/Scripts/Game/TBD/Systems/Spawning/Deploy/
├── TBD_DeployExecutor.c       the deploy decision and the body takeover; the last deploy result
├── TBD_DeployWatchdog.c       spawn landing, the arrival check, the transform log and the body census
├── TBD_DeployWaves.c          the BRIEFING holder deploy, automatic seating, slot changes, the retry ladder
├── TBD_PossessTicketLedger.c  spawn tickets: one player, one body, spent once
└── TBD_ReadyDeploy.c          Ready and Continue: the slot path, else a local-play walk-on body
```

## How it works

`TBD_DeployExecutor.DeployPlayerInternal` decides every deploy, in order: `NOT_MINE` on a client or
without a framework mission; `DENIED` for a spent life unless an admin respawn overrides; `FAILED`
during `LOBBY`; `DENIED` while the lineup is unplayable; `RETRY` until the lineup and the roster are
ready; `ALREADY` for a bound player; then the seat and the platform gate
(`TBD_SpawnDeploymentGate`). A missing, dead, forced or foreign body is rematerialized; a body still
being dressed answers `RETRY`. `HandPlayerOntoBody` sets the faction, releases a spectator's
streaming host, opens a `TBD_PossessTicketLedger` ticket for that body, asks vanilla's possess
request (falling back to a direct bind) and arms the watchdog. `DeployPlayerEx` records each result
for the platform-aware callers.

`TBD_DeployWaves` deploys every claimed holder 250 ms after `LOBBY` becomes `BRIEFING` (or when the
settle opens during `BRIEFING`), with automatic deploy on also every unclaimed player, moves a
holder who claims another seat, and retries a `RETRY` every 500 ms up to 20 times, stamped with the
connection epoch and carrying the admin override only for a pending admin respawn.
`TBD_DeployWatchdog` re-arms a deploy whose body never arrived within 10 s, logs the landed
transform and runs a character census 5 s after a spawn burst.

## Authority

- Server: everything (`@authority server` on the entry points); the executor answers `NOT_MINE` on
  a client.
- Client: nothing.
- Owner: nothing.
- RPCs: none here; Ready and Continue arrives through the RPC pair in `Client/`.
- Replicated properties: none.

## Boundaries

- Depends on: vanilla's `SCR_PossessSpawnRequestComponent`, `SCR_PossessSpawnData`,
  `SCR_PlayerFactionAffiliationComponent` and `SCR_FactionManager`; `TBD_SpectatorHost`,
  `TBD_MissionLoader`, `TBD_RosterLoader`, `TBD_FrameworkManager`, `TBD_CharacterUtil`, and
  `TBD_SpawnManager` with its other helpers.
- Used by: `TBD_SpawnManager` (`DeployPlayerEx`, `DeployOnReady`, `LastDeployResult`,
  `ForgetDeployResult` and its getters), the modded `SCR_MenuSpawnLogic`,
  `SCR_PossessSpawnHandlerComponent` and `SCR_RespawnSystemComponent` in `VanillaBridge/`, and
  `TBD_SpawnDeploymentGate` in `Deployment/`.
- Rules: every deploy passes `DeployPlayerInternal`; only the admin respawn and its retries pass the
  override; a ticket names one body and is spent once; only `NOT_MINE` lets vanilla spawn; the
  walk-on runs in local play only and never while the platform decides the seat.
