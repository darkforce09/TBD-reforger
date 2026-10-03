# API foundation source

The source of `api_foundation`: four modules, each a folder of its own, under the crate root.

## Contents

```text
crates/api/api_foundation/src/
├── error.rs         `Error`: a wire date that does not parse, and `Result`
├── error_handling/  `ApiError`, its `{error, details?}` envelope and rejection mapping; `message_with_causes`
├── http/            `PageParams` and `PathParams`, the query and path parameters every route reads
├── lib.rs           the crate root: module header, `mod` lines and the re-export of `Error`
├── prelude.rs       `ApiError`, `PageParams`, `PathParams` and `RawJson` for glob import
├── text/            the HTML sanitiser, its preview helpers and the content URL policy
└── wire_format/     the date spelling, the `jsonb` passthrough and the canonical-JSON digest
```

## How it works

Each folder is a public module with its own README. `http::path_parameters` answers a refused
path segment through `error_handling::api_error::ApiError`; the other modules stand alone.

## Boundaries

- Depends on: `ammonia`, `axum`, `chrono`, `content_digest`, `serde`, `serde_json`, `sqlx`,
  `thiserror` and `tracing`.
- Used by: the crate root and, through it, every API crate and the API application.
- Rules: no module names a domain or another API crate.
