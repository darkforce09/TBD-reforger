**Status:** live

# Match telemetry, fleet status and derived statistics

Design for the telemetry requirements (`telemetry_match_identity`, `telemetry_telemetry_revisions`,
`telemetry_telemetry_corrections`, `telemetry_telemetry_atomicity`, `telemetry_detailed_events`,
`telemetry_telemetry_queue`) and the dashboard requirements (`dashboard_fleet_dashboard`,
`dashboard_statistics_recomputation`). It records the chosen semantics before implementation;
the tests it names hold it. The wire shapes are
`contracts/definitions/match-telemetry.schema.json` and the `RuntimeHeartbeat` definition of
`game-runtime-session.schema.json`.

## Authentication

Every ingest route takes the `MachineCaller` extractor and requires the `mod_runtime` executor kind
(machine_credentials.md). The server is the credential's server; a body that names `server_id`
answers 400. `POST /api/v1/ingest/link-confirm` authenticates the same way and its audit record names
the confirming server. `ServiceAuth`, `SERVICE_TOKEN` and the `X-Service-Token` header no longer exist.

`/metrics` and the detailed `/healthz` authenticate with `OBSERVABILITY_TOKEN`, sent as
`Authorization: Bearer <token>` and compared in constant time. Unset, `/metrics` answers 401 and
`/healthz` answers only its public `{status}` view; a missing or wrong bearer on `/healthz` also
yields the public view rather than an error. The token is an operator secret for scrapers and
nothing else accepts it.

All `/api/v1/ingest/*` routes use the global rate-limit tier: every caller is an authenticated game
server, several servers can share one host address, and event batches arrive in bursts.

Refusals a game runtime must act on are 409 answers carrying `details.code`, never 404: the mod
classifies 404 as permanent.

## Match identity

`POST /api/v1/ingest/matches` registers a match before any report about it:

```
{ source_match_id, runtime_session_id, started_at, mission_id?, event_id?, terrain? }
```

- `source_match_id` is 1–128 bytes after trimming and is scoped to the server: two servers may use
  the same string for two different matches.
- The runtime session must belong to the caller's server (403 otherwise; a session id naming no
  session is a 400). It may already have ended, because the mod's durable queue delivers registrations after a restart; the session is
  stored as provenance.
- The registration digest is the SHA-256 of the canonical JSON of the request body.
  - A new `(server, source)` answers 201 `{match_id, registered: true}` with a `pending` match at
    revision 0.
  - The same body again answers 200 `{match_id, registered: false}`.
  - A different body for a registered source answers 409 `REGISTRATION_CONFLICT` with `match_id`.
- Results or events for a source the caller's server has not registered answer 409
  `MATCH_NOT_REGISTERED`; nothing is written.
- `matches.server_id`, `registered_runtime_session_id` and `registration_sha256` never change once
  set (trigger `match_revision_guard`). Matches recorded before registration existed keep
  `server_id` NULL and stay readable; they accept no further reports.
- A heartbeat's `current_match_id` must name a match of the caller's server; another server's match
  answers 400.

## Results revisions

`POST /api/v1/ingest/match-results` carries one revision of the match report:

```
{ revision, match: { source_match_id, outcome, event_id?, mission_id?, terrain?, started_at?,
  ended_at?, winning_faction?, aar_replay_url? },
  players: [ { arma_id, role_played, source_event_id, counters? } ],
  removed_lines?: [ { arma_id, source_event_id } ] }
```

- Unknown keys are refused (`deny_unknown_fields`); flat top-level counter keys on a player line are
  unknown keys. `revision` is an integer ≥ 1.
- The report digest is the SHA-256 of the canonical JSON of the body without `revision`. Canonical
  JSON sorts object keys explicitly (serde_json's `preserve_order` feature is unified into the API
  build, so a `Map` keeps insertion order).
- The whole body is validated before the transaction opens. The first invalid entry answers 400
  `INVALID_MATCH_RESULTS` with `details {index, field}` (`index` is the position in `players` or
  `removed_lines`, absent for a match-level field); nothing is written.
- Under the match row lock the revision is decided:

