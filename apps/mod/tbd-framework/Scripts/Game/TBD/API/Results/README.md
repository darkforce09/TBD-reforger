# Match results

The end-of-round side of match telemetry: a round that goes `LIVE` is registered with the platform,
and once it ends one results revision of its outcome and one line per participating player is
queued for delivery.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/API/Results/
├── TBD_ResultsPayload.c   the round's outcome, player lines, identity census and source match id
└── TBD_ResultsReporter.c  watches the stage: registers the round at LIVE, queues its results at END
```

## How it works

`TBD_ResultsReporter` is armed by `TBD_MissionLoader` and polls the stage once a second. At `LIVE`
it stamps the round's start and a fresh `source_match_id` and hands both to
`TBD_MatchRegistration.BeginRound`, which queues the round's registration. At `END` or `DEBRIEF`
after a `LIVE` it takes the winner from `TBD_FactionElimination.CountSurvivors` (the rule the END
banner uses), the outcome from `TBD_ResultsPayload.ResolveOutcome` (`success` needs the mission's
`faction_eliminated` end trigger and exactly one of at least two fielded sides alive, else
`aborted`) and one `TBD_MatchPlayerLine` per connected player holding a claimed slot from
`CollectPlayers`: `arma_id`, `role_played`, the line key `source_event_id` (the deployed event, or
the source match id without one), kills, team kills, the longest kill and vehicles destroyed from
`TBD_MatchTelemetryTally` and deaths (0 or 1 under one life) from `TBD_SpawnManager`. The same
stage watch starts `TBD_MatchEventRecorder` right after the registration at `LIVE`, and flushes it
before the report at `END` or `DEBRIEF` and on `LOADING`. `TBD_MatchResultsRevision.Enqueue` issues the next
revision and queues the body; the durable queue and its delivery in
[`../MatchTelemetry/`](/apps/mod/tbd-framework/Scripts/Game/TBD/API/MatchTelemetry/README.md) carry
it to `POST /api/v1/ingest/match-results`. A player without an identity is dropped, and
`LogIdentityCensus` logs how many lines are durable, synthetic or unresolved. Nothing here sends a
request, so nothing blocks the stage machine.

## Authority

- Server: everything; `TBD_ResultsReporter.Arm` and `OnStageChanged` return on a client.
- Client: nothing.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_BackendText` in `apps/mod/tbd-framework/Scripts/Game/TBD/API/Http/`;
  `TBD_MatchRegistration`, `TBD_MatchResultsRevision` and `TBD_MatchPlayerLine` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/API/MatchTelemetry/Reports/`; `TBD_PlayerIdentity` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/API/Identity/`; `TBD_FrameworkManager`,
  `TBD_FactionElimination` and `TBD_EGameStage` in `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/`; `TBD_DeployedMission` and
  `TBD_MissionLoader` in `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Loaders/`;
  `TBD_SpawnManager` in `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Spawning/`;
  `TBD_MatchEventRecorder` and `TBD_MatchTelemetryTally` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/MatchEvents/`. Over HTTP,
  through the telemetry queue, the match registration and results of
  `apps/website/api_v2/src/match_telemetry/`.
- Used by: `TBD_MissionLoader` in `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Loaders/`
  (`Arm`).
- Rules: every `arma_id` comes from `TBD_PlayerIdentity.GetArmaId`; `outcome` is always a member of
  the backend's set; every line has a non-empty `role_played` (`unassigned` for a slot authored
  without one) and `source_event_id`; `cargo xtask verify results-reporter-identity-comments` keeps the reporter's
  identity comments truthful; lines added stay ASCII and `cargo xtask mod compile` checks that the
  scripts compile.

## Related documentation

- [Platform bridge](/apps/mod/tbd-framework/Scripts/Game/TBD/API/README.md) — the machine-credential tier every report uses
- [Match telemetry transport](/apps/mod/tbd-framework/Scripts/Game/TBD/API/MatchTelemetry/README.md) — the durable queue
  and its delivery
- [Match telemetry design](/documentation_v2/website/api_v2/verification_evidence/telemetry.md) — registration,
  results revisions and the queue
- [Match telemetry domain](/apps/website/api_v2/src/match_telemetry/README.md) — how match results
  are taken in
