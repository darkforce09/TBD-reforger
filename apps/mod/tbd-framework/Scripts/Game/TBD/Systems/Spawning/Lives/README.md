# One life, deaths and departures

ONE LIFE bookkeeping and the two events that end a sitting: a death, which spends the life or
schedules a redeploy, and a disconnect, which retains a spent life's seat and clears every row
keyed on the numeric id. The admin respawn is the only way back for a spent life.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Spawning/Lives/
├── TBD_DeathRespawnFlow.c  a death, the automatic redeploy, the admin respawn and its settlement
├── TBD_OneLifeLedger.c     spent lives keyed on the bind key; pending admin respawns
└── TBD_SpawnDeparture.c    a disconnect: seat, life, body and per-id rows
```

## How it works

`TBD_DeathRespawnFlow.OnPlayerKilled` re-arms the killed player's deploy bookkeeping and closes their
spawn ticket; with `m_bOneLife` on it then writes the spent life into `TBD_OneLifeLedger` and keeps
the seat, otherwise, with automatic deploy on, it redeploys after `m_iRedeployDelayMs`.
`AdminRespawn` deploys a fresh body past the one-life guard and gives the life back only once the
result is `DEPLOYED`; `RETRY` and a platform decision keep the player dead with the override
pending. `TBD_SpawnDeparture.OnPlayerDisconnected` releases a spectator's streaming host, remembers
a live player's slot for a reconnect, forgets the body vanilla is about to delete, retains a spent
life's seat, drops a NUMERIC death mark so the next holder of the id is not dead on arrival, and
closes the connection epoch.

## Authority

- Server: everything (`@authority server` on the entry points); `TBD_SpawnManager` returns before
  calling them on a client.
- Client: nothing.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_SpectatorHost`, and `TBD_SpawnManager` with its other helpers.
- Used by: `TBD_SpawnManager` (`OnPlayerKilled`, `OnPlayerDisconnected`, `AdminRespawn`,
  `IsPlayerDead`), which the admin, lobby, spectator, win-condition and results code call;
  `TBD_DeployWaves` and `TBD_SpawnDeploymentGate` settle admin respawns through
  `FinishAdminRespawn`.
- Rules: the death mark is written before anything can deploy the player again; lives are keyed on
  the bind key, never the numeric id; a spent life's seat stays claimed and counted.
