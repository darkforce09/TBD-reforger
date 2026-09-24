# Match telemetry domain

The [API](/documentation_v2/glossary.md#api)'s
[match telemetry](/documentation_v2/glossary.md#match-telemetry) domain: the write half of the
game-server channel. A running [game runtime](/documentation_v2/glossary.md#game-runtime) posts its
live server status as heartbeats within its runtime session, and posts a finished-match report when
a [mission](/documentation_v2/glossary.md#mission) ends. Presenting these figures back to members
belongs to `command_center`; the server [registry](/documentation_v2/glossary.md#registry), the
runtime sessions and the live status topic belong to `server_infrastructure`.

## Contents

```text
apps/website/api_v2/src/match_telemetry/
├── handlers/  the heartbeat and match-results ingests, their wire contract, parsing and match upsert
├── mod.rs     the module tree; re-exports `routes`
├── models/    the stored match, its outcome and the per-player lines
├── routes.rs  the domain's `/api/v1` route table
└── services/  the source-match guard that serialises reports of one match
```

## How it works

A heartbeat is authenticated by the server's `mod_runtime`
[machine credential](/documentation_v2/glossary.md#machine-credential) and fenced by the runtime
session's generation and a sequence that strictly increases within the session; the fence, the
partial status update and the history sample commit together, a low-FPS warning follows
best-effort, and the stored row is then published on the server's
[SSE](/documentation_v2/glossary.md#sse) topic.

A match report is authenticated by the shared service token and is idempotent: a report that
repeats a `source_match_id` merges into the stored match, re-derives attendance for the
registrants of the old and the new [event](/documentation_v2/glossary.md#event) mission, and
retracts what the previous report attributed. The whole report (roster checks, player lines,
attendance attribution, statistics and leaderboard recomputation, audit) commits in one transaction,
behind the source-match guard and the shared identity and account lock order.

## Public surface

- `routes::routes()`: the table `core::http_router` merges under `/api/v1`, one route each:
  - `POST /api/v1/game-runtime/sessions/{sessionId}/heartbeats`: `mod_runtime` machine
    credential; the session-fenced live status.
  - `POST /api/v1/ingest/match-results`: `ServiceAuth` (`X-Service-Token`); the finished-match
    report.
- `models::match_record`: `Match`, `MissionOutcome` and `MatchPlayerStat`, read by the member
  [service record](/documentation_v2/glossary.md#service-record) in `operations`.

## Boundaries

- Depends on:
  - `core`: the application state, errors, the `ServiceAuth` extractor, the URL guard, the
    Postgres error codes and the wire formats;
  - `server_infrastructure` for the machine caller, the runtime-session fence, the server status
    row and its publisher; `identity_and_access` for the identity and account locks;
    `operations::services::participation_attribution` for attendance; `command_center::services`
    for the statistics and the leaderboard; `administration` for the audit writers;
    `missions::models` for the terrain a match played.
- Used by:
  - `core::http_router`, which merges the route table, and `operations`, through the match
    models;
  - over HTTP, the game runtime in `apps/mod/tbd-framework/Scripts/Game/TBD/API/`: the runtime
    session loop posts heartbeats and the results reporter posts match reports.
- Rules: handlers never import another domain's handlers, and `routes.rs` exports the table the
  router merges (`apps/website/api_v2/src/tests/architecture_rules.rs` checks both); every handler
  carries its `/// @route` tag (`cargo xtask verify route-tags`).

## Related documentation

- [API overview](/documentation_v2/website/api_v2/api_overview.md) — every domain's routes.
- [Machine credentials and runtime sessions](/documentation_v2/website/api_v2/verification_evidence/machine_credentials.md)
  — the credential and the session fence a heartbeat passes.
- [Reservation and attendance separation](/documentation_v2/website/api_v2/verification_evidence/reservation_attendance.md)
  — how a match report attributes and corrects attendance.
- [Identity transactions](/documentation_v2/website/api_v2/verification_evidence/identity_transactions.md)
  — the lock order and gameplay attribution a report shares with identity linking.
