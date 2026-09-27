# Handler errors

`ApiError`, the failure type the [API](/documentation_v2/glossary/a_to_f.md#api)'s handlers and services
return, and the JSON envelope it renders into: `{"error": "<message>"}`, with a `details` value when the failure carries one.

## Contents

```text
apps/website/api_v2/src/core/error_handling/
├── api_error.rs  `ApiError`: a status, a client message, optional details, and its JSON rendering
├── mod.rs        the module tree
└── tests/        unit tests for the JSON body and query string rejection mappings and the envelope
```

## How it works

`ApiError` carries the status the client sees. `ApiError::new` and `ApiError::with_details` take
any status; `bad_request`, `unauthorized`, `forbidden`, `not_found`, `conflict` and `internal`
name the common ones. A `sqlx::Error` converts into a logged `500` with the message
`internal error`, so a handler that owes the client a more specific answer (a `409` for a unique
violation, say) maps that case itself with `crate::core::database::postgres_errors`. The
extractors and the rate limiter in `crate::core::middleware` answer with the same envelope through
`json_error`.

`ApiError::from_json_rejection` turns the rejection of axum's `Json` extractor into the answer a
handler owes: a body over the request body limit (`core::middleware::MAX_JSON_BODY`) answers `413`
with `details.code = "request_too_large"`, a missing or non-JSON `Content-Type` answers `415`, and
every other unreadable body (malformed JSON, a value that does not decode into the target type)
answers `400` with the rejection's reason as the message. A handler opts in by taking
`Result<Json<T>, JsonRejection>` and mapping the error through it.

`ApiError::from_query_rejection` does the same for the rejection of axum's `Query` extractor: a
query string that does not decode (a `limit` that is not an integer, say) answers `400` in the
envelope, never axum's plain-text body, with the message `invalid <query name>: <reason>`, whose
reason names the refused parameter. A handler opts in by taking
`Result<Query<T>, QueryRejection>` and mapping the error through it with the name of its query.

## Boundaries

- Depends on: `axum` (the response types and the `Json` and `Query` rejections), `serde_json` and
  `tracing`; the tests drive the rejections through `tower`'s `ServiceExt::oneshot`.
- Used by: the handlers and services of all eight domains, among them the vehicle database, wiki
  and announcement handlers of `community_content`, which map their JSON body rejections through
  `from_json_rejection`; the announcement list handlers of `community_content` and the personnel
  roster of `administration`, which map their query string rejections through
  `from_query_rejection`; the `event_reservation_reevaluator`
  and `runtime_session_expiry`
  [background workers](/documentation_v2/glossary/a_to_f.md#background-workers);
  `crate::core::authentication_primitives`, whose `SessionAuthority` refuses with it; two
  integration suites under `apps/website/api_v2/tests/`; and, over HTTP, the single-page app, which
  reads `error` and a string-array `details` in
  `apps/website/frontend/src/v2/core/api/client/errors.rs`.
- Rules: `error` stays a string and `details` stays optional, the shape the single-page app
  parses; a database error never reaches the client as text; the rejection mapping answers `413`,
  `415` and `400` exactly as `ContentRefusal` in
  `contracts_v2/definitions/content-upload.schema.json` describes, and an undecodable query string
  answers `400` in the envelope (`tests/api_error.rs`).