| Stored state | Answer | Effect |
|---|---|---|
| no registration for `(server, source)` | 409 `MATCH_NOT_REGISTERED` | none |
| `revision < stored` | 409 `STALE_REVISION`, `details.revision` = stored | none |
| `revision == stored`, same digest | 200, `applied: false` | none (inert retry) |
| `revision == stored`, other digest | 409 `REVISION_CONFLICT`, `details {revision, report_sha256}` | none |
| `revision > stored`, match finalized, `outcome` = `pending` | 409 `MATCH_FINALIZED` | none |
| `revision > stored` otherwise | 200, `applied: true` | revision applied |

- Applying a revision:
  - match-level fields: a present field replaces the stored value, an absent one keeps it;
    `outcome` is always present, and a terminal outcome may be re-adjudicated to another terminal
    outcome; `finalized_at` is set once and never cleared;
  - a present player line replaces its row keyed by `(match_id, arma_id, source_event_id)`:
    `role_played` and the owning account always, the counters when `counters` is present (so a
    higher revision may lower them); a line without `counters` makes no counter claim and keeps the
    stored counters;
  - a line the revision omits is kept; `removed_lines` deletes the named rows (naming a row that
    does not exist is not an error; naming a line the same revision also reports is a 400);
  - `revision` and `report_sha256` are stored with the match.
- The answer is `{match_id, revision, applied, players, linked, unlinked, unlinked_arma_ids}`,
  where `linked + unlinked == players` counts the submitted lines. A duplicate computes the same
  split read-only.

## Detailed events

`POST /api/v1/ingest/match-events` carries a batch of 1–500 events of one registered match:

```
{ source_match_id, events: [ { event_id, sequence, kind, mission_time_ms, occurred_at, payload } ] }
```

- `event_id` matches `^[A-Za-z0-9._:-]{1,64}$` and is unique within the match; `sequence` is ≥ 1,
  unique within the match, and assigned by the mod in capture order. `sequence` is the order of
  every read.
- Kinds and payloads (`arma_id` values 1–128 bytes, prefab and weapon names 1–256 bytes):

| Kind | Payload | Actor / subject |
|---|---|---|
| `combat.kill` | `killer_arma_id`, `victim_arma_id?`, `victim_is_player`, `team_kill`, `distance_m ≥ 0`, `weapon?` | killer / victim |
| `combat.death` | `victim_arma_id`, `cause` ∈ `ai`, `environment`, `self`, `unknown` | — / victim |
| `medical.incapacitated` | `subject_arma_id` | — / subject |
| `medical.revived` | `subject_arma_id` | — / subject |
| `vehicle.destroyed` | `vehicle_prefab`, `instigator_arma_id?` | instigator / — |
| `vehicle.entered` | `arma_id`, `vehicle_prefab`, `compartment` | arma_id / — |
| `vehicle.exited` | `arma_id`, `vehicle_prefab`, `compartment` | arma_id / — |

- The TBD mod sends `compartment` as `pilot`, `turret`, `cargo` or `other` and `distance_m` in whole
  metres; the API accepts any 1–64 byte compartment name and any non-negative distance.
- Validation of the whole batch precedes the transaction: 400 `INVALID_EVENT` with
  `details {index, field}` for the first invalid event (unknown kind, malformed payload, duplicate
  `event_id` or `sequence` inside the batch), 400 `EVENT_BATCH_TOO_LARGE` above 500 events.
- The event digest is the SHA-256 of the canonical JSON of the event. Under the match row lock:
  - an `event_id` already stored with another digest answers 409 `EVENT_CONFLICT`, and a `sequence`
    held by another `event_id` answers 409 `EVENT_SEQUENCE_CONFLICT`, each with `details {index}`;
    nothing of the batch is written;
  - otherwise every event is inserted with `ON CONFLICT DO NOTHING`; only the rows actually
    inserted increment `match_event_totals` (per match, `arma_id`, kind and participant role) and
    `matches.event_count`, in the same transaction. A retry or an overlapping batch never counts an
    event twice.
- Events are accepted after the match is finalized (late delivery from the queue).
- The answer is `{match_id, accepted, duplicates, event_count, last_sequence}`.
- `GET /api/v1/matches/{matchId}/events?after_sequence=&limit=` (any signed-in user, `limit` ≤ 500,
  default 100) answers `{items, next_after_sequence}` in `sequence` order; `next_after_sequence` is
  null on the last page.
- Event totals are keyed by `arma_id` and joined to accounts when read, so link changes never
  rewrite them.

