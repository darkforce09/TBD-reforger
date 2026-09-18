# Core Observability (`core/observability/`)

Prometheus metric collectors, structured log tracing initialization, and system health instrumentation.

---

## 1. Modules

### `metrics.rs` (<350 LOC)
- **Purpose**: Exposes Prometheus metrics on `/metrics` endpoint.
- **Metrics Collected**:
  - `http_requests_total`: Counter partitioned by method, route pattern, and status code.
  - `http_request_duration_seconds`: Histogram measuring end-to-end request latencies.
  - `db_pool_connections`: Gauge tracking active, idle, and max database connections.
  - `active_sse_subscribers`: Gauge tracking connected SSE streams.
- **Invariants**:
  - Prometheus observation middleware sits strictly outside rate-limiters and panic catchers to accurately record 429 and 500 status codes.

### `tracing_setup.rs` (<250 LOC)
- **Purpose**: Configures `tracing_subscriber` based on active environment.
- **Invariants**:
  - Development: Human-readable ANSI colored log lines.
  - Production: Compact, structured NDJSON log entries including trace and span IDs.
