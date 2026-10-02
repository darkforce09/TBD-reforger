# Foundation crates

The lowest tier of the library crates: small building blocks that depend on no workspace crate,
so every other crate may link them.

## Contents

```text
crates/foundation/
├── browser_platform/  `browser_platform`: browser console macros and same-origin GETs for wasm32 code
├── content_digest/  `content_digest`: lowercase hex SHA-256 and SHA-384, an incremental framed SHA-256
├── deterministic_random/  `deterministic_random`: SplitMix64, the seeded generator of every reproducible draw
├── http_url_guard/  `http_url_guard`: whether a string is an `http` or `https` URL a browser follows
├── newtype_ids/  `newtype_ids`: the `string_id!`, `integer_id!` and `uuid_id!` identifier macros
└── time_source/  `time_source`: the wall-clock trait and clocks, monotonic time, RFC 3339 UTC stamps
```

## Boundaries

- Depends on: external crates only.
- Used by: the API and the single-page app; `content_digest` by `xtask`; `browser_platform` by
  the map engine's `render` tier.
  - `deterministic_random`, `newtype_ids` and `time_source` have no caller yet; they replace the
    generators, clocks and timestamp helpers of the map engine and the repository tools.
- Rules: a foundation crate declares `category = "crates/foundation"` and depends on no
  workspace crate (`cargo xtask verify crate-tiers`).
