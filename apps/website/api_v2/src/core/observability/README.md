# Metrics and health probe

Everything that reports on the running [API](/documentation_v2/glossary/a_to_f.md#api): the Prometheus
metrics every request feeds and `GET /metrics` exposes, and the `GET /healthz` probe that
preflight checks and gates read.

## Contents

```text
apps/website/api_v2/src/core/observability/
├── health_probe.rs        `healthz`: the database and migration checks behind `GET /healthz`
├── metrics_exposition.rs  `metrics_scrape`: the Prometheus 0.0.4 text served at `GET /metrics`
├── metrics_registry.rs    `Registry`: the metric families, their labels and the series cap
├── mod.rs                 the module tree
├── request_observer.rs    `observe`: the middleware that counts every request into the registry
└── tests/                 unit tests for the registry's cap and guard, and the exposition escaping
```

## How it works

`crate::core::http_router` creates one `Registry` per router, so the process holds no global
metrics state, and shares it with `observe`, `/metrics` and `/healthz`. `observe` sits inside the
access log but outside panic recovery and the rate limiter, so a panic's `500` and a throttle's
`429` are both counted. Its `route` label is the matched route template (`/api/v1/missions/{id}`,
never one series per id), `<unmatched>` for a request no route matched, and each family holds at
most `Registry::MAX_SERIES` (1024) series; a sample beyond that is dropped and counted in
`tbd_metrics_series_dropped_total`.

`GET /metrics` needs the `X-Service-Token` that `ServiceAuth` checks and answers 401 without it,
or while `SERVICE_TOKEN` is unset. It renders the request counters, the latency histogram, the
in-flight gauge, `tbd_http_rate_limited_total`, the build and the uptime, plus what the metrics
registry cannot accumulate and the scrape samples itself: a bounded database ping and the pool's
connections.

`GET /healthz` runs two checks, either of which turns it red: `database`, a `SELECT 1` bounded at
2 s, and `migrations`, which reads `_sqlx_migrations` and fails when the table is unreadable or
records a failed migration. The answer is 200 with `{"status": "ok"}` or 503 with
`{"status": "unavailable"}`. The route needs no credentials, because probes call it bare; only a
caller presenting a matching `X-Service-Token` gets the full report: the version, the uptime,
each check's status, latency and error, the migration counts and the pool gauges.

## Boundaries

- Depends on: `crate::core::configuration` and `crate::core::authentication_primitives` for the
  service-token check; the `axum` matched-path extension; the Postgres pool and its
  `_sqlx_migrations` table.
- Used by:
  - `crate::core::http_router`, which mounts `observe` and serves `/metrics` and `/healthz`;
  - the integration suite `apps/website/api_v2/tests/observability.rs`;
  - over HTTP: the Caddy site in `tools_v2/xtask/deploy/Caddyfile.website` publishes
    `/healthz`, and `cargo xtask platform preflight`, the `editor-api-boot` task of
    `cargo xtask ci` and the [Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator)
    smoke gates in `tools_v2/developer-tools/src/browser_testing/editor_smoke_tests/` probe it
    without credentials.
- Rules: `observe` stays outside panic recovery and the rate limiter, which the router's own tests
  check (`throttled_requests_are_counted` in
  `apps/website/api_v2/src/core/tests/http_router.rs`); `/healthz` discloses nothing beyond
  `status` without the token (`healthz_discloses_nothing_to_an_unauthenticated_caller`); the
  `route` label is always a template, never a raw path.
