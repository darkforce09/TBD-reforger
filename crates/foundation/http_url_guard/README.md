# HTTP URL guard

The `http_url_guard` crate: the one predicate that decides whether a string is an absolute `http`
or `https` URL a browser follows rather than executes, and the table of inputs and verdicts that
pins it. The [API](/documentation/glossary/a_to_f.md#api) calls it before it stores a link; the
single-page app calls it before it renders one.

## Contents

```text
crates/foundation/http_url_guard/
├── Cargo.toml  the package: `url`, the dev-only `test_fixtures` feature, layout tier 0
└── src/        the predicate, the case table and the prelude
```

## How it works

`is_http_url` accepts an absolute URL whose scheme is `http` or `https` and whose host is not
empty, and refuses anything holding an ASCII control character or surrounding whitespace, so the
bytes checked are the bytes a browser follows. It is a scheme allowlist and nothing more: no
server-side request forgery check, no domain allowlist, no opinion about path, query or fragment.

Both boundaries link this crate, so the value the API stores and the value the page renders pass
the same compiled check. The case table `cases::IS_HTTP_URL_CASES` holds 59 inputs that must be
refused and 27 that must be accepted. The refused groups are executing schemes in any case,
hostile schemes that carry a real host, strings whose stored and parsed forms differ (leading or
trailing whitespace, tab, newline, carriage return and NUL inside the URL), invisible characters,
encoded schemes, scheme-relative and relative paths, and URLs with an empty host. The accepted
groups are ordinary `http` and `https` links, and hostile URLs the guard accepts because it checks
the scheme only: loopback and metadata addresses, userinfo, lookalike hosts and backslash or slash
variants. The crate's own tests run the predicate over every entry; the tests of each caller run
its storing or rendering path over the same entries.

## Getting started

Run from the repository root:

```bash
cargo test -p http_url_guard   # the predicate's unit tests and the whole case table
```

To add a case, put the entry in its group of `src/cases.rs` with its verdict, then run the
command above and the callers' tests, for example `cargo test -p frontend url_guard`.

## Configuration

One feature, `test_fixtures`, off by default: it compiles the `cases` module for the tests of
other crates and is enabled only from their `[dev-dependencies]`. The crate reads no environment
variable.

## Public surface

- `is_http_url(candidate: &str) -> bool`, also in `prelude`.
- `cases::IS_HTTP_URL_CASES: &[(&str, bool)]`, under `cfg(test)` or the `test_fixtures` feature:
  `true` means a browser follows the input, `false` means the guard refuses it.

## Boundaries

- Depends on: `url` (the WHATWG parser).
- Used by: the API (`apps/api`), whose writers of every URL column call the guard (listed in
  `crates/api/api_foundation/src/text/README.md`); the single-page app (`apps/frontend`), whose avatar
  sanitiser, Mission Creator settings, announcement, mission library, service record and
  leaderboard links call it. With `test_fixtures`: `apps/api/tests/aar_replay_url_backfill.rs`
  and the frontend page tests that render a stored link or image.
- Rules: a `false` entry of the table is a security assertion — when the guard starts answering
  `true` to it, the guard is wrong, never the table
  (`refuses_every_input_the_shared_table_marks_refused`); the table keeps its adversarial inputs
  (`shared_table_still_carries_the_adversarial_inputs`); foundation tier, so the crate depends on
  no workspace crate (`cargo xtask verify crate-tiers`).

## Related documentation

- [Engine boundary rules](/documentation/standards/engine_boundary_rules.md) — the dependency
  directions between the workspace crates.
