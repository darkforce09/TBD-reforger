# Platform API bridge

The dedicated server's side of the platform [API](/documentation_v2/glossary/a_to_f.md#api): the
[game runtime](/documentation_v2/glossary/g_to_m.md#game-runtime) session with its heartbeats, the shared
transport of every `/api/v1/game-runtime/` and `/api/v1/fleet-executor/` call, in-game identity
linking, and the end-of-round match results.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/API/
├── FleetCommands/                 the fleet command executor: claim, check, report, run
├── TBD_BackendConfig.c            reads the backend URL and both secrets from the profile
├── TBD_GameRuntimeAnswer.c        classifies an answer: success, 409 refusal, transient, permanent
├── TBD_GameRuntimeHttp.c          the machine-credential transport: one answer per call, backoff
├── TBD_IdentityLink.c             the `#tbd link <code>` chat command and its confirmation
├── TBD_LoadedArtifactReport.c     the artifact (or none) this world's session start reports
├── TBD_PlayerIdentity.c           the one accessor of the `arma_id` every payload carries
├── TBD_ResultsReporter.c          posts the round's outcome and player rows when it ends
├── TBD_RuntimeSession.c           this world's runtime session: start, heartbeats, refusals
├── TBD_RuntimeSessionClosing.c    closes an ended world's session: offline heartbeat, then end
├── TBD_RuntimeSessionLifecycle.c  modded `SCR_BaseGameMode`: starts and stops session and claims
├── TBD_RuntimeSessionWire.c       the session start answer and the start or heartbeat call
└── TBD_RuntimeStatusReadings.c    the live readings a heartbeat carries
```

## How it works

### Two authentication tiers

`TBD_BackendConfig` reads `$profile:TBD_BackendConfig.json` (copied from
`apps/mod/tbd-framework/Data/backend.example.json`), whose keys are the fields of
`TBD_BackendConfigStruct`:

| Key | Default | Sent as | Used for |
|---|---|---|---|
| `backendUrl` | none; no platform connection without it | the base of every URL | every call |
| `serverToken` | none | `X-Service-Token` | `POST /api/v1/ingest/link-confirm` and `POST /api/v1/ingest/match-results` |
| `machineCredential` | none | `Authorization: Bearer tbdm_…` | every `/api/v1/game-runtime/` and `/api/v1/fleet-executor/` route |

The `machineCredential` is this server's `mod_runtime`
[machine credential](/documentation_v2/glossary/g_to_m.md#machine-credential), issued by an administrator.
A value that does not start with `tbdm_`, such as the example's placeholder, counts as unset: no
deployment is read, no runtime session starts, no roster loads and no fleet command is claimed.
`Reload` re-reads the file while the server runs and keeps the settings in force when the file does
not read or parse; the loops that wait on the platform (`TBD_DeployedMission` and
`TBD_RosterLoader` in `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Loaders/`) call it,
so a credential pasted in later is picked up without a restart. `SetBackend`, behind the admin
chat command `#tbd backend`, repoints the URL and token and saves the file. Neither secret is
logged. The mission and its [event](/documentation_v2/glossary/a_to_f.md#event) are not configured here:
they come with the deployment the platform holds for this server.

### The machine-credential transport

`TBD_GameRuntimeHttp.Post` and `Get` open a `RestContext` with the bearer credential, the JSON
content type and a 15 s timeout (`REQUEST_TIMEOUT_S`), and deliver exactly one
`TBD_GameRuntimeAnswer` to the sender's `TBD_GameRuntimeCall` subclass; a request the engine never
reports is answered transient by a 25 s watchdog (`WATCHDOG_MS`). `TBD_GameRuntimeAnswer` classifies
by HTTP status and by the `details.code` of a 409, never by message text: `SUCCESS` (2xx),
`REFUSED` (a 409 fence refusal, with its parsed details), `TRANSIENT` (no answer, a timeout, 408 or
a server error) or `PERMANENT` (any other client error), and keeps the `details.code` of any error,
such as `NO_DEPLOYMENT`, for the caller. `BackoffMs` gives the exponential retry delay every loop
uses. Every sender of the game-runtime routes goes through this transport: the deployed mission and
its artifact, the roster, deployment authorization and ended lives, the runtime session, the fleet
commands, the deployable mission list and the in-game deployment relay.

### The runtime session

```text
SCR_BaseGameMode.OnGameStart (authority, framework world)
  -> TBD_RuntimeSession.Start and TBD_FleetCommandPoller.Start
       waits for TBD_LoadedArtifactReport (decided once per world by TBD_DeployedMission)
       and for TBD_RuntimeSessionClosing to finish closing the previous world's session
  -> POST /api/v1/game-runtime/sessions {loaded_artifact_id, loaded_artifact_sha256} or {}
  -> every heartbeat_interval_seconds:
     POST /api/v1/game-runtime/sessions/{sessionId}/heartbeats {generation, sequence, readings}
SCR_BaseGameMode.OnGameEnd
  -> poller stopped; TBD_RuntimeSessionClosing: an offline heartbeat,
     then POST /api/v1/game-runtime/sessions/{sessionId}/end
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

### Identity linking and match results

`TBD_PlayerIdentity.GetArmaId` returns the engine's player identity exactly as it goes on the wire,
or empty when the host issues none; `IsDurable` is false for the `00bbbddd-` name hash a listen
host synthesizes. Both service-token callers use it, so the `arma_id` a link writes is byte for byte
the one match results carry.

`TBD_IdentityLink` handles `#tbd link <code>` and `#tbd link status`. The line is consumed before
the chat broadcast, so the code never reaches public chat; the authority queues one confirmation at
a time (at most 16 waiting, each with a watchdog) and posts `{code, arma_id, arma_character}` to
`POST /api/v1/ingest/link-confirm`, answering the player privately. A player without a durable
identity is refused and told why.

`TBD_ResultsReporter` is armed by `TBD_MissionLoader` and polls the stage once a second. It stamps
the round's start at `LIVE`, and at `END` or `DEBRIEF` after a `LIVE` posts once to
`POST /api/v1/ingest/match-results`: the match (`source_match_id`, event, mission, terrain, start
and end, `outcome` `success` or `aborted`, `winning_faction`) and one row per connected player
holding a claimed slot, with `role_played`, `deaths` (0 or 1 under one life) and a `counters` block
whose unmeasured counters are zero. A player without an identity is dropped, and
`LogIdentityCensus` logs how many rows are durable, synthetic or unresolved. A failed post is
sent again up to `MAX_ATTEMPTS` (3) attempts in all, idempotent on `source_match_id`; nothing
blocks the stage machine.

## Authority

- Server: everything that talks to the platform. Every file except `TBD_BackendConfig.c` carries
  `@authority server`; `TBD_RuntimeSessionLifecycle` starts the session only when
  `RplSession.Mode()` is not `RplMode.Client` and `TBD_FrameworkManager.IsFrameworkWorld()` holds,
  and `TBD_IdentityLink.Arm`, `TBD_ResultsReporter.Arm` and its stage handler return on a client.
- Client: `TBD_IdentityLink.TryConsumeBeforeBroadcast` runs on every peer from the chat hook in
  `TBD_AdminCommands` and swallows a `#tbd link` line so it is never broadcast; only the authority
  sends it on.
- Owner: nothing.
- RPCs: none; replies to players go through private chat (`SCR_ChatComponent.SendPrivateMessage`).
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_Log` and `TBD_PlayerChat` in `apps/mod/tbd-framework/Scripts/Game/TBD/Core/`;
  `TBD_FrameworkManager` and `TBD_EGameStage` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/`; `TBD_DeployedMission` and
  `TBD_MissionLoader` in `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Loaders/`;
  `TBD_SpawnManager` in `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Spawning/`; the engine's
  `RestContext`, `RestCallback`, `SCR_PlayerIdentityUtils` and `SCR_ChatComponent`. Over HTTP, the
  API domains `apps/website/api_v2/src/server_infrastructure/` (sessions and fleet executor),
  `apps/website/api_v2/src/match_telemetry/` (heartbeats and match results),
  `apps/website/api_v2/src/identity_and_access/` (link confirmation),
  `apps/website/api_v2/src/missions/` and `apps/website/api_v2/src/operations/`.
- Used by: `TBD_GameRuntimeHttp` by `TBD_DeployedMission`, `TBD_MissionArtifactVerification` and
  `TBD_RosterLoader` in `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Loaders/`, by
  `TBD_DeploymentAuthorization`, `TBD_DeploymentRequest`, `TBD_DeploymentRequestQueue` and
  `TBD_DeploymentEndQueue` in `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Spawning/`, and by
  `TBD_DeployableMissionList` and `TBD_MissionDeploymentRelay` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Session/MissionSelector/`; `TBD_PlayerIdentity` by the
  same relay, `TBD_RosterLoader` and `TBD_DeploymentAuthorization`; `TBD_BackendConfig.SetBackend`
  by `TBD_AdminCommands`; `TBD_IdentityLink` by `TBD_AdminCommands` and `TBD_MissionLoader`;
  `TBD_ResultsReporter` by `TBD_MissionLoader`, `TBD_FrameworkManager` and `TBD_DebriefScreen`, which
  adds `FillScoreboard` as a modded method.
- Rules: every `arma_id` on the wire comes from `TBD_PlayerIdentity.GetArmaId`, and a player
  without one is dropped, never sent under a substitute; no secret is logged; answers are read by
  status and `details.code`, never by message text; a world's session starts only after its
  artifact report is decided and the previous session has closed; the wire shapes follow
  `contracts_v2/definitions/game-runtime-session.schema.json` and
  `contracts_v2/definitions/fleet-command.schema.json`; lines added stay ASCII and
  `cargo xtask mod compile` checks that the scripts compile.

## Related documentation

- [Server infrastructure domain](/apps/website/api_v2/src/server_infrastructure/README.md) — runtime
  sessions, machine credentials and the fleet command ledger on the API side
- [Match telemetry domain](/apps/website/api_v2/src/match_telemetry/README.md) — how heartbeats
  and match results are taken in
- [Identity and access domain](/apps/website/api_v2/src/identity_and_access/README.md) — the link
  code handshake the `#tbd link` command completes
- [Discord identity link specification](/documentation_v2/mod/tbd-framework/UI/discord_identity_link/discord_identity_link_specification.md)
  — the in-game linking flow
