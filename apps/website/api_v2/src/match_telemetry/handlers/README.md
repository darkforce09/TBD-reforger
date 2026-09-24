# Match telemetry handlers

The two ingest endpoints a game server posts to, the live-status heartbeat and the finished-match
report, with the wire contract, parsing and match upsert they are built from.

## Contents

```text
apps/website/api_v2/src/match_telemetry/handlers/
├── ingest_parsing.rs          wire-value parsing for both ingests; foreign-key errors mapped to 400
├── match_results.rs           the match report: roster checks, facts, attribution, recomputation
├── match_results_contract.rs  the wire contract of the match-results body and its per-player lines
├── match_upsert.rs            the idempotent write of the `matches` row, keyed by `source_match_id`
├── mod.rs                     the module tree
├── server_heartbeat.rs        the session-fenced live-status heartbeat and its partial update
└── tests/                     unit tests for the parsing, the contract, the upsert and both ingests
```

## How it works

- **Heartbeat.** `ingest_server_status` takes a `MachineCaller` that must be a `mod_runtime`
  [machine credential](/documentation_v2/glossary.md#machine-credential); the server is the
  credential's, never a value in the body. The body names the runtime session's generation and a
  sequence that strictly increases within the session, and
  `server_infrastructure::services::runtime_sessions::admit_heartbeat` fences it in the same
  transaction as the status write, so a stale runtime or a delayed message cannot overwrite newer
  state. Every reading is optional: an absent one keeps the stored value, `current_match_id` is
  cleared by an explicit `""`, and a heartbeat with no reading at all is a 400. The handler appends
  a `server_status_histories` sample, writes a `server.low_fps` warning when the server falls below
  20 FPS, and publishes the stored row on the server's [SSE](/documentation_v2/glossary.md#sse)
  topic.
- **Match results.** `ingest_match_results` takes `ServiceAuth` (`X-Service-Token`). It validates
  the complete roster, serialises reports of the same match, locks the identities and accounts in
  the shared order, upserts the match by `source_match_id` (`match_upsert.rs`), stores each
  player's line, attributes attendance through `operations::services::participation_attribution`,
  retracts what a previous report attributed to another
  [event mission](/documentation_v2/glossary.md#event), and recomputes the affected statistics and
  the leaderboard before it commits. Players whose Arma identity no account owns keep their gameplay
  facts and are listed in the answer and in a system audit row.
- **Contract.** `outcome` is required; every other match field is optional, where absent keeps the
  stored value and present replaces it, and an unparseable non-empty `event_id` or `mission_id` is a
  400.

## Boundaries

- Depends on: `server_infrastructure` (`MachineCaller`, `ExecutorKind`, `ServerStatus`, the
  runtime-session fence, `publish_server_status`); `identity_and_access` (`lock_identities`,
  `lock_accounts`); `operations::services::participation_attribution`; `command_center::services`
  (`recompute_user_stats_on_connection`, `refresh_leaderboard_on_connection`); `administration`
  (audit writers); `missions::models::mission::TerrainType`; the domain's models and
  `services::result_serialization`; `core` for errors, the URL guard and the Postgres error codes.
- Used by: the domain's `routes.rs`; over HTTP, the
  [game runtime](/documentation_v2/glossary.md#game-runtime)'s session loop in
  `apps/mod/tbd-framework/Scripts/Game/TBD/API/TBD_RuntimeSession.c` and its results reporter in
  `apps/mod/tbd-framework/Scripts/Game/TBD/API/TBD_ResultsReporter.c`.
- Rules: every handler carries its `/// @route` tag (`cargo xtask verify route-tags`); no handler
  imports another domain's handlers (`apps/website/api_v2/src/tests/architecture_rules.rs`); a
  report of the same `source_match_id` merges into the stored match rather than adding one.
