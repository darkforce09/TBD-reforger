# Handler errors

`ApiError`, the failure type the [API](/documentation_v2/glossary.md#api)'s handlers and services
return, and the JSON envelope it renders into: `{"error": "<message>"}`, with a `details` value when the failure carries one.

## Contents

```text
apps/website/api_v2/src/core/error_handling/
├── api_error.rs  `ApiError`: a status, a client message, optional details, and its JSON rendering
└── mod.rs        the module tree
```

## How it works

`ApiError` carries the status the client sees. `ApiError::new` and `ApiError::with_details` take
any status; `bad_request`, `unauthorized`, `forbidden`, `not_found`, `conflict` and `internal`
name the common ones. A `sqlx::Error` converts into a logged `500` with the message
`internal error`, so a handler that owes the client a more specific answer (a `409` for a unique
violation, say) maps that case itself with `crate::core::database::postgres_errors`. The
extractors and the rate limiter in `crate::core::middleware` answer with the same envelope through
`json_error`.

## Boundaries

- Depends on: `axum`, `serde_json` and `tracing`.
- Used by: the handlers and services of all eight domains; the `event_reservation_reevaluator`
  and `runtime_session_expiry` background workers; `crate::core::authentication_primitives`,
  whose `SessionAuthority` refuses with it; two integration suites under
  `apps/website/api_v2/tests/`; and, over HTTP, the single-page app, which reads `error` and a
  string-array `details` in `apps/website/frontend/src/v2/core/api/client/errors.rs`.
- Rules: `error` stays a string and `details` stays optional, the shape the single-page app
  parses; a database error never reaches the client as text.
