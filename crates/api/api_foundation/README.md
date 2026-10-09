# API foundation

The `api_foundation` crate: the primitives every crate of the
[API](/documentation/glossary/a_to_f.md#api) builds on. It holds the one handler failure and the
`{error, details?}` JSON envelope it renders into, the JSON wire formats the models share, the
HTML sanitiser and the content URL policy of authored text, and the query and path parameters
every route reads.

## Contents

```text
crates/api/api_foundation/
├── Cargo.toml  the package: `axum`, `sqlx` (JSON), `ammonia`, `content_digest`, layout tier 1
└── src/        the error handling, wire format, text and request parameter modules
```

## How it works

A handler or service returns `ApiError`, which carries the status, the client message and
optional details, and which axum renders as `{"error": message}` plus `"details"` when set. A
`sqlx::Error` converts into a logged `500 internal error`, so a database failure never reaches
the client as text; axum's JSON, query and path rejections map into the same envelope, and every
route reads its path through `PathParams`, which applies that mapping. List routes clamp their
paging through `PageParams`.

The models write dates through `wire_format::rfc3339_utc_date`, pass `jsonb` columns through
`RawJson` byte for byte, and digest structured inputs as canonical JSON through
`wire_format::content_digest`. The text module sanitises the few fields that render as HTML and
decides which link targets and image sources authored content may carry, agreeing with the
contract schemas' URL patterns.

## Getting started

Run from the repository root:

```bash
cargo test -p api_foundation   # the envelope, rejection mapping, digests, text and parameter tests
```

## Configuration

No feature and no environment variable.

## Public surface

- `error_handling::api_error::ApiError` and `error_handling::error_causes::message_with_causes`.
- `wire_format`: `RawJson`, `rfc3339_utc_date`, `content_digest::{canonical_json, canonical_sha256}`.
- `text`: `html_sanitizer::{sanitize_html, snippet, truncate, cap_runes}` and
  `content_url_policy::{is_safe_image_url, is_safe_link_url, is_external_link}`.
- `http`: `pagination::PageParams`, `path_parameters::PathParams` and
  `required_text_field::required_trimmed_text`.
- `Error` and `Result`.
- `prelude`: `ApiError`, `PageParams`, `PathParams` and `RawJson`.

## Boundaries

- Depends on: `content_digest` and external crates (`ammonia`, `axum`, `chrono`, `serde`,
  `serde_json`, `sqlx`, `thiserror`, `tracing`).
- Used by: the API application
  (`crates/api/api_server`): every domain's handlers, services and models, its middleware and its integration
  suites.
- Rules: nothing here names a domain or an API crate above it; the wire spellings are contract,
  pinned by `crates/api/api_server/tests/models_serde.rs` and the golden tests.

## Related documentation

- [API crates](/crates/api/README.md) — the category this crate belongs to and its rules.
- [API errors and logging](/documentation/standards/coding_standards/api_errors_and_logging.md) —
  how handlers report failures through `ApiError`.
