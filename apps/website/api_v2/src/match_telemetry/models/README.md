# Match telemetry models

The stored record of a match (the match row, its outcome enum `success`, `failure`, `aborted` or
`pending`, and the per-player lines) and the wire shapes of the machine-authenticated ingests: the
match registration, the results revision, the detailed event batch, the event read page and the
refusals a game runtime classifies by code. Keys are snake_case, absent values are skipped and
timestamps are RFC 3339.

## Contents

```text
apps/website/api_v2/src/match_telemetry/models/
├── generated/                 types generated from `match-telemetry.schema.json`
├── match_event.rs             the event batch, the seven event kinds and their payloads, the digest
├── match_event_page.rs        `MatchEventPage`, one page of stored events in `sequence` order
├── match_record.rs            `Match`, its `MissionOutcome` and the per-player `MatchPlayerStat` lines
├── match_registration.rs      `MatchRegistration`, its answer and the `source_match_id` rule
├── match_results_revision.rs  `MatchResultsRevision`: report, player lines, counters, removed lines
├── mod.rs                     the module tree; re-exports the three stored types
├── telemetry_refusal.rs       the 400 and 409 refusals carrying `details.code`, `index` and `field`
└── tests/                     unit tests for the event, results-revision and refusal decoders
```

## How it works

Each ingest body arrives as raw JSON and its decoder (`decode_registration`,
`decode_results_revision`, `decode_event_batch`) validates it whole before any transaction opens.
Unknown keys are refused at every level, `server_id` included, since the machine credential names
the server. The first invalid entry becomes a 400 built by `telemetry_refusal.rs`, with
`details.code` (`INVALID_MATCH_REGISTRATION`, `INVALID_MATCH_RESULTS`, `INVALID_EVENT` or
`EVENT_BATCH_TOO_LARGE`), the entry's `index` in the submitted array and the offending `field`; the
same file builds the 409 conflicts the services answer.

Each decoded value carries the SHA-256 digest of its canonical JSON
(`core::wire_format::content_digest`): the registration's over the whole body, the results
revision's over the body without `revision`, and each event's over the event object as sent. The
services compare these digests to tell an inert retry from a conflict.

- **Registration.** `source_match_id` is 1–128 bytes after trimming; `runtime_session_id` and
  `started_at` are required, `mission_id`, `event_id` and `terrain` optional.
- **Results revision.** `revision` is at least 1; `match.outcome` is required and every other
  match field optional; a player line names `arma_id`, `role_played` and `source_event_id`, with
  an optional nested `counters` block; `removed_lines` names rows to delete. No two lines, and no
  line and removed line, share a key.
- **Event batch.** 1–`MAX_EVENTS_PER_BATCH` (500) events, each with an `event_id` matching
  `^[A-Za-z0-9._:-]{1,64}$`, a `sequence` of at least 1, both unique within the batch, and one of
  `combat.kill`, `combat.death`, `medical.incapacitated`, `medical.revived`, `vehicle.destroyed`,
  `vehicle.entered` and `vehicle.exited` with its payload. `EventBody` derives each event's actor
  and subject identities for the per-identity totals.

## Boundaries

- Depends on: `core::wire_format` for timestamps and the canonical digest, `core::text` for the URL
  guard on `aar_replay_url`, `core::error_handling` for the refusals;
  `missions::models::mission::TerrainType`; serde and sqlx. `generated/` follows
  `contracts_v2/definitions/match-telemetry.schema.json`.
- Used by: the domain's handlers and services; the member's
  [service record](/documentation_v2/glossary/n_to_z.md#service-record) in
  `apps/website/api_v2/src/operations/handlers/member_service_record.rs` (`Match`,
  `MatchPlayerStat`); the integration tests in `apps/website/api_v2/tests/`
  (`telemetry_revisions.rs`, `detailed_events.rs` and `telemetry_queue.rs` decode live answers
  into the generated types; the null-tolerance tests read `Match`).
- Rules: every wire type carries its `@contract` tag into `match-telemetry.schema.json`
  (`cargo xtask schema citations`); `generated/` is written by `cargo xtask ci schema-codegen` and
  never edited by hand (`cargo xtask ci verify-codegen-fresh` checks it); `Match.winning_faction`,
  `aar_replay_url` and `created_at` are plain fields over nullable columns, so every query that reads
  them coalesces them; `MissionOutcome` and the `mission_outcome` enum in the migrations change
  together; a refusal the runtime must act on is never a 404.
