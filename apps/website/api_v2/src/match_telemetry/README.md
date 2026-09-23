# `match_telemetry/`

The write half of the game-server channel: the live server-status heartbeat a running game runtime
posts within its runtime session, and the finished-match report it posts when a mission ends. The
heartbeat is authenticated by the server's `mod_runtime` machine credential and fenced by the
session's generation and a strictly increasing sequence; the match report is authenticated by the
shared service token and is idempotent — a re-ingest of the same `source_match_id` merges into the
existing row, re-derives attendance for the registrants of the old and new attachment, and
retracts what the previous ingest attributed.

Presenting these figures back to members is `command_center`; the server registry and its live SSE
topic are `server_infrastructure`.

## Public surface

- **`routes::routes()`** — the domain's `/api/v1` table, merged by
  `core::http_router::api_v1_routes` and nested under `/api/v1`. The literals in `routes.rs` are the
  public URLs: `/game-runtime/sessions/{sessionId}/heartbeats`, `/ingest/match-results`.
- **`models::match_record`** — `Match`, `MissionOutcome`, `MatchPlayerStat`: the stored record of a
  finished match, projected by the leaderboards and the deployment history.

The domain has no `services/` directory: the ingest work is transaction-shaped and runs inside the
handlers, which split it into a parsing module, a contract module, the match upsert, and the
attendance attribution so each file states one step.

## Dependency rules

- Handlers here never import another domain's handlers. `src/tests/architecture_rules.rs` enforces
  it across all eight domains.
- This domain imports `core`, `missions::models` (the mission a match played),
  `server_infrastructure::{models, services}` (the machine caller, the runtime-session fence, the
  server row and its status broadcast),
  `operations::services::participation_attribution` (finalized attendance and correction provenance),
  `command_center::services::user_stats` (the counters an ingest changes), and
  `administration::{models, services}` (the papertrail).
- Nothing in `core` imports it.

## Files

```text
mod.rs                                 Domain module tree; re-exports `routes`.
routes.rs                              The `/api/v1` route table for match telemetry.
handlers/
  mod.rs                               The two ingest endpoints and the steps they are composed from.
  ingest_parsing.rs                    Wire-value parsing and validation shared by both ingest handlers.
  match_results.rs                     The finished-match report: roster validation and the write.
  match_results_contract.rs            The wire contract of the match-results body and its per-player rows.
  match_upsert.rs                      Idempotent write of the `matches` row: find by `source_match_id` or create.
  server_heartbeat.rs                  The session-fenced live-status heartbeat and its partial update.
  tests/
    ingest_parsing.rs                  Sibling unit tests for `ingest_parsing.rs`.
    match_results.rs                   Sibling unit tests for `match_results.rs`.
    match_results_contract.rs          Sibling unit tests for `match_results_contract.rs`.
    match_upsert.rs                    Sibling unit tests for `match_upsert.rs`.
    server_heartbeat.rs                Sibling unit tests for `server_heartbeat.rs`.
models/
  mod.rs                               Match-telemetry database and wire models.
  match_record.rs                      The match row, its outcome enum, and the per-player statistics row.
```

Unit tests live in the sibling files above, declared from the production file as
`#[cfg(test)] #[path = "tests/<file>.rs"] mod tests;`.
