# API match telemetry

The `api_match_telemetry` crate: the [API](/documentation/glossary/a_to_f.md#api)'s
[match telemetry](/documentation/glossary/g_to_m.md#match-telemetry) domain. It holds the write
half of the game-server channel and the read of a match's detailed events: the session-fenced
live status heartbeat a [game runtime](/documentation/glossary/g_to_m.md#game-runtime) posts, the
registration of each match it plays, the numbered results revisions of a match's report and the
batches of detailed combat, medical and vehicle events, with the `/api/v1` route table the API's
router merges.

## Contents

```text
crates/api/api_match_telemetry/
├── Cargo.toml  the package: `api_state`, `api_caller_identity`, `api_member_activity`, `api_server_infrastructure`, `api_http_layer`, `api_audit_log`, `api_failpoints`, `fleet_wire_contract`, sqlx (`postgres`), axum, layout tier 8
└── src/        the route table, the handlers, the registration, results-revision and event-batch services, the models, the error and the prelude
```

## How it works

Every write is authenticated by the server's `mod_runtime`
[machine credential](/documentation/glossary/g_to_m.md#machine-credential), which names the
server; a body never does. A heartbeat passes the runtime-session fence of
`api_server_infrastructure` and its stored status row is published on the server's topic. A match
is registered once per runtime source id; its results arrive as whole numbered revisions applied
under the match row lock, each recomputing attendance, the member statistics and the leaderboard
through `api_member_activity` in the same transaction. Detailed events are stored idempotently by
their runtime ids. A refusal the runtime must act on is a 400 or a 409 carrying `details.code`.
The source tree README has the detail.

## Getting started

Run from the repository root:

```bash
cargo test -p api_match_telemetry
cargo clippy -p api_match_telemetry --all-targets -- -D warnings
cargo xtask db test-it --test telemetry_revisions --test telemetry_queue --test detailed_events --test route_acceptance_fleet_and_telemetry
```

The unit tests cover the wire decoders and their refusals, the revision decision, the shared
ingest parsers and the source pins of the heartbeat and the results transaction; the transactions
and the routes are proved against Postgres by the API's integration suites.

## Configuration

No feature and no variable of its own. The `fail_point!` sites around the results revision commit
compile to nothing outside test builds (`api_failpoints`).

## Public surface

- `routes()`: the domain's `/api/v1` route table.
- `handlers`: the heartbeat, the match registration, the results revision, the event batch and
  the event read, each with its `/// @route` tag, and the heartbeat's wire input.
- `services`: `register_match`, `ingest_results_revision`, `ingest_event_batch`, the registered
  match lock, the revision decision and writes, and the shared ingest parsers.
- `models`: `Match`, `MissionOutcome` and `MatchPlayerStat`, the registration, results revision
  and event batch wire shapes with their decoders and answers, the event page and the refusals.
- `Error` and `Result` (the shape failures the decoders share, converting into `ApiError`), and
  `prelude`.

## Boundaries

- Depends on: `api_state`, `api_caller_identity`, `api_member_activity`,
  `api_server_infrastructure` (the runtime-session fence, the status row and its publisher),
  `api_http_layer`, `api_audit_log`, `api_database`, `api_failpoints`, `api_foundation`,
  `api_identifiers`, `api_mission_vocabulary`, `fleet_wire_contract`, `http_url_guard`, sqlx,
  axum, serde, chrono, thiserror and uuid. It names no other domain.
- Used by: the API application (`crates/api/api_server`): its router merges `routes`, the operations domain
  reads the match models for the member service record, and the integration suites call the
  ingest services. Over HTTP: the game runtime's session loop and telemetry delivery in
  `apps/mod/tbd-framework/Scripts/Game/TBD/API/`.
- Rules: the API crate rules of [crates/api](/crates/api/README.md);
  `crates/api/api_server/src/tests/architecture_rules.rs` checks its route table, its handlers and its
  imports against the domain graph.

## Related documentation

- [API match telemetry source](/crates/api/api_match_telemetry/src/README.md) — the files, the
  routes and how each ingest is decided.
- [Match telemetry, fleet status and derived statistics](/documentation/crates/api/api_server/verification_evidence/telemetry.md)
  — registration, revisions, detailed events, the lock order and the game runtime's queue.
- [API overview](/documentation/crates/api/api_server/api_overview.md) — every domain's routes.
- [API crates](/crates/api/README.md) — the category this crate belongs to and its rules.
