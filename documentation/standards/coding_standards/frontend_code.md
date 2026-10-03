**Status:** live

# Frontend code

Rules TS-1 to TS-7 and LOG-2: the rules for the single-page app, `apps/frontend/`
(package `frontend`, Leptos compiled to WebAssembly). The codes keep their TS prefix
because the rules were first written for a TypeScript app; no TypeScript remains. Each rule below
states its Rust form and whether anything checks it. Where the app's files go is in
[Where does X go?](/documentation/standards/where_does_x_go.md); its gates are in
[Testing and CI](/documentation/runbooks/testing_and_ci.md).

## Types and contracts

- **TS-1 (Debuggability) — The compiler runs in its strictest mode.** Rust form: the Rust
  compiler, with `cargo clippy -p frontend` for `wasm32-unknown-unknown` and natively, both with
  `-D warnings`, in `cargo xtask mk ci-local-leptos`, so a warning fails the job. Status: retired as
  a separate rule; the Rust compiler carries it.
- **TS-3 (Debuggability) — Contract data is fully typed.** Rust form: every
  [API](/documentation/glossary/a_to_f.md#api) answer deserialises into a `serde` DTO under
  `apps/frontend/src/foundation/transport/dto/`. Status: retired as a separate rule; the Rust type
  system carries it.
- **TS-6 (Readability) — A cross-boundary type mirrors its API model exactly.** Rust form: each
  DTO mirrors the snake_case model in `apps/api/src/<domain>/models/`, and the API wins
  a disagreement (CLAUDE.md law 9). The R-api golden tests in
  `apps/frontend/src/foundation/transport/dto/tests/` hold each DTO to an answer captured from the
  API: re-serialising reproduces the capture byte for byte, and the keys no field reads are
  exactly the ones the test lists. Gate: CI-BLOCK, `cargo test -p frontend` in the
  `frontend` job. The `@contract` citation gate (`cargo xtask ci verify-citations`) prints
  that its TS-6 slot is retired, because the app has no separate export-tag surface; the
  [DTO README](/apps/frontend/src/foundation/transport/dto/README.md) describes the goldens.
- **TS-5 (Readability) — Every exported contract item carries a doc comment.** Rust form: the
  `///` rules of the
  [documentation standards](/documentation/standards/documentation_standards.md). Status:
  retired as a code rule; the documentation standards own it.

## Layers

- **TS-2 (Scalability) — Layer boundaries hold.** Rust form: `src/foundation/` holds what every
  layer shares (the transport and DTOs, the route table, auth, the design-system primitives,
  utilities); `src/features/` holds capabilities several pages and workspaces show;
  `src/pages/` holds the platform pages, one folder per area and page; `src/workspaces/` holds
  the standalone workspaces, such as the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator);
  `src/shell/` holds the frame around every route, and the routes live in
  `apps/frontend/src/app_routes.rs`. A layer imports only the layers below it (foundation <
  features < pages, workspaces < shell); inside the foundation a sub-area imports only the
  sub-areas before it (ui < utils < transport < route_table < auth < {offline, map_view}, the
  last two peers), and only test files import `foundation/test_support`. The app never names
  `wgpu`: it reaches the GPU only through `map_renderer`, `paper_doll_renderer` and `gpu_frame`'s
  frame pump. Gate: CI-SCRIPT, `cargo xtask verify frontend-layering` for the layer and sub-area
  order (any edge fails) and the wgpu firewall of `cargo xtask verify crate-tiers` for the crate
  wall ([Crate boundary rules](/documentation/standards/crate_boundary_rules.md)).

## Errors and logging

- **TS-4 (Usability) — A failed request shows the user an error.** Rust form: the client turns an
  error body into a message with `error_body_message` in
  `apps/frontend/src/foundation/transport/client/errors.rs`, which appends up to six `details`
  lines when `details` is an array of strings; each page renders that message in its error
  state. Status: live, unenforced.
- **TS-7 (Usability) — No failure is swallowed.** Rust form: a `Result` is surfaced, retried or
  handled; discarding one is a compile warning (`unused_must_use`), and an explicit discard needs
  a reason beside it. Status: live; gate: CI-SCRIPT, the app's clippy runs deny warnings
  (`cargo xtask mk ci-local-leptos`).
- **LOG-2 (Debuggability) — No debug console logging is committed.** Rust form:
  `leptos::logging::error!` and `warn!` report real failures; a development counter or overlay
  sits behind a development guard. Status: live, unenforced: no gate scans the app for console
  logging.

## Related

- [API errors and request logging](/documentation/standards/coding_standards/api_errors_and_logging.md)
  — the error envelope the app reads.
- [Testing bar](/documentation/standards/coding_standards/testing_bar.md) — TEST-2, the app's
  unit tests.