## Lock order

- Results: the `matches` row `FOR NO KEY UPDATE` (found by server and source) → revision decision →
  obligated registrants → identities (sorted) → accounts (sorted) → match and line writes →
  attendance reconciliation → unlinked-player audit → per-account statistics → leaderboard refresh
  (its advisory lock is always last).
- Events: the `matches` row → conflict probe → inserts → totals.
- Registration: session read → insert or compare.
- Link, unlink and deleted-owner release lock identities then accounts and never a match row, so no
  cycle exists with results.

## Game-runtime telemetry queue

The mod keeps every registration, results revision and event batch in a durable, bounded queue
under `$profile:TBD/Telemetry/` until the API acknowledges it.

- One file per entry holds the route, the body, the enqueue time and a trailer with the body length
  and SHA-256; a file that fails its trailer (a torn write) is removed and counted as dropped. Two
  alternating state slots hold the next entry id, the drop total and the per-match revision and
  event sequence counters.
- Capacity is 512 entries; 32 are reserved for registrations and results. When full, the oldest
  event batch is dropped (the oldest entry when only registrations and results remain), counted and
  logged at ERROR. A newer unsent results revision of a match replaces the older one in place.
- One request is in flight at a time, pumped by the game-mode heartbeat loop:

| Answer | Queue action |
|---|---|
| 2xx; 409 `STALE_REVISION` | acknowledged: the entry is deleted |
| 409 `MATCH_NOT_REGISTERED` | the match's registration is sent first, then the entry retries |
| 409 `REGISTRATION_CONFLICT`, `REVISION_CONFLICT`, `EVENT_CONFLICT`, `EVENT_SEQUENCE_CONFLICT`, `MATCH_FINALIZED`; 400 | dropped, counted, logged at ERROR |
| 401, 403 | kept; retried with backoff (a credential fix must still deliver) |
| transport failure, timeout, 429, 5xx | kept; retried with backoff (2 s doubling to 60 s) |

- Every heartbeat carries `telemetry_queue {backlog, capacity, dropped_total, oldest_age_seconds}`.
  The block is optional; when present all four are required, all are ≥ 0 and `backlog ≤ capacity`;
  an absent block keeps the stored reading. The API stores it on `server_statuses` with its
  `reported_at`, and `ServerStatus.telemetry_queue` carries it through the status read, the SSE
  stream, the scheduled publisher, Server Intel, the dashboard and Server Control. A server that
  never reported one has no `telemetry_queue` key.

## Fleet

The configured fleet is the set of servers with `is_active = true`.

- `GET /api/v1/dashboard` answers `fleet {servers: [{server_id, name, status?}], totals: {configured,
  online, players, max_players, telemetry_backlog, telemetry_dropped_total}}` over the active
  servers ordered by name then id, in place of the former single `server_status`. A server without
  a status row is listed with no status and counts as offline. `online`, `players` and
  `max_players` count online servers only; `telemetry_backlog` and `telemetry_dropped_total` sum
  every reported queue reading.
- The status publisher republishes active servers only. `GET /api/v1/servers` lists every server,
  with `is_active`, to administrators and only active servers to everyone else; the status read and
  the status stream of an inactive server answer 404 to non-administrators.

## Derived statistics

`users.total_deployments`, `users.attendance_rate` and `leaderboard_totals` are recomputed inside
the transaction of every change that affects them: an applied results revision, link confirmation,
unlink, relink and deleted-owner release. Duplicate and stale revisions change no facts and skip the
recomputation. A reader therefore never observes a committed fact without its derived statistics.

## Tests

`tests/match_identity.rs` (`match_identity_*`), `tests/telemetry_revisions.rs`
(`telemetry_revisions_*`), `tests/telemetry_corrections.rs` (`telemetry_corrections_*`),
`tests/telemetry_atomicity.rs` (`telemetry_atomicity_*`), `tests/detailed_events.rs`
(`detailed_events_*`), `tests/telemetry_queue.rs` (`telemetry_queue_*`), `tests/fleet_dashboard.rs`
(`fleet_dashboard_*`), `tests/statistics_recomputation.rs` (`statistics_recomputation_*`), the
observability cases in `tests/observability.rs`, and the unit tests beside
`match_telemetry/services/`.
