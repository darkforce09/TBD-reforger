# Match telemetry domain

The [API](/documentation/glossary/a_to_f.md#api)'s
[match telemetry](/documentation/glossary/g_to_m.md#match-telemetry) domain: the write half of the
game-server channel and the read of a match's detailed events. A running
[game runtime](/documentation/glossary/g_to_m.md#game-runtime) posts its live server status as
heartbeats within its runtime session, registers each match it plays, and reports the match as
numbered results revisions and batches of detailed combat, medical and vehicle events. Presenting
these figures back to members belongs to `api_command_center`; the server
[registry](/documentation/glossary/n_to_z.md#registry), the runtime sessions and the live status
topic belong to `api_server_infrastructure`.

## Contents

```text
crates/api/api_match_telemetry/src/
├── error.rs    `Error` and `Result`: the shape failures the wire decoders share, converting into `ApiError`
├── handlers/   the heartbeat, the three ingests and the event read, with the heartbeat's wire contract
├── lib.rs      the crate root; re-exports `routes`, `Error` and `Result`
├── models/     the stored match, the ingest wire shapes and their validation, the refusals
├── prelude.rs  the match models and the registration and results ingest services
├── routes.rs   the domain's `/api/v1` route table
└── services/   the registration, results-revision and event-batch transactions, the match row lock
```

## How it works

Every write is authenticated by the server's `mod_runtime`
[machine credential](/documentation/glossary/g_to_m.md#machine-credential); the server is the
credential's, and a body that names `server_id` is a 400.

- **Heartbeat.** Fenced by the runtime session's generation and a sequence that strictly increases
  within the session; the fence, the partial status update (the telemetry queue reading included)
  and the history sample commit together, a low-FPS warning follows best-effort, and the stored row
  is then published on the server's [SSE](/documentation/glossary/n_to_z.md#sse) topic.
- **Registration.** `POST /ingest/matches` creates a `pending` match at revision 0 for a
  `(server, source_match_id)` pair, or answers the existing one when the same body repeats; a
  different body is a 409 `REGISTRATION_CONFLICT`.
- **Results revisions.** `POST /ingest/match-results` carries a whole revision of the report. The
  body is validated before the transaction; under the match row lock a strictly higher revision is
  applied, the same revision is inert with the same digest and a conflict with another, and a lower
  one is stale. An applied revision writes the match and its player lines, reconciles attendance for
  the registrants of the [event](/documentation/glossary/a_to_f.md#event)
  [mission](/documentation/glossary/g_to_m.md#mission), audits unlinked identities and recomputes
  the statistics and the leaderboard in the same transaction.
- **Detailed events.** `POST /ingest/match-events` stores a batch of 1–500 events idempotently and
  counts only the rows it inserts into the per-identity totals and the match's event count;
  `GET /matches/{matchId}/events` pages them in `sequence` order.

A refusal the runtime must act on is a 400 or a 409 carrying `details.code`, never a 404, which the
mod treats as permanent. Reports about a source the server has not registered are the 409
`MATCH_NOT_REGISTERED`, which makes the mod's durable queue send the registration first.

## Public surface

- `routes::routes()`: the table the API's router (`crates/api/api_server/src/router.rs`) merges
  under `/api/v1`:
  - `POST /api/v1/game-runtime/sessions/{sessionId}/heartbeats`: `mod_runtime` machine
    credential; the session-fenced live status.
  - `POST /api/v1/ingest/matches`: `mod_runtime` machine credential; the match registration.
  - `POST /api/v1/ingest/match-results`: `mod_runtime` machine credential; one results revision.
  - `POST /api/v1/ingest/match-events`: `mod_runtime` machine credential; a detailed event batch.
  - `GET /api/v1/matches/{matchId}/events`: any signed-in user; a page of the match's events.
- `models::match_record`: `Match`, `MissionOutcome` and `MatchPlayerStat`, read by the member
  [service record](/documentation/glossary/n_to_z.md#service-record) in `api_operations`.
- `Error` and `Result` (`error.rs`): the shape failures the wire decoders share, each turned into
  the decoder's coded refusal; `prelude` re-exports the match models and the registration and
  results ingest services.
- `contract_schema_types::match_telemetry::match_telemetry` (in the `contract_schema_types`
  crate): the types generated from `contracts/definitions/match-telemetry.schema.json`, which the
  integration tests deserialize live answers into.

## Boundaries

- Depends on:
  - `api_state` for the application state; `api_foundation` for the errors, the canonical JSON
    digest in `api_foundation::wire_format::content_digest` and the wire formats;
    `api_http_layer` for the `AuthUser` extractor; `api_database` for the Postgres error codes;
    `http_url_guard` for the URL guard;
  - `api_server_infrastructure`, the one domain it names, for the runtime-session fence, the
    server status row and its publisher;
  - the API crates: `api_caller_identity` for the machine caller and the identity and account locks;
    `api_member_activity` for attendance attribution, the statistics and the leaderboard;
    `api_audit_log` for the audit writers; `api_mission_vocabulary` for the terrain a match played.
- Used by:
  - the API's router (`crates/api/api_server/src/router.rs`), which merges the route table, and
    `api_operations`, through the match models;
  - over HTTP, the game runtime in `mod/tbd-framework/Scripts/Game/TBD/API/`: the runtime
    session loop posts heartbeats, and the telemetry delivery in `MatchTelemetry/Delivery/` posts
    the registrations, results revisions and event batches its durable queue holds.
- Rules: handlers never import another domain's handlers, and `routes.rs` exports the table the
  router merges (`crates/api/api_server/src/tests/architecture_rules.rs` checks both); every handler
  carries its `/// @route` tag (`cargo xtask verify route-tags`); every wire model carries its
  `@contract` tag into `match-telemetry.schema.json` (`cargo xtask schema citations`); the
  generated types are regenerated, never edited (`cargo xtask ci schema-codegen`).

## Related documentation

- [Match telemetry, fleet status and derived statistics](/documentation/crates/api/api_server/verification_evidence/telemetry.md)
  — registration, revisions, detailed events, the lock order and the game runtime's queue.
- [API overview](/documentation/crates/api/api_server/api_overview.md) — every domain's routes.
- [Machine credentials and runtime sessions](/documentation/crates/api/api_server/verification_evidence/machine_credentials.md)
  — the credential every ingest requires and the session fence a heartbeat passes.
- [Reservation and attendance separation](/documentation/crates/api/api_server/verification_evidence/reservation_attendance.md)
  — how a match report attributes and corrects attendance.
- [Identity transactions](/documentation/crates/api/api_server/verification_evidence/identity_transactions.md)
  — the lock order and gameplay attribution a report shares with identity linking.
