# Metrics and health probe

Everything that reports on the running [API](/documentation/glossary/a_to_f.md#api): the Prometheus
metrics every request feeds and `GET /metrics` exposes, and the `GET /healthz` probe that
preflight checks and gates read.

## Contents

```text
crates/api/api_http_layer/src/observability/
├── health_probe.rs        `healthz`: the database and migration checks behind `GET /healthz`
├── metrics_exposition.rs  `metrics_scrape`: the Prometheus 0.0.4 text served at `GET /metrics`
├── metrics_registry.rs    `Registry`: the metric families, their labels and the series cap; the Discord outcome counts
├── mod.rs                 the module tree
├── observability_auth.rs  `ObservabilityAuth` and the `OBSERVABILITY_TOKEN` bearer check
├── request_observer.rs    `observe`: the middleware that counts every request into the registry
└── tests/                 unit tests for the bearer check
```

## How it works

The metrics live in one `Registry` per application state: `AppState::new` creates it as
`AppState::metrics_registry`, so no metric is process-global state, and the API's router (`api_server::router`)
shares it with `observe`, `/metrics` and `/healthz`. `observe` sits inside the
access log but outside panic recovery and the rate limiter, so a panic's `500` and a throttle's
`429` are both counted. Its `route` label is the matched route template (`/api/v1/missions/{id}`,
never one series per id), `<unmatched>` for a request no route matched, and each family holds at
most `Registry::MAX_SERIES` (1024) series; a sample beyond that is dropped and counted in
`tbd_metrics_series_dropped_total`.

`GET /metrics` takes the `ObservabilityAuth` extractor: it needs
`Authorization: Bearer <OBSERVABILITY_TOKEN>`, compared in constant time, and answers 401 without
it or while `OBSERVABILITY_TOKEN` is unset. The token is an operator secret for scrapers; no user
session or machine credential is accepted here, and no other route accepts the token. It renders the request counters, the latency histogram, the
in-flight gauge, `tbd_http_rate_limited_total`, the build and the uptime, plus what the metrics
registry cannot accumulate and the scrape samples itself: a bounded database ping and the pool's
connections.

The one family recorded outside the request path is `tbd_discord_reconcile_outcomes_total{outcome}`:
the Discord membership reconciliation worker receives the application state and counts each
request into its registry with `Registry::record_discord_reconcile_outcome`. Its five series are always
present, zeros included: `member` and `nonmember` (the answer was recorded), `lease_lost` (the
answer arrived after the lease lapsed and was discarded), `rate_limited` (Discord answered 429)
and `unavailable` (no usable answer: a transport failure, a non-success status or a malformed
body; the membership stays and `last_error` names the failure). Each counted request also writes
one `info` line with target `discord_reconciliation` and the fields `outcome`, `guild_scope`
(`main` or `partner`), `retry_after_ms` (failures only) and `revision`; it names no account, guild
id or token. The Discord client reads `HTTPS_PROXY` when it is built, so a proxy on a closed port
turns every request into `unavailable`.

`GET /healthz` runs two checks, either of which turns it red: `database`, a `SELECT 1` bounded at
2 s, and `migrations`, which reads `_sqlx_migrations` and fails when the table is unreadable or
records a failed migration. The answer is 200 with `{"status": "ok"}` or 503 with
`{"status": "unavailable"}`. The route needs no credentials, because probes call it bare; only a
caller for whom `observability_bearer_matches` holds (the same bearer) gets the full report, and a
missing or wrong bearer yields the public answer rather than an error: the version, the uptime,
each check's status, latency and error, the migration counts and the pool gauges.

## Boundaries

- Depends on: `api_configuration::configuration` (`observability_token`),
  `crate::authentication_primitives` (`constant_time_equal`) and `crate::middleware`
  (`json_error`) for the bearer check; the `axum` matched-path extension; the Postgres pool and its
  `_sqlx_migrations` table.
- Used by:
  - `api_state`'s `AppState`, which owns the `Registry`;
  - the API's router (`api_server::router`), which mounts `observe` and serves `/metrics` and `/healthz`;
  - `api_identity_and_access::services::discord_rest_reconciliation`, which counts each
    Discord membership request into the application state's `Registry`;
  - the integration suite `crates/api/api_server/tests/http_infrastructure/observability.rs`;
  - over HTTP: the Caddy site in `deploy/caddy/Caddyfile` publishes
    `/healthz`, and the ticket manager's preflight, the `editor-api-boot` task of
    `cargo xtask ci` and the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)
    smoke gates in `tools/browser_testing/browser_gate_suites/src/editor_smoke_tests/` probe it
    without credentials.
- Rules: `observe` stays outside panic recovery and the rate limiter; `/healthz` discloses nothing
  beyond `status` without the token (`healthz_discloses_nothing_to_an_unauthenticated_caller` in
  `crates/api/api_server/src/tests/router.rs`); the
  `route` label is always a template, never a raw path.
