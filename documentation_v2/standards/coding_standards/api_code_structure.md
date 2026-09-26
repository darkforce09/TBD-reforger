**Status:** live

# API code structure and route tags

Rules GO-1 to GO-9: how the [API](/documentation_v2/glossary/a_to_f.md#api) crate
(`apps/website/api_v2/`, package `website-api`) is laid out, how it treats database errors, and how
its handlers are tagged. The codes keep their GO prefix because the rules were first written for a
Go backend; no Go remains, and each rule now states its Rust form or is retired. GO-7 is live and
gated; the rest are either conventions that the Rust type system, clippy and the crate's layout
tests carry, or retired.

## Layout

A domain's `handlers/` are the HTTP edge, its `services/` the logic core and its `models/` the
snake_case database and wire contract. The eight domains are `administration`, `command_center`,
`community_content`, `identity_and_access`, `match_telemetry`, `missions`, `operations` and
`server_infrastructure`; where each kind of file goes is in
[Where does X go?](/documentation_v2/standards/where_does_x_go.md).

- **GO-1 (Scalability) — Business logic lives in `services/`; handlers do HTTP only.** A handler
  extracts and validates input, checks authorization, calls a service and maps the result to a
  status and a body. Multi-step database work, [ORBAT](/documentation_v2/glossary/n_to_z.md#orbat) materialisation and telemetry math live in
  the domain's `services/`. Status: live, unenforced: no gate measures how much logic a handler
  holds.
- **GO-9 (Scalability) — Handlers reach other code through services and models only.** Rust form:
  `core` imports no domain except its composition root, a domain's handlers never import another
  domain's handlers, `background_workers` is imported only by the binary, every domain exports one
  route table that the router merges, and there is no top-level `handlers/`, `services/` or
  `models/`. Gate: CI-BLOCK, the tests `core_imports_no_domain_except_composition_root`,
  `domain_handlers_import_no_foreign_handlers`, `background_workers_used_only_by_the_binary`,
  `every_domain_exports_a_route_table` and `no_legacy_top_level_modules` in
  [architecture_rules.rs](/apps/website/api_v2/src/tests/architecture_rules.rs), run by the API's
  `cargo test`.

## Errors and lints

- **GO-2 (Debuggability) — A database error whose result is used is handled.** Rust form: a
  `sqlx::Error` propagates with `?` into `ApiError`, which logs it and answers `500 internal
  error`; a handler that owes the client a more specific status maps that case itself. Discarding
  a `Result` is a compile warning (`unused_must_use`), and clippy runs with `-D warnings`.
  Gate: CI-BLOCK, `cargo xtask mk rust-clippy` in the `website-api` job.
- **GO-3 (Debuggability) — A best-effort write logs its failure and says why dropping it is
  safe.** Rust form: the write goes through a function that logs the error and returns nothing,
  as `write_audit` in `apps/website/api_v2/src/administration/services/audit_writer.rs` does,
  while a write that must not happen without its record uses `required_audit.rs` in the same
  transaction; a bare discard (`let _ = …` or `.ok()`) carries a comment giving the reason.
  Status: live, unenforced: clippy accepts an explicit discard without a comment.
- **GO-4 (Debuggability) — A propagated error keeps its cause.** Rust form: `?` with
  `anyhow::Context` or a typed error that wraps the source. Status: retired as a separate rule;
  the Rust type system carries it.
- **GO-5 (Usability) — A unique-constraint clash answers `409` through SQLSTATE `23505`, never a
  string match.** Rust form: `is_unique_violation` in
  [postgres_errors.rs](/apps/website/api_v2/src/core/database/postgres_errors.rs), then
  `ApiError::conflict`. Status: live, unenforced: no gate finds a handler that string-matches an
  error message.
- **GO-6 (Readability) — Every public item carries a doc comment.** Rust form: the `///` rules of
  the [documentation standards](/documentation_v2/standards/documentation_standards.md). Status:
  retired as a code rule; the documentation standards own it.
- **GO-8 (Debuggability) — The static analyser runs with every check on.** Rust form: clippy with
  `-D warnings` over `--all-targets`; the generated contract types under
  `apps/website/api_v2/src/missions/contract/generated/` are exempt from the crate's prose tests
  but not from clippy. Gate: CI-BLOCK, `cargo xtask mk rust-clippy`.

The `website-api` job labels its clippy step "GO-2..8 analog" and its format step "FMT-1 analog":
clippy and `cargo fmt` stand in for the Go-era rules GO-2 to GO-8 and FMT-1 together.

## Route tags

- **GO-7 (Readability) — Every handler that a route table registers carries `@route` in its doc
  comment, and the tag matches the wired route.** The route side is the eight
  `apps/website/api_v2/src/<domain>/routes.rs` tables that `api_v1_routes` in
  [http_router.rs](/apps/website/api_v2/src/core/http_router.rs) merges under `/api/v1`. The
  check runs in both directions: every `/// @route METHOD PATH` tag names a route registered on
  that method for that handler, and every registered route carries a matching tag, keyed on
  method, path and handler function. Gate: CI-SCRIPT, `cargo xtask verify route-tags`, run by
  `cargo xtask ci verify-coding-standards` and by the platform
  [wave](/documentation_v2/glossary/n_to_z.md#wave) gate and slice gate.

GO-7 is the one GO rule clippy and `cargo fmt` cannot see: `@route` lives in a doc comment, clippy
does not read doc comments and `cargo fmt` only reflows them. A tag that names a route which does
not exist, or a route whose handler has no tag, compiles cleanly; only the route-tag gate turns it
red. The gate's guards and exit codes are in the
[architecture verifications README](/tools_v2/xtask/src/verifications/architecture/README.md#route-tags);
the tag grammar is in the [documentation standards](/documentation_v2/standards/documentation_standards.md).

## Forbidden

- Business logic or multi-table SQL inline in a handler where a service would carry it (GO-1,
  GO-9).
- A discarded database or write error with no stated reason (GO-2, GO-3).
- `panic!`, `unwrap()` or `expect()` on a value a request controls, on a request path. Status:
  live, unenforced; clippy's default lints allow `unwrap()`.
