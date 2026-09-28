# Request middleware and authentication extractors

The layers every request of the [API](/documentation_v2/glossary/a_to_f.md#api) passes through
(correlation id, access log, CORS, rate limiting) and the extractors through which a handler
states who may call it.

## Contents

```text
apps/website/api_v2/src/core/middleware/
├── authentication.rs           `AuthUser` and the three role-gated extractors
├── authorized_event_stream.rs  re-authorizes a long-lived SSE stream; closes it at shutdown
├── client_identity.rs          the client address both rate-limit tiers key on
├── cross_origin.rs             `cors`: reflects an allow-listed `Origin`, answers preflights with 204
├── durable_ratelimit.rs        `PgRateLimiter`: token buckets in Postgres, shared across processes
├── mod.rs                      the module tree; re-exports, body caps, `role_rank` and `json_error`
├── rate_limiting.rs            `rate_limit`: an in-memory tier and, on strict prefixes, a Postgres one
├── tests/                      unit tests: client resolution, limiter placement, event stream shutdown
└── tracing_correlation.rs      `request_id` and `logging`: the `X-Request-ID` and the access log line
```

## How it works

`crate::core::http_router` applies the chain, outermost first: `request_id`, `logging`, the
metrics observer of `crate::core::observability`, panic recovery, `cors`, the default body limit
`MAX_JSON_BODY` (1 MiB), and `rate_limit`. The `/api/v1/cms/uploads` route raises its own limit to
`MAX_MULTIPART_BODY` (6 MiB), the [mission](/documentation_v2/glossary/g_to_m.md#mission) version save
route takes the limit that `MISSION_VERSION_MAX_BODY_BYTES` sets, and the ballistics catalog
upload `POST /api/v1/ballistics-catalogs` takes `MAX_CATALOG_UPLOAD_BODY_BYTES` (17 MiB + 64 KiB:
the `calibration` part's 16 MiB cap, the `catalog` part's 1 MiB cap and 64 KiB of multipart
framing), declared in `crate::operations::handlers::ballistics_catalogs::upload`, which also
answers 413 for a single part over its own cap.

```text
request ─▶ request_id ─▶ logging ─▶ metrics observer ─▶ panic recovery
        ─▶ cors ─▶ body limit ─▶ rate_limit ─▶ /api/v1, /healthz, /metrics, /uploads, the app
                              └─▶ /map-assets and /map-assets/glyphs, mounted below rate_limit
```

`request_id` keeps an inbound `X-Request-ID` or mints a UUID, and echoes it on the response;
`logging` writes one `access` line per request with that id, the method, path, status and
latency. `cors` adds the CORS headers only for an `Origin` in `ALLOWED_ORIGINS`, never `*` and
never with credentials, and answers every `OPTIONS` with 204; the allowed request headers are
`Authorization`, `Content-Type` and `X-Request-ID`.

`rate_limit` keys both of its tiers on one address from `client_identity`: the connection's peer,
or, when that peer is listed in `TRUSTED_PROXIES`, the rightmost `X-Forwarded-For` hop that is not
a trusted proxy. An empty list ignores the header, and a chain that cannot be read falls back to
the peer. The in-memory tier (`IpLimiter`) allows 20 requests a second with a burst of 40, and 1 a
second with a burst of 10 on the one `STRICT_PREFIXES` entry, `/api/v1/auth/`, the only
unauthenticated family. That prefix also passes the durable tier, `PgRateLimiter`, with the same strict numbers in the
`rate_limit_buckets` table, so a restart or a second process grants no fresh bucket. A refusal is
`429` with `Retry-After` and `{"error": "rate limit exceeded"}`; a durable tier that cannot reach
Postgres answers `503` instead of letting the request through. `/map-assets` and
`/map-assets/glyphs` are mounted below the layer and never reach it; `/uploads` stays limited. The
`ratelimit_cleanup_worker` [background worker](/documentation_v2/glossary/a_to_f.md#background-workers)
deletes buckets idle for an hour. `/api/v1/game-runtime/` and `/api/v1/ingest/` stay on the global
tier: every caller there is a game server with its own machine credential, several servers can share
one host address, and a mission start or an event batch arrives in a burst.

Authentication is not a layer. A handler takes an extractor, and the tier travels with it:
`AuthUser` needs `Authorization: Bearer <token>`, verifies the token and asks the session
authority in `AppState` for the member's current session (401 when either fails); `LeaderUser`,
`MissionMakerUser` and `AdminUser` also need a `role_rank` at least that of their
[role](/documentation_v2/glossary/n_to_z.md#role) (403 otherwise). The ranks run `guest` 0, `enlisted`
1, `leader` 2, `mission_maker` 3, `admin` 4, and an unknown role ranks below `guest`. The other
callers' extractors live with the code that owns them: `MachineCaller` for game hosts in
`server_infrastructure`, and `ObservabilityAuth` for the operator's scraper in
`crate::core::observability`. `authorize_event_stream` wraps an [SSE](/documentation_v2/glossary/n_to_z.md#sse)
stream: before each delivery, and at least every five seconds, it asks the session authority
again, and it ends the stream with an `authorization_expired` SSE event as soon as the session or
its role stops qualifying. It also races the stream against
`crate::core::process_lifecycle::process_shutdown`: once the `api` binary begins shutting down,
the body ends with no further event, ahead of a ready delivery and without waiting for a
re-authorization in flight, so the graceful drain never waits on an open stream and the client
reconnects with `Last-Event-ID` as after any end of stream.

## Boundaries

- Depends on: `crate::core::authentication_primitives` (the token manager, the session authority
  trait), `crate::core::configuration` (`TRUSTED_PROXIES`), `crate::core::application_state`,
  `crate::core::process_lifecycle` (the shutdown the event streams end on), `governor` for the
  in-memory buckets, and the `rate_limit_buckets` table of migration `0021`.
- Used by:
  - `crate::core::http_router`, which mounts the chain and the exempt asset mounts;
  - the handlers of all eight domains, through the extractors and `json_error`; `role_rank` in the
    mission write lock, [mission deployments](/documentation_v2/glossary/g_to_m.md#mission-deployment), the
    approval queue, reservation authority,
    [fleet commands](/documentation_v2/glossary/a_to_f.md#fleet-command) and the server intel's
    inactive-server scoping; `MAX_MULTIPART_BODY` in the `community_content` route table;
  - `crate::core::observability::observability_auth`, through `json_error`;
  - `authorize_event_stream`, in the audit log feed of `administration` and the server status
    stream of `server_infrastructure`;
  - the `ratelimit_cleanup_worker` background worker, through `PgRateLimiter`;
  - integration suites under `apps/website/api_v2/tests/`, among them `http_middleware.rs`,
    `durable_rate_limit.rs`, `forwarded_for_trust.rs`, `map_assets_rate_limit_exemption.rs` and
    `audit_replay_shutdown.rs`.
- Rules:
  - `STRICT_PREFIXES` is the only path test in the limiter; the asset exemption is where the
    router mounts them, below the layer (`the_exempt_mount_is_registered_below_the_rate_limit_layer`
    in `tests/rate_limiting.rs`, and `apps/website/api_v2/tests/map_assets_rate_limit_exemption.rs`);
  - the durable tier fails closed, and its numbers equal the in-memory strict tier's
    (`durable_strict_policy_matches_the_in_memory_strict_policy`);
  - `RATE_LIMIT_BUCKETS_DDL` is migration `0021` verbatim
    (`migration_0021_is_the_ddl_constant_verbatim` in
    `apps/website/api_v2/tests/durable_rate_limit.rs`);
  - `mission_maker` outranks `leader` in `role_rank` on purpose;
  - every SSE handler passes its stream through `authorize_event_stream`, the one place a stream
    ends at shutdown (`shutdown_closes_an_idle_stream_without_an_event` and its neighbours in
    `tests/authorized_event_stream.rs`).

## Related documentation

- [Identity transactions](/documentation_v2/website/api_v2/verification_evidence/identity_transactions.md)
  — how a bearer token's persisted session supplies the caller's current authority.
