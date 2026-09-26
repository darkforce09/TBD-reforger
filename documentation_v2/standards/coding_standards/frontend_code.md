**Status:** live

# Frontend code

Rules TS-1 to TS-7 and LOG-2: the rules for the single-page app, `apps/website/frontend/`
(package `website-frontend`, Leptos compiled to WebAssembly). The codes keep their TS prefix
because the rules were first written for a TypeScript app; no TypeScript remains. Each rule below
states its Rust form and whether anything checks it. Where the app's files go is in
[Where does X go?](/documentation_v2/standards/where_does_x_go.md); its gates are in
[Testing and CI](/documentation_v2/runbooks/testing_and_ci.md).

## Types and contracts

- **TS-1 (Debuggability) — The compiler runs in its strictest mode.** Rust form: the Rust
  compiler, with `cargo clippy -p website-frontend --target wasm32-unknown-unknown` in
  `cargo xtask mk ci-local-leptos`. Clippy runs there without `-D warnings`, so a warning prints and
  does not fail the job. Status: retired as a separate rule; the Rust compiler carries it.
- **TS-3 (Debuggability) — Contract data is fully typed.** Rust form: every
  [API](/documentation_v2/glossary/a_to_f.md#api) answer deserialises into a `serde` DTO under
  `apps/website/frontend/src/v2/core/api/dto/`. Status: retired as a separate rule; the Rust type
  system carries it.
- **TS-6 (Readability) — A cross-boundary type mirrors its API model exactly.** Rust form: each
  DTO mirrors the snake_case model in `apps/website/api_v2/src/<domain>/models/`, and the API wins
  a disagreement (CLAUDE.md law 9). The R-api golden tests in
  `apps/website/frontend/src/v2/core/api/dto/tests/` hold each DTO to an answer captured from the
  API: re-serialising reproduces the capture byte for byte, and the keys no field reads are
  exactly the ones the test lists. Gate: CI-BLOCK, `cargo test -p website-frontend` in the
  `website-frontend` job. The `@contract` citation gate (`cargo xtask ci verify-citations`) prints
  that its TS-6 slot is retired, because the app has no separate export-tag surface; the
  [DTO README](/apps/website/frontend/src/v2/core/api/dto/README.md) describes the goldens.
- **TS-5 (Readability) — Every exported contract item carries a doc comment.** Rust form: the
  `///` rules of the
  [documentation standards](/documentation_v2/standards/documentation_standards.md). Status:
  retired as a code rule; the documentation standards own it.

## Layers

- **TS-2 (Scalability) — Layer boundaries hold.** Rust form: `src/v2/core/` holds what every
  page shares (the API client and DTOs, auth, the design-system primitives, utilities);
  `src/v2/pages/` holds the platform pages, one folder per area and page; `src/v2/apps/` holds
  the standalone workspaces, such as the [Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator);
  the routes live in `apps/website/frontend/src/app_routes.rs`. The app never names the graphics
  engine: it reaches the GPU only through the map engine. Gate: CI-SCRIPT for the engine wall
  only, rule 6 of `cargo xtask verify engine-layers`
  ([Engine boundary rules](/documentation_v2/standards/engine_boundary_rules.md)); the page, app
  and core layering inside the crate is unenforced.

## Errors and logging

- **TS-4 (Usability) — A failed request shows the user an error.** Rust form: the client turns an
  error body into a message with `error_body_message` in
  `apps/website/frontend/src/v2/core/api/client/errors.rs`, which appends up to six `details`
  lines when `details` is an array of strings; each page renders that message in its error
  state. Status: live, unenforced.
- **TS-7 (Usability) — No failure is swallowed.** Rust form: a `Result` is surfaced, retried or
  handled; discarding one is a compile warning (`unused_must_use`), and an explicit discard needs
  a reason beside it. Status: live, unenforced: the app's clippy run does not deny warnings.
- **LOG-2 (Debuggability) — No debug console logging is committed.** Rust form:
  `leptos::logging::error!` and `warn!` report real failures; a development counter or overlay
  sits behind a development guard. Status: live, unenforced: no gate scans the app for console
  logging.

## Related

- [API errors and request logging](/documentation_v2/standards/coding_standards/api_errors_and_logging.md)
  — the error envelope the app reads.
- [Testing bar](/documentation_v2/standards/coding_standards/testing_bar.md) — TEST-2, the app's
  unit tests.
