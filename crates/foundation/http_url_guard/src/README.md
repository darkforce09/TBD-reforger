# HTTP URL guard source

The source of `http_url_guard`: the scheme predicate, the case table that pins it, and the crate
root that exports them.

## Contents

```text
crates/foundation/http_url_guard/src/
├── cases.rs     `IS_HTTP_URL_CASES`: 86 URL inputs and the verdict the guard gives each
├── http_url.rs  `is_http_url`: an absolute `http` or `https` URL with a host, no control bytes
├── lib.rs       the crate root: module header, `mod` lines and the re-export
├── prelude.rs   `is_http_url` for glob import
└── tests/       unit tests of the predicate and the whole case table
```

## How it works

`lib.rs` re-exports `http_url::is_http_url` at the crate root and in `prelude`. `cases.rs`
compiles only for this crate's tests and for the `test_fixtures` feature; `tests/http_url.rs` runs
the predicate over every entry, refused and followed halves separately, and checks that the table
still carries the adversarial inputs.

## Boundaries

- Depends on: `url`.
- Used by: the API and the single-page app, through the crate root.
- Rules: `cases.rs` holds Rust string literals, not a data file, so `rustc` unescapes the control
  characters the cases depend on and no hand-written unescaper sits between table and tests.
