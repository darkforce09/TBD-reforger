# Match results

The thin end-of-round results report: once a round that went `LIVE` ends, one POST of its outcome
and one row per participating player to the platform, retried a bounded number of times.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/API/Results/
├── TBD_ResultsPayload.c   the round's winner, outcome and player rows, and the JSON body
└── TBD_ResultsReporter.c  watches the stage and posts one report per round with bounded retry
```

## How it works

`TBD_ResultsReporter` is armed by `TBD_MissionLoader` and polls the stage once a second. It stamps
the round's start and its `source_match_id` at `LIVE`, and at `END` or `DEBRIEF` after a `LIVE`
posts once to `POST /api/v1/ingest/match-results`. `TBD_ResultsPayload` builds the body: the match
(`source_match_id`, event, mission, terrain, start and end, `outcome` `success` or `aborted`,
`winning_faction`) and one row per connected player holding a claimed slot, with `role_played`,
`deaths` (0 or 1 under one life) and a complete `counters` block whose unmeasured counters are
zero. `success` needs the mission's `faction_eliminated` end trigger and exactly one of at least two
fielded sides alive. A player without an identity is dropped, and `LogIdentityCensus` logs how many
rows are durable, synthetic or unresolved. A failed post is sent again up to `MAX_ATTEMPTS` (3)
attempts in all, byte-identical and idempotent on `source_match_id`; a missing backend configuration
is a legal local state and is not retried; nothing blocks the stage machine.

## Authority

- Server: everything; `TBD_ResultsReporter.Arm` and `OnStageChanged` return on a client.
- Client: nothing.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_BackendConfig` and `TBD_BackendText` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/API/Http/`; `TBD_PlayerIdentity` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/API/Identity/`; `TBD_FrameworkManager` and
  `TBD_EGameStage` in `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/`; `TBD_DeployedMission` and
  `TBD_MissionLoader` in `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Loaders/`;
  `TBD_SpawnManager` in `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Spawning/`. Over HTTP, the
  match results of `apps/website/api_v2/src/match_telemetry/`.
- Used by: `TBD_MissionLoader` in `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Loaders/`
  (`Arm`).
- Rules: every `arma_id` comes from `TBD_PlayerIdentity.GetArmaId`; `outcome` is always a member of
  the backend's set; `cargo xtask verify results-reporter-identity-comments` keeps the reporter's
  identity comments truthful; lines added stay ASCII and `cargo xtask mod compile` checks that the
  scripts compile.

## Related documentation

- [Platform bridge](/apps/mod/tbd-framework/Scripts/Game/TBD/API/README.md) — the service-token tier this report uses
- [Match telemetry domain](/apps/website/api_v2/src/match_telemetry/README.md) — how match results
  are taken in
