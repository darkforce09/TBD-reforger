**Status:** live

# API errors and request logging

Rules ERR-1, ERR-2, ERR-4, ERR-5 and LOG-3: the one error shape the
[API](/documentation_v2/glossary/a_to_f.md#api) speaks, the status codes it answers with, and what it
logs about a failed request. The code number ERR-3 is not used; its logging requirement is part of
LOG-3. The error type itself is described in the
[handler errors README](/apps/website/api_v2/src/core/error_handling/README.md).

## Error envelope

- **ERR-1 (Usability) — An error body is `{"error": "<message>"}`, with an optional `details`.**
  `ApiError` in [api_error.rs](/apps/website/api_v2/src/core/error_handling/api_error.rs) renders
  it, and `json_error` in `apps/website/api_v2/src/core/middleware/mod.rs` renders the same shape
  for the extractors and the rate limiter. `details` is any JSON value: an array of strings for
  payload validation (`"invalid mission payload"` with one message per problem, in
  `apps/website/api_v2/src/missions/handlers/mission_versions.rs`), or an object carrying a `code`
  or a `reason` elsewhere (the [mission deployment](/documentation_v2/glossary/g_to_m.md#mission-deployment) refusals, the modpack body error). The
  single-page app folds `details` into the message it shows only when it is an array of strings
  (`apps/website/frontend/src/v2/core/api/client/errors.rs`). A `sqlx::Error` never reaches the
  client as text: it becomes a logged `500` with the message `internal error`. Status: live, held
  by construction (every handler returns `ApiError`); no gate asserts the shape.
- **ERR-4 (Usability) — No error body carries a top-level key other than `error` and `details`.**
  `ApiError` cannot render another key. Status: live, unenforced: no gate finds a handler that
  builds its own error JSON instead of returning `ApiError`.

A success list answers `{"data": [...], "total": n, "limit": l, "offset": o}`, with `limit`
defaulting to 20 and capped at 100 (`PageParams` in
`apps/website/api_v2/src/core/http/pagination.rs`); the audit log pages by cursor instead, with
`{"data": [...], "next_cursor": id}`.

## Status codes

- **ERR-2 (Usability) — Status codes follow this table.**

  | Status | Meaning | Used when |
  |---|---|---|
  | `200 OK` | success | a read or an update |
  | `201 Created` | a resource was created | a POST that persists, such as a mission or a mission version |
  | `400 Bad Request` | malformed or invalid input | an extractor rejection or a payload that fails validation (`details`) |
  | `401 Unauthorized` | no valid session | a missing, expired or invalid token |
  | `403 Forbidden` | signed in, not allowed | the wrong role, or "not your mission" |
  | `404 Not Found` | no such resource | an unknown id, or a draft hidden from someone other than its author |
  | `409 Conflict` | a state or uniqueness clash | a duplicate version, a unique violation (SQLSTATE `23505`, GO-5), a refused deployment |
  | `413 Payload Too Large` | the body passes the route's limit | a mission version over `MISSION_VERSION_MAX_BODY_BYTES` (256 MiB by default); an upload over its cap |
  | `422 Unprocessable Entity` | well formed, but the request cannot be carried out | a mission artifact or deployment selection that does not hold, a fire mission with no solution, a runtime session request that does not apply |
  | `429 Too Many Requests` | a rate limit tripped | with `Retry-After` and `{"error": "rate limit exceeded"}` |
  | `500 Internal Server Error` | an unexpected fault | an unhandled database or internal error |
  | `502 Bad Gateway` | an upstream service failed | the Discord push of an announcement |
  | `503 Service Unavailable` | a dependency is down | the durable rate limiter cannot reach its store; `GET /healthz` with a failing check |

  Status: live, unenforced: no gate compares the answers with the table; the integration tests
  under `apps/website/api_v2/tests/` pin the statuses of the routes they cover.
- **ERR-5 (Usability) — Each status class a resource answers with has a named integration test.**
  Status: live, unenforced: the integration suites cover many routes, and no gate checks that
  every class of every resource has its test.

## Request logging

- **LOG-3 (Debuggability) — A request that fails is logged with its path, status and duration.**
  The `logging` middleware in
  [tracing_correlation.rs](/apps/website/api_v2/src/core/middleware/tracing_correlation.rs), which
  `apps/website/api_v2/src/core/http_router.rs` mounts on every route, writes one structured
  `access` line per request with the request id, method, path, status and elapsed milliseconds,
  so every `4xx` and `5xx` is logged without handler code. A database error is logged a second
  time, at error level, where it converts into `ApiError`. An operation that fails on the side of
  a request that still answers `200` logs that failure itself. The request metrics use the
  matched route template (`/missions/{id}`), never the raw path. Status: live, held by the
  middleware; no gate checks it.

## Related

- [API code structure and route tags](/documentation_v2/standards/coding_standards/api_code_structure.md)
  — GO-5, the `409` for a unique violation.
- [Testing bar](/documentation_v2/standards/coding_standards/testing_bar.md) — TEST-1, the
  integration tests that pin statuses.
