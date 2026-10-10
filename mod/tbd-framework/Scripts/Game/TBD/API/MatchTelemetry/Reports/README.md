# Match telemetry reports

The bodies the game server reports about a match, built by hand to the match telemetry schema and
queued for delivery: the registration when a round goes `LIVE`, a results revision when it ends,
and batches of detailed events.

## Contents

```text
mod/tbd-framework/Scripts/Game/TBD/API/MatchTelemetry/Reports/
├── TBD_MatchEventBatch.c       the `MatchEventBatch` envelope around 1 to 500 events, and its enqueue
├── TBD_MatchRegistration.c     the round's `MatchRegistration`, its answered `match_id`, `current_match_id`
└── TBD_MatchResultsRevision.c  the player line and the `MatchResultsRevision` body, and its enqueue
```

## How it works

`TBD_MatchRegistration.BeginRound` snapshots the round's source match id, start, deployed mission
and event and terrain when `TBD_ResultsReporter` sees `LIVE`, and queues
`{source_match_id, runtime_session_id, started_at, mission_id?, event_id?, terrain?}` as soon as
`TBD_RuntimeSession` holds a session (at once, normally; otherwise on a later delivery tick). The
queue remembers the body so it can be sent again ahead of a report the API answers
`MATCH_NOT_REGISTERED`. The API's answer `{match_id, registered}` is kept for the current round, and
every online heartbeat reports it as `current_match_id` (an empty value while the round's
registration is unanswered).

`TBD_MatchResultsRevision.Enqueue` takes the next revision from `TBD_TelemetryQueue` and queues
`{revision, match{source_match_id, outcome, event_id?, mission_id?, terrain?, started_at?, ended_at?,
winning_faction?}, players[{arma_id, role_played, source_event_id, counters}]}`. Every line carries a
complete `counters` block (`kills`, `deaths`, `team_kills`, `longest_kill_m`, `vehicles_destroyed`,
`is_command`, `command_win`); there are no flat counter keys. Kills, team kills, the longest kill
and vehicles destroyed come from `TBD_MatchTelemetryTally` and deaths from `TBD_SpawnManager`;
`is_command` and `command_win` are sent as false and null.

`TBD_MatchEventBatch.Enqueue(source, events)` is the entry point of detailed-event capture: it wraps
1 to 500 complete `MatchEvent` objects, in capture order and numbered with
`TBD_TelemetryQueue.ReserveEventSequences`, in `{source_match_id, events}` and queues them.
`TBD_MatchEventRecorder` in `mod/tbd-framework/Scripts/Game/TBD/Systems/MatchEvents/` calls it
with batches of at most 100 events, recording under `TBD_MatchRegistration.GetSourceMatchId`.

`mission_id` and `event_id` are sent only as UUIDs and the other optional fields only when known: an
absent field keeps the stored value, while a malformed one would refuse the whole report.

## Authority

- Server: everything; the callers run on the authority.
- Client: nothing.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_TelemetryQueue` in
  `mod/tbd-framework/Scripts/Game/TBD/API/MatchTelemetry/Queue/`; `TBD_RuntimeSession` in
  `mod/tbd-framework/Scripts/Game/TBD/API/RuntimeSession/`; `TBD_BackendText` and
  `TBD_GameRuntimeAnswer` in `mod/tbd-framework/Scripts/Game/TBD/API/Http/`;
  `TBD_DeployedMission` and `TBD_MissionLoader` in
  `mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Loaders/`; `TBD_Log` in
  `mod/tbd-framework/Scripts/Game/TBD/Core/`.
- Used by: `TBD_ResultsReporter` and `TBD_ResultsPayload` in
  `mod/tbd-framework/Scripts/Game/TBD/API/Results/`; `TBD_MatchEventRecorder` in
  `mod/tbd-framework/Scripts/Game/TBD/Systems/MatchEvents/`; `TBD_TelemetryDelivery` in
  `mod/tbd-framework/Scripts/Game/TBD/API/MatchTelemetry/Delivery/`;
  `TBD_RuntimeStatusReadings` in `mod/tbd-framework/Scripts/Game/TBD/API/RuntimeSession/`.
- Rules: each body builder carries `@contract match-telemetry.schema.json#/definitions/<Name>`;
  every string goes through `TBD_BackendText.JsonEscape`; lines added stay ASCII and
  `cargo xtask mod compile` checks that the scripts compile.

## Related documentation

- [Match results](/mod/tbd-framework/Scripts/Game/TBD/API/Results/README.md) — the stage watch that registers
  and reports each round
- [Match telemetry design](/documentation/crates/api/api_server/design_notes/telemetry.md) — match identity,
  results revisions and detailed events
