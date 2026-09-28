# Match telemetry handlers

The endpoints a game server posts to (the live-status heartbeat, the match registration, the
results revision and the detailed event batch) and the signed-in read of a match's events, with the
heartbeat's wire contract. Each handler checks the caller, decodes the body and hands it to the
domain's models and services.

## Contents

```text
apps/website/api_v2/src/match_telemetry/handlers/
├── match_event_batches.rs        `POST /ingest/match-events`: one batch of a registered match's events
├── match_event_reads.rs          `GET /matches/{matchId}/events`: a page of events in `sequence` order
├── match_registration.rs         `POST /ingest/matches`: 201 for a new source match, 200 for a repeat
├── match_results.rs              `POST /ingest/match-results`: one results revision
├── mod.rs                        the module tree
├── server_heartbeat.rs           the session-fenced live-status heartbeat and its partial update
├── server_heartbeat_contract.rs  the heartbeat body, its telemetry queue block and their checks
└── tests/                        unit tests for the heartbeat
```

## How it works

- **Caller.** Every handler that writes takes a `MachineCaller` that must be a `mod_runtime`
  [machine credential](/documentation_v2/glossary/g_to_m.md#machine-credential); the server is the
  credential's, never a value in the body, and a body that names `server_id` is a 400.
- **Heartbeat.** `ingest_server_status` decodes `ServerStatusInput`
  (`server_heartbeat_contract.rs`, which refuses unknown keys). The body names the runtime
  session's generation and a sequence that strictly increases within the session, and
  `server_infrastructure::services::runtime_sessions::admit_heartbeat` fences it in the same
  transaction as the status write, so a stale runtime or a delayed message cannot overwrite newer
  state. Every reading is optional: an absent one keeps the stored value, `current_match_id` is
  cleared by an explicit `""` and must otherwise name a match of the caller's server, and a
  heartbeat with no reading at all is a 400. The optional `telemetry_queue` block is
  all-or-nothing (`backlog`, `capacity`, `dropped_total`, `oldest_age_seconds`, all at least 0,
  `backlog` at most `capacity`) and is stored with its `reported_at`; an absent block keeps the
  stored reading. The handler appends a `server_status_histories` sample, writes a
  `server.low_fps` warning when the server falls below 20 FPS, and publishes the stored row on the
  server's [SSE](/documentation_v2/glossary/n_to_z.md#sse) topic.
- **Ingests.** `ingest_match_registration`, `ingest_match_results` and `ingest_match_events` take
  the body as raw JSON, validate it whole through the decoders in `models/` (the first invalid entry
  is a 400 naming its index and field), and call the matching transaction in `services/`. The
  registration answers 201 for a new match and 200 for a repeat of the same body.
- **Event read.** `list_match_events` takes a signed-in `AuthUser`, pages the stored events after
  `after_sequence` (default 0) with `limit` from 1 to 500 (default 100), answers
  `{items, next_after_sequence}` with `next_after_sequence` null on the last page, and is a 404 for
  an unknown match.

## Boundaries

- Depends on: `server_infrastructure` (`MachineCaller`, `ExecutorKind`, the runtime-session fence,
  `publish_server_status_by_id`); `administration` (the low-FPS audit writer); the domain's
  models and services; `core` for errors and `AuthUser`.
- Used by: the domain's `routes.rs`; over HTTP, the
  [game runtime](/documentation_v2/glossary/g_to_m.md#game-runtime)'s session loop in
  `apps/mod/tbd-framework/Scripts/Game/TBD/API/RuntimeSession/TBD_RuntimeSession.c` and its results reporter in
  `apps/mod/tbd-framework/Scripts/Game/TBD/API/Results/TBD_ResultsReporter.c`. The reporter's
  registrations and results revisions and the recorded event batches reach the ingests through the
  durable queue that
  `apps/mod/tbd-framework/Scripts/Game/TBD/API/MatchTelemetry/Delivery/TBD_TelemetryDelivery.c`
  sends; no frontend page calls the event read.
- Rules: every handler carries its `/// @route` tag (`cargo xtask verify route-tags`); no handler
  imports another domain's handlers (`apps/website/api_v2/src/tests/architecture_rules.rs`); the
  three ingests validate in `models/` and write in `services/`, while the heartbeat keeps its fenced
  status upsert in its own transaction.
- Body decoding: every JSON body is read through `ApiError::from_json_rejection`: 413 with
  `details.code = request_too_large` over the body limit, 415 without a JSON content type, and 400
  with the decoder's message (which names the failing field) otherwise.
- Path decoding: every path segment is read through `core::http::path_parameters::PathParams`: a
  segment that does not decode into its type answers 400 in the `{error}` envelope with a message
  naming the parameter, never axum's plain-text rejection.
