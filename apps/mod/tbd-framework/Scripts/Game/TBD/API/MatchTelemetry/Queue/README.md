# Telemetry queue

The durable, bounded queue that holds every match registration, results revision and event batch
until the platform acknowledges it, across scenario restarts and server restarts.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/API/MatchTelemetry/Queue/
├── TBD_TelemetryQueue.c         the queue: enqueue, capacity and drops, per-match counters, reload, stats
├── TBD_TelemetryQueueEntry.c    one entry, its kind and route, and the heartbeat's queue reading
├── TBD_TelemetryQueueState.c    the state slot record: next entry id, drop total, per-match counters
├── TBD_TelemetryQueueStorage.c  entry files and the two alternating state slots on disk
└── TBD_TelemetrySealedFile.c    a file sealed by a trailer with its record's length and SHA-256
```

## How it works

On disk, under `$profile:TBD/Telemetry/`:

| File | Content |
|---|---|
| `entry_<id>.entry` | a header line `{kind, source_match_id, route, enqueued_at}` (epoch seconds), then the body |
| `state_a.state`, `state_b.state` | `{serial, next_entry_id, dropped_total, matches[{source_match_id, results_revision, event_sequence, registration_body}]}` |

Every file ends in an 82-byte trailer, `\nTBDQ <10-digit length> <SHA-256>\n`, over the record
before it. `FileIO` has no rename, so `TBD_TelemetrySealedFile` writes the record, reads it back in
one `ReadArray`, hashes it with `TBD_Sha256` and only then appends the trailer; a write cut short
leaves a file whose trailer is missing or wrong. On reload an entry file that fails its seal is
deleted, logged at ERROR and added to `dropped_total`. State saves alternate between the two slots
and the valid slot with the higher `serial` wins, so a torn save never loses the previous state.

`TBD_TelemetryQueue` loads once per process at the first game start (`EnsureLoaded`, called from
the heartbeat) and writes through on every change; the state is saved before an entry file is
written, so an entry id is never issued twice. The capacity is 512 entries, 32 of them reserved for
registrations and results: event batches may hold at most 480. When an entry does not fit, the
oldest event batch is dropped, or the oldest entry when only registrations and results remain,
never the entry in flight; every drop is counted and logged at ERROR. A newer results revision of a
match replaces an unsent older one in place. `Stats()` answers the heartbeat's
`telemetry_queue {backlog, capacity, dropped_total, oldest_age_seconds}`.

The enqueue surface:

| Call | Use |
|---|---|
| `EnqueueRegistration(source, body)` | queues a `MatchRegistration` and remembers it for `MATCH_NOT_REGISTERED` |
| `EnqueueResults(source, body)` | queues a `MatchResultsRevision`, replacing an unsent older one |
| `EnqueueEventBatch(source, body)` | queues a `MatchEventBatch`; `TBD_MatchEventBatch.Enqueue` builds the envelope |
| `NextResultsRevision(source)` | issues and persists the next results revision (1 first) |
| `ReserveEventSequences(source, count)` | reserves and persists `count` event sequences, answering the first |
| `LastEventSequence(source)` | the last event sequence issued, 0 before the first |

The counters of at most 16 matches are kept; a match is forgotten only while no queued entry names
it.

## Authority

- Server: everything; the queue is loaded and drained only on the authority.
- Client: nothing.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_Sha256` in `apps/mod/tbd-framework/Scripts/Game/TBD/Core/Hashing/`; `TBD_Log` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Core/`; `TBD_BackendText` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/API/Http/`; the engine's `FileIO` and `FileHandle`.
- Used by: `TBD_MatchRegistration`, `TBD_MatchResultsRevision` and `TBD_MatchEventBatch` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/API/MatchTelemetry/Reports/`; `TBD_TelemetryDelivery` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/API/MatchTelemetry/Delivery/`; `TBD_RuntimeStatusReadings`
  in `apps/mod/tbd-framework/Scripts/Game/TBD/API/RuntimeSession/`; `TBD_RuntimeHeartbeat` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Orchestrator/Heartbeat/`.
- Rules: file IO follows `TBD_MissionArtifactCache` (write, then check the size on disk); bytes are
  read with one linear `ReadArray`, never byte by byte from a string; hashing is synchronous, so a
  body stays small enough to hash within a frame; lines added stay ASCII and
  `cargo xtask mod compile` checks that the scripts compile.

## Related documentation

- [Match telemetry transport](/apps/mod/tbd-framework/Scripts/Game/TBD/API/MatchTelemetry/README.md) — how the
  queue, the reports and the delivery fit together
- [Match telemetry design](/documentation/crates/api/api_server/verification_evidence/telemetry.md) — the
  game-runtime telemetry queue section
