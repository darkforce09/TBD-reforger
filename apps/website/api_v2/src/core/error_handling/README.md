# Core Error Handling (`core/error_handling/`)

Centralized domain error enum, HTTP response serialization, and sanitized client diagnostics.

---

## 1. Modules

### `app_error.rs` (<350 LOC)
- **Purpose**: Defines `AppError` enum implementing Axum's `IntoResponse` trait.
- **Error Variants**:
  - `NotFound(String)`: Returns HTTP 404.
  - `Unauthorized(String)`: Returns HTTP 401.
  - `Forbidden(String)`: Returns HTTP 403.
  - `BadRequest(String)`: Returns HTTP 400.
  - `Conflict(String)`: Returns HTTP 409 (e.g., duplicate slug or reservation collision).
  - `TooManyRequests(u64)`: Returns HTTP 429 with `Retry-After` header.
  - `InternalServerError`: Returns HTTP 500 without leaking internal database or system error traces.

### `error_response.rs` (<200 LOC)
- **JSON Wire Contract**:
  ```json
  {
    "error": "Resource not found",
    "code": "RESOURCE_NOT_FOUND",
    "details": null,
    "request_id": "c6a1b2c3-d4e5-4f6a-8b9c-0d1e2f3a4b5c"
  }
  ```
- **Invariants**:
  - Automatically captures the active request ID from context to assist user debugging.
