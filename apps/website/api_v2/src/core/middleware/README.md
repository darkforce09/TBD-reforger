# Core Middleware (`core/middleware/`)

Axum middleware components enforcing security barriers, telemetry tracking, rate limits, and cross-origin resource access.

---

## 1. Middleware Stack Order

```text
Request Entry
  │
  ▼
1. Trace & Request ID (`request_context.rs`)
  │
  ▼
2. Observability Metrics (`observability/metrics.rs`)
  │
  ▼
3. CORS Policy (`cors.rs`)
  │
  ▼
4. Rate Limiter (`rate_limiter.rs`) <--- NOTE: /map-assets mounts OUTSIDE/BELOW this layer!
  │
  ▼
5. Authentication & Authorization Guards (`auth_guard.rs`)
  │
  ▼
Domain Handler Dispatch
```

---

## 2. Modules

### `rate_limiter.rs` (<420 LOC)
- **Purpose**: Sliding-window rate limiter keyed by client IP or authenticated user ID.
- **Invariants**:
  - Critical Seam: `/map-assets` static binary terrain tiles MUST be mounted below or outside this layer. Terrain tile bursts during Scenario Creator boots (up to 951 tiles) must never trigger 429 cascades.

### `auth_guard.rs` (<350 LOC)
- **Purpose**: Extracts and verifies JWT credentials from `Authorization: Bearer <token>` or HTTP-only cookies.
- **Invariants**:
  - Injects `CurrentUser` into Axum request extensions for zero-database role verification downstream.

### `request_context.rs` (<220 LOC)
- **Purpose**: Injects unique UUID `x-request-id` into request and response headers.
- **Invariants**:
  - Propagates request ID through all tracing spans and database query tags.

### `cors.rs` (<180 LOC)
- **Purpose**: Configures permissive headers for local development (:3000 -> :8080) and strict domain whitelists for staging/production.
