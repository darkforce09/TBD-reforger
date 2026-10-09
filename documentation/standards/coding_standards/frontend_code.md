**Status:** live

# Frontend code

Rules TS-1 to TS-7 and LOG-2: the rules for the single-page app, `crates/frontend/shell/frontend_application/`
(package `frontend_application`, Leptos compiled to WebAssembly), and its library crates under
`crates/frontend/`. The codes keep their TS prefix
because the rules were first written for a TypeScript app; no TypeScript remains. Each rule below
states its Rust form and whether anything checks it. Where the app's files go is in
[Where does X go?](/documentation/standards/where_does_x_go.md); its gates are in
[Testing and CI](/documentation/runbooks/testing_and_ci.md).

## Types and contracts

- **TS-1 (Debuggability) — The compiler runs in its strictest mode.** Rust form: the Rust
  compiler, with `cargo clippy` for `wasm32-unknown-unknown` and natively, both with
  `-D warnings`, in `cargo xtask mk ci-local-leptos`, so a warning fails the job. The lane derives
  its package list from the workspace members: the app `frontend` first, then every member under
  `crates/frontend/`, each with its own `-p`, so a frontend crate is formatted, linted for both
  targets and tested from the moment the workspace names it. Status: retired as
  a separate rule; the Rust compiler carries it.
- **TS-3 (Debuggability) — Contract data is fully typed.** Rust form: every
  [API](/documentation/glossary/a_to_f.md#api) answer deserialises into a `serde` DTO under
  `crates/frontend/foundation/frontend_api_dtos/src/`. Status: retired as a separate rule; the Rust type
  system carries it.
- **TS-6 (Readability) — A cross-boundary type mirrors its API model exactly.** Rust form: each
  DTO mirrors the snake_case model in `crates/api/api_<domain>/src/models/`, and the API wins
  a disagreement (CLAUDE.md law 9). The R-api golden tests in
  `crates/frontend/foundation/frontend_api_dtos/src/tests/` hold each DTO to an answer captured from the
  API: re-serialising reproduces the capture byte for byte, and the keys no field reads are
  exactly the ones the test lists. Gate: CI-BLOCK, `cargo test -p frontend_api_dtos` in the
  `frontend` job. The
  [DTO README](/crates/frontend/foundation/frontend_api_dtos/src/README.md) describes the goldens.
- **TS-5 (Readability) — Every exported contract item carries a doc comment.** Rust form: the
  `///` rules of the
  [documentation standards](/documentation/standards/documentation_standards.md). Status:
  retired as a code rule; the documentation standards own it.

## Layers

- **TS-2 (Scalability) — Layer boundaries hold.** Rust form: the frontend's library crates sit
  in four layer folders under `crates/frontend/`: `foundation/` holds what every layer shares (the
  design-system primitives and utilities, the DTOs, the transport, the route table, the session,
  the map view, offline use, the test support); `features/` holds capabilities several pages and
  workspaces show; `pages/` holds the platform pages, one crate per navigation area;
  `workspaces/` holds the standalone workspaces, such as the five crates of the
  [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator) and the debug benches. The
  app `crates/frontend/shell/frontend_application` is the shell: the frame around every route in `src/shell/`, and the routes
  in `crates/frontend/shell/frontend_application/src/app_routes.rs`. A crate depends only on the layers below it (foundation
  < features < pages, workspaces < shell), page crates never depend on each other, and inside a
  layer folder a crate depends only on the crates before it in its crate order (foundation:
  `frontend_ui` < `frontend_api_dtos` < {`frontend_transport`, `frontend_route_table`} <
  `frontend_session` < {`frontend_offline`, `frontend_map_view`}; the Mission Creator:
  `mission_creator_state` < `mission_creator_engine_bridge` < `mission_creator_session` <
  `mission_creator_arsenal` < `mission_creator_workspace`); `frontend_test_support` is only a
  dev-dependency. No frontend crate names `wgpu`: the GPU is reached only through
  `map_renderer`, `paper_doll_renderer` and `gpu_frame`'s frame pump. Gate: CI-SCRIPT, the wgpu
  firewall of `cargo xtask verify crate-tiers` for the crate wall; the layer and crate orders are
  checked on demand by `cargo xtask verify frontend-layering`
  ([Crate boundary rules](/documentation/standards/crate_boundary_rules.md)).

## Errors and logging

- **TS-4 (Usability) — A failed request shows the user an error.** Rust form: the client turns an
  error body into a message with `error_body_message` in
  `crates/frontend/foundation/frontend_transport/src/client/errors.rs`, which appends up to six `details`
  lines when `details` is an array of strings; each page renders that message in its error
  state. Status: live, unenforced.
- **TS-7 (Usability) — No failure is swallowed.** Rust form: a `Result` is surfaced, retried or
  handled; discarding one is a compile warning (`unused_must_use`), and an explicit discard needs
  a reason beside it. Status: live; gate: CI-SCRIPT, the frontend lane's clippy runs deny warnings
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
