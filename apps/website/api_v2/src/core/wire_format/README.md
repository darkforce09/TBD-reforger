# JSON wire formats

The serialization primitives the [API](/documentation/glossary/a_to_f.md#api)'s wire models share:
the spellings of instants and dates, the passthrough type for `jsonb` columns, and the canonical-JSON
content digest.

## Contents

```text
apps/website/api_v2/src/core/wire_format/
├── content_digest.rs      `canonical_json`, `sha256_hex` and `canonical_sha256`: key-sorted JSON and its SHA-256
├── mod.rs                 the module tree; re-exports `RawJson` and the three timestamp modules
├── raw_json.rs            `RawJson`: a `jsonb` column emitted verbatim, without re-serialization
├── rfc3339_timestamps.rs  the `#[serde(with = …)]` modules for instants and dates
└── tests/                 unit tests of `content_digest.rs`
```

## How it works

A model field applies one of three modules with `#[serde(with = …)]`. `rfc3339_utc` writes a
`timestamptz` as RFC 3339 in UTC with a `Z` suffix and fractional seconds only when they are not
zero, trailing zeros trimmed (`2026-07-06T12:00:00Z`, `2026-07-06T12:00:00.5Z`).
`rfc3339_utc_opt` does the same for an optional instant, which the field's `skip_serializing_if`
turns into an absent key. `rfc3339_utc_date` writes a Postgres `date` as midnight UTC
(`2026-07-06T00:00:00Z`) and reads that spelling or a bare `2026-07-06`. `RawJson` decodes a
`jsonb` column into a raw JSON value that serde writes out byte for byte.

`canonical_json` sorts object keys at every depth before serde writes compact JSON, because
serde_json's `preserve_order` feature is unified into the API build and a `Map` would otherwise keep
insertion order; `canonical_sha256` digests that text. Mission artifacts digest their inputs with it,
and match telemetry digests registrations, results revisions and events with it, so two senders of
the same content produce the same digest whatever their key order.

## Boundaries

- Depends on: `chrono`, `hex`, `serde`, `serde_json`, `sha2` and `sqlx`.
- Used by: the models of all eight domains, and integration suites under
  `apps/website/api_v2/tests/`; over HTTP, the single-page app's DTOs and the game server's mod
  parse the spellings.
- Rules: the spellings above are the wire contract, pinned by
  `apps/website/api_v2/tests/models_serde.rs`; a change to one changes what the single-page app's
  DTO golden tests and the mod parse.
