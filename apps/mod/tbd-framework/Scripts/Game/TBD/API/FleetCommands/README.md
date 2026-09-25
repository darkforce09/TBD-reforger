# Fleet command executor

The [game runtime](/documentation_v2/glossary.md#game-runtime) as an executor of
[fleet commands](/documentation_v2/glossary.md#fleet-command): while this world holds a runtime
session, it claims the commands addressed to it and runs `broadcast`, `kick` and `load_mission`,
each through the same report-before-effect protocol.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/API/FleetCommands/
├── TBD_FleetCommandArguments.c   the argument checks per action and the preconditions here
├── TBD_FleetCommandExecution.c   one command's protocol: check, executing, effect, result
├── TBD_FleetCommandPoller.c      claims the next command every 5 s while a session is held
├── TBD_FleetLoadMissionAction.c  `load_mission`: verify, cache, then restart in-process
└── TBD_FleetPlayerActions.c      `broadcast` to every player and `kick` with the reason first
```

## How it works

```text
TBD_FleetCommandPoller (every CLAIM_INTERVAL_MS = 5 s, only when idle)
  POST /api/v1/fleet-executor/commands/claim {runtime_session_id}
    204                       -> nothing claimable
    409 RUNTIME_SESSION_ENDED -> TBD_RuntimeSession.ReportSessionEnded
    200 claimed command       -> TBD_FleetCommandExecution.Begin
         CHECKING             TBD_FleetCommandArguments.Check; a refusal -> report failed
         REPORTING_EXECUTING  POST .../commands/{commandId}/executing {fencing_token}
         EFFECT               TBD_FleetPlayerActions or TBD_FleetLoadMissionAction, run once
         REPORTING_RESULT     POST .../commands/{commandId}/result:
                                succeeded {outcome} or failed {reason}
         DONE
```

One command runs at a time: `TBD_FleetCommandExecution` holds it in `s_Current`, and the poller
claims nothing until it is idle, so no effect runs twice or overlaps another. The effect starts only
after the platform admits `executing`. A report that gets no answer (a transient outcome) is sent
again with backoff from `REPORT_RETRY_BASE_MS` (2 s) to `REPORT_RETRY_CAP_MS` (30 s), without
repeating the effect. Any other refusal, a 409 on a stale fencing token included, abandons the
command with an ERROR and no further effect or report, with one exception: a result report refused
because the command already stands `succeeded` means an earlier attempt of that report was
recorded, so what follows a success still happens. A failure reason is cut to printable ASCII of at
most `FAILURE_REASON_MAX_BYTES` (500) bytes. Claim failures other than the session end are logged
at most once a minute; the answer to a claim sent by an earlier world is dropped, and its command
returns to the queue when the platform's 30 s lease lapses.

`TBD_FleetCommandArguments` checks each action's arguments again exactly as the platform validates
them, and only the action's own keys:

| Action | Arguments | Preconditions on this server |
|---|---|---|
| `broadcast` | `message`, 1 to 256 bytes | none |
| `kick` | `arma_id` and optional `reason`, 1 to 128 bytes each; `runtime_session_id` | the session is this runtime's; a connected player has that `arma_id` |
| `load_mission` | `deployment_id` and `artifact_id` (UUIDs), `artifact_sha256` (64 lowercase hex), `runtime_session_id` | the session is this runtime's |

Text is trimmed and holds no control character. Any other action fails as not a game-runtime action.

`broadcast` succeeds with `{delivered_to}`, the number of players reached. `kick` tells the player
the reason in chat, removes them `KICK_DELAY_MS` (2 s) later against whoever holds the identity
then, records the kick with `TBD_AdminAudit`, and succeeds with `{kicked_player_id, arma_id}`; a
player who left in the meantime fails it. `load_mission` reads `GET /api/v1/game-runtime/deployment`
and requires the command's deployment, artifact and SHA-256; it fetches and verifies the artifact
(`TBD_MissionArtifactVerification`), writes it to the profile cache (`TBD_MissionArtifactCache`),
and succeeds with `{artifact_id, restart_requested: true}`. Once that result is recorded it tells
every player, stops the poller and the runtime session, and calls
`GameStateTransitions.RequestScenarioRestart`; the next world's boot loads the cached artifact and
its session start confirms the deployment. A failure before the report is reported `failed`, and
nothing restarts.

## Authority

- Server: everything; every file carries `@authority server`, and the poller starts only from
  `TBD_RuntimeSessionLifecycle` on the authority of a framework world.
- Client: nothing.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_GameRuntimeHttp`, `TBD_GameRuntimeAnswer`, `TBD_RuntimeSession` and
  `TBD_PlayerIdentity` in `apps/mod/tbd-framework/Scripts/Game/TBD/API/`; `TBD_PlayerChat` and
  `TBD_Sha256` in `apps/mod/tbd-framework/Scripts/Game/TBD/Core/`; `TBD_MissionArtifactVerification`,
  `TBD_MissionArtifactCache` and `TBD_RuntimeDeploymentStruct` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Loaders/`; `TBD_AdminAudit` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Session/Admin/`; the engine's `GameStateTransitions`.
  Over HTTP, the executor routes of `apps/website/api_v2/src/server_infrastructure/` and
  `GET /api/v1/game-runtime/deployment` of `apps/website/api_v2/src/missions/`.
- Used by: `TBD_RuntimeSessionLifecycle` in `apps/mod/tbd-framework/Scripts/Game/TBD/API/`, which
  starts and stops the poller.
- Rules: `executing` is admitted before any effect starts, and an effect never repeats; one command
  at a time; the argument checks match the platform's validation in
  `contracts_v2/definitions/fleet-command.schema.json` (`ClaimRequest`, `ClaimedFleetCommand`,
  `ExecutionStart`, `ExecutionResult`); lines added stay ASCII and `cargo xtask mod compile` checks
  that the scripts compile.

## Related documentation

- [Server infrastructure domain](/apps/website/api_v2/src/server_infrastructure/README.md) — the
  command ledger and the executor routes these scripts call
