# Core Tests (`core/tests/`)

Sibling unit and infrastructure test specifications for configuration validation, database connection pooling, error responses, and middleware pipelines.

---

## 1. Test Modules

Declared via Monorepo Law #7 (`#[cfg(test)] #[path = "tests/<file>.rs"] mod tests;`).

### `configuration.rs`
- **Coverage**: Missing environment variable panics, malformed port strings, production credential constraints, and development flag defaults.

### `database.rs`
- **Coverage**: Pool connection timeout enforcement, migration idempotency, and transactional rollback on error.

### `error_handling.rs`
- **Coverage**: Correct HTTP status code mappings for each `AppError` variant, JSON wire format adherence, and internal error stack trace masking.

### `rate_limiter.rs`
- **Coverage**: Sliding window accuracy, IP extraction behind proxies, rate-limit header emission (`X-RateLimit-Remaining`), and 429 response formatting.

### `auth_guard.rs`
- **Coverage**: Bearer token extraction, invalid token rejection (401), expired token rejection, and request extension injection.

### `realtime_hub.rs`
- **Coverage**: Multi-subscriber fanout, slow subscriber channel lag handling, and heartbeat packet intervals.
