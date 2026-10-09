# Match telemetry transport

The game server's side of match telemetry: every match registration, results revision and
detailed event batch goes into a durable, bounded queue under `$profile:TBD/Telemetry/` and stays
there until the platform's ingest routes acknowledge it.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/API/MatchTelemetry/
├── Delivery/  the one-request-at-a-time sender and its answer table
├── Queue/     the durable queue: entries, sealed files, state slots, capacity and drops
└── Reports/   the registration, results revision and event batch bodies, and how they are queued
```

## How it works

```text
TBD_ResultsReporter (round LIVE)  -> TBD_MatchRegistration.BeginRound -> queue: REGISTRATION
TBD_ResultsReporter (round END)   -> TBD_MatchResultsRevision.Enqueue -> queue: RESULTS
TBD_MatchEventRecorder (10 s, END) -> TBD_MatchEventBatch.Enqueue     -> queue: EVENT_BATCH
TBD_RuntimeHeartbeat.TickServer (every beat)
  -> TBD_TelemetryDelivery.Tick -> TBD_GameRuntimeHttp.Post(head entry) with the machine credential
TBD_RuntimeStatusReadings (every online heartbeat)
  -> telemetry_queue {backlog, capacity, dropped_total, oldest_age_seconds} and current_match_id
```

`Reports/` builds each body and queues it through `Queue/`, which writes one sealed file per
entry and keeps the next entry id, the drop total and the per-match results revision and event
sequence counters in two alternating state slots. `Delivery/` posts the head entry and settles
the answer: acknowledge, send the match's registration first, drop, or keep and back off. The
[match telemetry design](/documentation/crates/api/api_server/verification_evidence/telemetry.md) is
the contract; each subfolder's README describes its part.

## Authority

- Server: everything; `TBD_TelemetryDelivery.Tick` returns on a client and the heartbeat loads the
  queue only on the authority.
- Client: nothing.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_GameRuntimeHttp`, `TBD_GameRuntimeAnswer` and `TBD_BackendText` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/API/Http/`; `TBD_RuntimeSession` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/API/RuntimeSession/`; `TBD_Sha256` and `TBD_Log` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Core/`; `TBD_DeployedMission` and `TBD_MissionLoader`
  in `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Loaders/`. Over HTTP, the ingest
  routes of `crates/api/api_match_telemetry/src/`.
- Used by: `TBD_ResultsReporter` in `apps/mod/tbd-framework/Scripts/Game/TBD/API/Results/`;
  `TBD_MatchEventRecorder` in `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/MatchEvents/`;
  `TBD_RuntimeStatusReadings` in `apps/mod/tbd-framework/Scripts/Game/TBD/API/RuntimeSession/`;
  `TBD_RuntimeHeartbeat` in `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Orchestrator/Heartbeat/`.
- Rules: the wire shapes follow `contracts/definitions/match-telemetry.schema.json`; every body
  is hand-built and carries a `@contract` tag; the credential and the bodies are never logged;
  lines added stay ASCII and `cargo xtask mod compile` checks that the scripts compile.

## Related documentation

- [Match telemetry design](/documentation/crates/api/api_server/verification_evidence/telemetry.md) — match identity,
  results revisions, detailed events and the queue's answer table
- [Match telemetry domain](/crates/api/api_match_telemetry/src/README.md) — how the ingest routes take the
  reports in
- [Platform bridge](/apps/mod/tbd-framework/Scripts/Game/TBD/API/README.md) — the machine-credential transport
