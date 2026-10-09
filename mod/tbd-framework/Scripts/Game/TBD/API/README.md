# Platform API bridge

The dedicated server's side of the platform [API](/documentation/glossary/a_to_f.md#api): the
[game runtime](/documentation/glossary/g_to_m.md#game-runtime) session with its heartbeats, the shared
transport of every `/api/v1/game-runtime/`, `/api/v1/fleet-executor/` and `/api/v1/ingest/` call,
in-game identity linking, the end-of-round match results and the durable match telemetry queue.

## Contents

```text
mod/tbd-framework/Scripts/Game/TBD/API/
├── FleetCommands/   the fleet command executor: claim, check, report, run
├── Http/            backend settings, the machine-credential transport and its answers, backend text
├── Identity/        the `arma_id` accessor and the `#tbd link <code>` chat command
├── MatchTelemetry/  the durable telemetry queue, its delivery, and the registration, results and event bodies
├── Results/         the round's stage watch: registration at LIVE, results revision at END
└── RuntimeSession/  this world's runtime session: lifecycle, start, heartbeats, closing
```

## How it works

One authentication tier reaches the platform. `TBD_BackendConfig` (in `Http/`) reads the backend URL
and this server's `machineCredential` from the profile. The machine credential
(`Authorization: Bearer tbdm_...`) carries every `/api/v1/game-runtime/`, `/api/v1/fleet-executor/`
and `/api/v1/ingest/` route through `TBD_GameRuntimeHttp`, which delivers exactly one classified
`TBD_GameRuntimeAnswer` per call: identity link confirmation (`Identity/`, interactive, in memory)
and match telemetry (`MatchTelemetry/`, through the durable queue) included. The engine reports a
request on its REST callback thread; the transport records the answer there and runs every
`OnAnswered` on the main thread (see [Threads](/mod/tbd-framework/Scripts/Game/TBD/API/Http/README.md#threads)),
so the fleet command effects, the deployment decisions and the chat replies that follow an answer
run on the main thread.

```text
SCR_BaseGameMode.OnGameStart (authority, framework world)
  -> TBD_RuntimeSessionLifecycle.Begin
       -> TBD_RuntimeSession: session start, then heartbeats         (RuntimeSession/)
       -> TBD_FleetCommandPoller: claims every 5 s while a session is held  (FleetCommands/)
TBD_MissionLoader.ParseMissionJson
  -> TBD_IdentityLink.Arm      `#tbd link <code>` -> POST /api/v1/ingest/link-confirm  (Identity/)
  -> TBD_ResultsReporter.Arm   round LIVE -> registration, round END -> results revision
                               queued in TBD_TelemetryQueue                       (Results/, MatchTelemetry/)
TBD_RuntimeHeartbeat, every beat on the server
  -> TBD_TelemetryDelivery.Tick  head entry -> POST /api/v1/ingest/{matches,match-results,match-events}
SCR_BaseGameMode.OnGameEnd
  -> TBD_RuntimeSessionLifecycle.End: poller stopped, session closed
```

Every `arma_id` on the wire comes from `TBD_PlayerIdentity.GetArmaId`, so the id a link writes is
byte for byte the one match results carry. Each subfolder's README describes its part.

## Authority

- Server: everything that talks to the platform; the classes carry `@authority server`.
  `TBD_RuntimeSessionLifecycle` starts the session only when `TBD_Authority.IsClient()` is false
  and `TBD_FrameworkManager.IsFrameworkWorld()` holds, and `TBD_IdentityLink.Arm`,
  `TBD_ResultsReporter.Arm` and its stage handler return on a client.
- Client: `TBD_IdentityLink.TryConsumeBeforeBroadcast` runs on every peer from the chat hook in
  `TBD_AdminCommands` and swallows a `#tbd link` line so it is never broadcast; only the authority
  sends it on.
- Owner: nothing.
- RPCs: none; replies to players go through private chat (`SCR_ChatComponent.SendPrivateMessage`).
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_Log` and `TBD_PlayerChat` in `mod/tbd-framework/Scripts/Game/TBD/Core/`;
  `TBD_FrameworkManager` and `TBD_EGameStage` in
  `mod/tbd-framework/Scripts/Game/TBD/Gamemode/`; `TBD_DeployedMission` and
  `TBD_MissionLoader` in `mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Loaders/`;
  `TBD_SpawnManager` in `mod/tbd-framework/Scripts/Game/TBD/Systems/Spawning/`; the engine's
  `RestContext`, `RestCallback`, `SCR_PlayerIdentityUtils` and `SCR_ChatComponent`. Over HTTP, the
  API domains `crates/api/api_server_infrastructure/src/` (sessions and fleet executor),
  `crates/api/api_match_telemetry/src/` (heartbeats, match registration, results and events),
  `crates/api/api_identity_and_access/src/` (link confirmation),
  `crates/api/api_missions/src/` and `crates/api/api_operations/src/`.
- Used by: `TBD_GameRuntimeHttp` by `TBD_DeployedMission`, `TBD_MissionArtifactVerification` and
  `TBD_RosterLoader` in `mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Loaders/`, by
  `TBD_DeploymentAuthorization`, `TBD_DeploymentRequest`, `TBD_DeploymentRequestQueue` and
  `TBD_DeploymentEndQueue` in `mod/tbd-framework/Scripts/Game/TBD/Systems/Spawning/`, and by
  `TBD_DeployableMissionList` and `TBD_MissionDeploymentRelay` in
  `mod/tbd-framework/Scripts/Game/TBD/Session/MissionSelector/`; `TBD_PlayerIdentity` by the
  same relay, `TBD_RosterLoader` and `TBD_DeploymentAuthorization`; `TBD_BackendConfig.SetBackend`
  by `TBD_AdminCommands`; `TBD_IdentityLink` by `TBD_AdminCommands` and `TBD_MissionLoader`;
  `TBD_ResultsReporter` by `TBD_MissionLoader`; `TBD_TelemetryQueue` and `TBD_TelemetryDelivery`
  by `TBD_RuntimeHeartbeat` in `mod/tbd-framework/Scripts/Game/TBD/Gamemode/Orchestrator/Heartbeat/`.
- Rules: every `arma_id` on the wire comes from `TBD_PlayerIdentity.GetArmaId`, and a player
  without one is dropped, never sent under a substitute; no secret is logged; answers are read by
  status and `details.code`, never by message text; no answer handler runs on the engine's REST
  callback thread, so an answer may change the world, a player or the chat; a world's session
  starts only after its artifact report is decided and the previous session has closed; the wire
  shapes follow
  `contracts/definitions/game-runtime-session.schema.json`,
  `contracts/definitions/match-telemetry.schema.json` and
  `contracts/definitions/fleet-command.schema.json`; lines added stay ASCII and
  `cargo xtask mod compile` checks that the scripts compile.

## Related documentation

- [Server infrastructure domain](/crates/api/api_server_infrastructure/src/README.md) — runtime
  sessions, machine credentials and the fleet command ledger on the API side
- [Match telemetry domain](/crates/api/api_match_telemetry/src/README.md) — how heartbeats
  and match telemetry are taken in
- [Match telemetry design](/documentation/crates/api/api_server/verification_evidence/telemetry.md) — match
  identity, results revisions, detailed events and the game-runtime queue
- [Identity and access domain](/crates/api/api_identity_and_access/src/README.md) — the link
  code handshake the `#tbd link` command completes
- [Discord identity link specification](/documentation/mod/tbd-framework/UI/discord_identity_link/discord_identity_link_specification.md)
  — the in-game linking flow
