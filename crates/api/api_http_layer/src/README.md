# API HTTP layer source

The source of `api_http_layer`: five modules, each a folder of its own, under the crate root.

## Contents

```text
crates/api/api_http_layer/src/
├── authentication_primitives/  access tokens, opaque-token hashing and the session-authority trait
├── error.rs                    `Error`: a refused access token or an unreachable rate-limit store, and `Result`
├── http_client/                the bounded retry for outbound calls answered with 429
├── lib.rs                      the crate root: module header, `mod` lines and the re-export of `Error`
├── middleware/                 the request middleware chain and the authentication extractors
├── observability/              Prometheus metrics, `GET /metrics` and `GET /healthz`
├── prelude.rs                  `Claims`, `Manager`, the extractors, `json_error` and `Hub` for glob import
└── realtime_hub/               the in-process publish-subscribe hub behind the SSE streams
```

## How it works

Each folder is a public module with its own README. `middleware` verifies bearer tokens with
`authentication_primitives`; `observability` checks its bearer with
`authentication_primitives::constant_time_equal` and answers refusals through
`middleware::json_error`; `authentication_primitives::session_authority` names
`middleware::AuthUser`, the identity it answers with. `realtime_hub` and `http_client` stand
alone.

## Boundaries

- Depends on: `api_foundation`, `api_configuration`, `api_identifiers`, `content_digest` and the
  external crates of `Cargo.toml`.
- Used by: the crate root and, through it, the API application.
- Rules: no module names the application state, another kernel crate above it or a domain.
