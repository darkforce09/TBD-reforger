# Runtime session

This server's [game runtime](/documentation_v2/glossary/g_to_m.md#game-runtime) session on the
platform: one session per world, fenced by a per-server generation, started once the world has
decided what it runs, kept alive by heartbeats and closed when the world ends.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/API/RuntimeSession/
├── TBD_LoadedArtifactReport.c         the artifact (or none) this world's session start reports
├── TBD_RuntimeSession.c               this world's runtime session: start, heartbeats, refusals
├── TBD_RuntimeSessionCall.c           a start or heartbeat request with its world and session
├── TBD_RuntimeSessionClosing.c        closes an ended world's session: offline heartbeat, then end
├── TBD_RuntimeSessionLifecycle.c      starts and stops session and claims; modded `SCR_BaseGameMode`
├── TBD_RuntimeStatusReadings.c        the live readings a heartbeat carries
└── TBD_StartedRuntimeSessionStruct.c  the session start answer
```

## How it works

```text
SCR_BaseGameMode.OnGameStart (authority, framework world)
  -> TBD_RuntimeSessionLifecycle.Begin: TBD_RuntimeSession.Start and TBD_FleetCommandPoller.Start
       waits for TBD_LoadedArtifactReport (decided once per world by TBD_DeployedMission)
       and for TBD_RuntimeSessionClosing to finish closing the previous world's session
  -> POST /api/v1/game-runtime/sessions {loaded_artifact_id, loaded_artifact_sha256} or {}
  -> every heartbeat_interval_seconds:
     POST /api/v1/game-runtime/sessions/{sessionId}/heartbeats {generation, sequence, readings}
SCR_BaseGameMode.OnGameEnd
  -> TBD_RuntimeSessionLifecycle.End: poller stopped; TBD_RuntimeSessionClosing sends an offline
     heartbeat, then POST /api/v1/game-runtime/sessions/{sessionId}/end
```

The start reports the loaded [artifact](/documentation_v2/glossary/a_to_f.md#artifact), and that report
confirms a [mission deployment](/documentation_v2/glossary/g_to_m.md#mission-deployment); a report the
platform rejects (422 `UNKNOWN_ARTIFACT`, 400) is an ERROR and the session starts without one.
Starting a session ends the server's previous one as `superseded`. The heartbeat sequence rises
strictly within a session, and one start or heartbeat is in flight at a time. A refused heartbeat
steers the loop: `STALE_SEQUENCE` continues past `details.last_sequence`, `STALE_GENERATION`
adopts the session's own generation, `RUNTIME_SESSION_ENDED` with `expired` starts a new session,
and any other end reason (`superseded`, `credential_revoked`, `ended_by_runtime`) stops the loop
with an ERROR. Other failures back off from 2 s to 60 s and never stop it. Without a usable
credential the world holds no session and checks again every minute. `GetSessionId` is the session
that deployment authorization, ended lives and fleet command claims address.
`TBD_RuntimeStatusReadings` fills each heartbeat with `player_count`, `max_players`, `server_fps`,
`uptime_seconds`, `ingame_time` and `ingame_weather`, omitting a reading the engine cannot give
rather than sending zero.

## Authority

- Server: everything; `TBD_RuntimeSessionLifecycle.RunsHere` admits only the authority of a
  framework world, and `TBD_RuntimeSession.Start` returns on a client.
- Client: nothing.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_GameRuntimeHttp`, `TBD_GameRuntimeAnswer`, `TBD_BackendConfig` and
  `TBD_BackendText` in `apps/mod/tbd-framework/Scripts/Game/TBD/API/Http/`; `TBD_FleetCommandPoller`
  in `apps/mod/tbd-framework/Scripts/Game/TBD/API/FleetCommands/`; `TBD_FrameworkManager` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/`; `TBD_Authority` and `TBD_Log` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Core/`; the engine's `SCR_BaseGameMode`,
  `TimeAndWeatherManagerEntity` and `ServerInfo`. Over HTTP, the session routes of
  `apps/website/api_v2/src/server_infrastructure/`.
- Used by: `TBD_DeployedMission` in `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Loaders/`
  (the loaded-artifact report); `TBD_DeploymentAuthorization` and `TBD_DeploymentRequestQueue` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Spawning/` (`CanHoldSession`, `GetSessionId`,
  `ReportSessionEnded`); the fleet commands in
  `apps/mod/tbd-framework/Scripts/Game/TBD/API/FleetCommands/`.
- Rules: a world's session starts only after its artifact report is decided and the previous session
  has closed; the wire shapes follow `contracts_v2/definitions/game-runtime-session.schema.json`;
  lines added stay ASCII and `cargo xtask mod compile` checks that the scripts compile.

## Related documentation

- [Platform bridge](/apps/mod/tbd-framework/Scripts/Game/TBD/API/README.md) — how the session fits the other backend clients
- [Server infrastructure domain](/apps/website/api_v2/src/server_infrastructure/README.md) — runtime
  sessions on the API side
- [Match telemetry domain](/apps/website/api_v2/src/match_telemetry/README.md) — how heartbeats are
  taken in
