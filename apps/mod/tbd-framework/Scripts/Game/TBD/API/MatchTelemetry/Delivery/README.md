# Telemetry delivery

The sender of the telemetry queue: one request in flight at a time, each answer settled by the
match telemetry design's outcome table.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/API/MatchTelemetry/Delivery/
└── TBD_TelemetryDelivery.c  the delivery call, the per-beat pump and the answer table
```

## How it works

`TBD_RuntimeHeartbeat.TickServer` calls `TBD_TelemetryDelivery.Tick` every beat. A tick first lets
`TBD_MatchRegistration` queue a registration that was waiting for a runtime session, then, when no
call is in flight and no backoff is pending, posts the queue's head entry to its route
(`/api/v1/ingest/matches`, `/api/v1/ingest/match-results` or `/api/v1/ingest/match-events`) through
`TBD_GameRuntimeHttp`, which sends the machine credential as `Authorization: Bearer`. When an entry
leaves the queue the next one is sent at once.

| Answer | Queue action |
|---|---|
| 2xx; 409 `STALE_REVISION` | acknowledged: the entry is deleted; a registration's `match_id` goes to `TBD_MatchRegistration` |
| 409 `MATCH_NOT_REGISTERED` | the match's registration is put first (queued, or re-queued from the state), then the entry retries; a second refusal of the same entry is kept with backoff |
| any other 409 code (`REGISTRATION_CONFLICT`, `REVISION_CONFLICT`, `EVENT_CONFLICT`, `EVENT_SEQUENCE_CONFLICT`, `MATCH_FINALIZED`); 400 and the other client errors | dropped, counted, logged at ERROR |
| 401, 403 | kept; retried with backoff, so a credential fix still delivers |
| no answer, a timeout, 429, 5xx; a request that could not be sent | kept; retried with backoff |

The backoff is 2 s after the first kept failure, doubling to 60 s (`TBD_GameRuntimeHttp.BackoffMs`);
an entry that leaves the queue resets it. A `MATCH_NOT_REGISTERED` for the round whose registration
still waits for a session is kept; one for a match whose registration is neither queued nor
remembered is dropped.

## Authority

- Server: everything; `Tick` returns on a client.
- Client: nothing.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_TelemetryQueue` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/API/MatchTelemetry/Queue/`; `TBD_MatchRegistration` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/API/MatchTelemetry/Reports/`; `TBD_GameRuntimeHttp` and
  `TBD_GameRuntimeAnswer` in `apps/mod/tbd-framework/Scripts/Game/TBD/API/Http/`; `TBD_Authority`
  and `TBD_Log` in `apps/mod/tbd-framework/Scripts/Game/TBD/Core/`. Over HTTP, the ingest routes of
  `crates/api/api_match_telemetry/src/`.
- Used by: `TBD_RuntimeHeartbeat` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Orchestrator/Heartbeat/`.
- Rules: answers are read by status and `details.code`, never by message text; the body and the
  credential are never logged; lines added stay ASCII and `cargo xtask mod compile` checks that the
  scripts compile.

## Related documentation

- [Telemetry queue](/apps/mod/tbd-framework/Scripts/Game/TBD/API/MatchTelemetry/Queue/README.md) — what is sent
  and how entries are dropped
- [Match telemetry design](/documentation/apps/api/verification_evidence/telemetry.md) — the answer
  table this delivery applies
