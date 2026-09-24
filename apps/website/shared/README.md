# Shared URL-guard test table

The one table of inputs and expected verdicts for `is_http_url`, the scheme guard that the
[API](/documentation_v2/glossary.md#api) and the single-page app each implement. Both crates'
tests include this file, so the two implementations cannot drift apart without a test failing.

## Contents

```text
apps/website/shared/
└── is_http_url_cases.rs  `IS_HTTP_URL_CASES`: 86 URL inputs and the verdict both guards give
```

## How it works

`is_http_url` exists twice: `apps/website/api_v2/src/core/text/http_url_guard.rs` refuses a
non-`http(s)` link when it is written, and `apps/website/frontend/src/v2/core/auth/url_guard.rs`
refuses to render one as an `href`. The folder belongs to no crate. Each test that needs the table
pulls it in with `include!`, so `rustc` compiles the same string literals, control characters and
all, into both test suites, and a change to either implementation that disagrees with the table
fails that crate's tests.

The table holds 59 inputs that must be refused and 27 that must be accepted. The refused groups
are executing schemes in any case, hostile schemes that carry a real host, strings whose stored
and parsed forms differ (leading or trailing whitespace, tab, newline, carriage return and NUL
inside the URL), invisible characters, encoded schemes, scheme-relative and relative paths, and
URLs with an empty host. The accepted groups are ordinary `http` and `https` links, and hostile
URLs the guard accepts because it checks the scheme only: loopback and metadata addresses,
userinfo, lookalike hosts and backslash or slash variants. The comment above each group gives
its reason.

## Format

- Encoding: one Rust source fragment, UTF-8, with no module or item wrapper: a comment header and
  a single `const IS_HTTP_URL_CASES: &[(&str, bool)]`, where `true` means a browser follows the
  link and `false` means the guard refuses it.
- Schema: each entry is `(input, verdict)`, a Rust string literal with Rust escapes (`\t`,
  `\u{200b}`) and a `bool`. It is Rust rather than a data file so no hand-written unescaper sits
  between the table and the tests.
- Adding a case: put the entry in the group it belongs to with its verdict and a comment when the
  reason is not obvious, then run both crates' tests, for example
  `cargo test -p website-api --lib http_url_guard` and
  `cargo test -p website-frontend url_guard`. A `false` entry is a security assertion: when
  an implementation starts returning `true` for it, the implementation is wrong, not the table.

## Producers and consumers

- Producers: people; no tool writes the file.
- Consumers: tests that `include!` the file.
  - API crate: `apps/website/api_v2/src/core/text/tests/http_url_guard.rs` checks `is_http_url`
    against every case, and `apps/website/api_v2/tests/aar_replay_url_backfill.rs` runs every input
    through the Rust guard and the database's `looks_like_http_url` to pin where the two differ.
  - Frontend crate: `apps/website/frontend/src/v2/core/auth/tests/url_guard.rs` checks the guard
    itself, and the page tests that render a stored link or image run every input through their
    render path:
    - `apps/website/frontend/src/v2/pages/account/settings/tests/settings.rs`;
    - `apps/website/frontend/src/v2/pages/command_center/announcements/tests/announcements.rs`;
    - `apps/website/frontend/src/v2/pages/mission_hub/library/tests/mission_library.rs`;
    - `apps/website/frontend/src/v2/pages/navigation/tests/layout.rs`;
    - `apps/website/frontend/src/v2/pages/operations/deployments/tests/deployments.rs`;
    - `apps/website/frontend/src/v2/pages/operations/event_detail/tests/event_hub.rs`;
    - `apps/website/frontend/src/v2/pages/operations/leaderboards/tests/leaderboards.rs`.

## Boundaries

- Depends on: nothing; the file is plain literals.
- Used by: the API and frontend tests listed above; the wave tooling in
  `tools_v2/xtask/src/commands/platform/wave_execution/`, which resolves a change to a file here to
  the crates that include it, since the folder has no `Cargo.toml` of its own.
- Rules: the constant keeps its name and type, `IS_HTTP_URL_CASES: &[(&str, bool)]`, because every
  consumer names it; the include paths are relative to each test file or to the crate's
  `CARGO_MANIFEST_DIR`, so moving the file updates every `git grep is_http_url_cases.rs` hit in the
  same change; the API test `matches_the_frontend_guard_on_every_shared_case` and the frontend
  tests in `apps/website/frontend/src/v2/core/auth/tests/url_guard.rs` fail when either guard
  disagrees with a case.
