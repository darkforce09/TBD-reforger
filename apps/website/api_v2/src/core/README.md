# Core Subsystem (`core/`)

Foundational application runtime, declarative router, shared state container, database connection pooling, error handling, Prometheus metrics engine, and global middleware pipeline.

---

## 1. Subsystem Topology & Responsibilities

The `core/` domain eliminates legacy root clutter (`app.rs`, `config.rs`, `db.rs`, `state.rs`, `error.rs`, `realtime.rs`, `middleware/`) by structuring shared infrastructure into focused, cohesive modules:

```text
src/core/
├── README.md                           <-- Domain documentation (this document)
├── http_router.rs                      <-- Declarative top-level router (<150 LOC)
├── application_state.rs                <-- Shared AppState Arc container & extractors
│
├── error_handling/
│   ├── mod.rs
│   ├── api_error.rs                    <-- ApiError struct & HTTP status mappings (<80 LOC)
│   └── tests/api_error.rs              <-- External sibling unit tests
│
├── configuration/
│   ├── mod.rs                          <-- Config struct, loader, and accessors (<250 LOC)
│   ├── proxy_network.rs                <-- ProxyNet parser, CIDR & bitwise IP math (<120 LOC)
│   └── tests/config.rs                 <-- External sibling unit tests
│
├── database/
│   ├── mod.rs                          <-- Pool connection & retry logic (<180 LOC)
│   ├── connection_pool.rs              <-- PgPool tuning from environment (<120 LOC)
│   ├── migration_runner.rs             <-- Embedded schema migrations runner
│   └── tests/database.rs               <-- External sibling unit tests
│
├── realtime_hub/
│   ├── mod.rs                          <-- Hub engine & topic subscription (<90 LOC)
│   ├── sse_broadcaster.rs              <-- Multi-channel broadcast bus
│   └── tests/realtime_hub.rs           <-- External sibling unit tests
│
├── observability/
│   ├── mod.rs                          <-- Observability facade & scrape handler (<150 LOC)
│   ├── metrics.rs                      <-- Prometheus 0.0.4 text registry & histograms (<200 LOC)
│   ├── health_probe.rs                 <-- Bounded 200/503 health probe (<150 LOC)
│   └── tests/observability.rs          <-- External sibling unit tests
│
└── middleware/
    ├── mod.rs                          <-- Execution order & ranking constants (<50 LOC)
    ├── authentication.rs               <-- AuthUser, LeaderUser, AdminUser, ServiceAuth (<125 LOC)
    ├── cross_origin.rs                 <-- CORS allow-list & OPTIONS preflight (<60 LOC)
    ├── tracing_correlation.rs          <-- RequestId injection & structured access logging (<65 LOC)
    ├── rate_limiting.rs                <-- RateLimitState & rate_limit middleware (<220 LOC)
    ├── client_identity.rs              <-- X-Forwarded-For trusted proxy chain parser (<150 LOC)
    ├── durable_ratelimit.rs            <-- Postgres L2 token bucket engine (<145 LOC)
    └── tests/rate_limiting.rs          <-- External sibling unit tests (<450 LOC)
```

---

## 2. Invariants & Key Architectural Rules

### 2.1 The T-630 Rate-Limit Seam
`/map-assets` provides DEM terrain elevation, satellite orthophotos, and 3D world geometry to the Mission Editor and Planner. A single cold editor load requests up to 951 binary chunks. In `http_router.rs`, `/map-assets` is mounted strictly **outside and below** the rate-limiting layer, ensuring it is completely exempt from rate limiting while all other routes remain protected.

### 2.2 Prometheus Metrics (Zero Third-Party Lockfile Bloat)
Prometheus metrics (`observability/metrics.rs`) implement format version 0.0.4 with cumulative-bucket latency histograms (12 fixed buckets from 1ms to 10s), an in-flight request gauge with RAII decrement on drop, and a hard cardinality ceiling (`MAX_SERIES = 1024`). The `observe` middleware is mounted **outside** the panic catcher and rate limiter so that 500 panics and 429 throttles are accurately recorded.

### 2.3 Health Probe Boundary Disclosure
`GET /healthz` executes bounded database probes (`SELECT 1` with 2-second timeout) and migration state verification:
- **Public Callers** (no token): Receives strictly `{"status": "ok"}` or `{"status": "unavailable"}` with 200 or 503 status code. No build, uptime, or pool information is disclosed.
- **Internal Callers** (`X-Service-Token` matching): Receives full diagnostics payload (version, uptime, check latencies, applied migration count, pool connection gauges).
