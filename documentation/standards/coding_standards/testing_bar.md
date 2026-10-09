**Status:** live

# Testing bar

Rules TEST-1 to TEST-3: the least testing a change of each layer ships with. How to run each suite,
and which job runs it where, is in [Testing and CI](/documentation/runbooks/testing_and_ci.md);
the integration test database and its options are in
[Database operations](/documentation/runbooks/database_operations.md#run-the-integration-tests).

## What to test (pre-alpha)

The repository is pre-alpha, so tests are kept to the logic whose silent break costs most
(`CLAUDE.md` law 11):

- **Test:** math and geometry, file and wire formats, CRDT merge and undo, auth and permissions,
  mission compile and validation, data integrity, and guards on destructive operations.
- **Never test:** source text (no `include_str!` scans of production code), prose and wording, CSS
  classes, constants, file layout, or `Debug`/`Display` output.
- **API:** one integration test binary per domain, each provisioning its database once; no
  failpoint suites and no property-evidence suites.
- **Slow suites** (headless browser gates, mod world boot) run nightly or on demand, never on every
  commit.
- **Before committing:** `cargo xtask mk rust-fmt`, `cargo xtask mk rust-clippy`, and the tests of
  the crates you touched.

## Rules

- **TEST-1 (Debuggability) — A change to a handler's behaviour ships with the
  [API](/documentation/glossary/a_to_f.md#api)'s tests green against Postgres.** The integration tests
  in `crates/api/api_server/tests/` run against a real database; a clean compile is not proof of the
  HTTP contract. Locally: `cargo xtask db test-it` (a new randomly named database, dropped at the
  end) or `cargo xtask ci rust-test-it`, the `ci-local` step, both after `cargo xtask db up`. Gate:
  CI-BLOCK, the `api` job of `.github/workflows/ci.yml`, whose test step runs
  `cargo xtask ci api-test` (the API's `cargo test`, unit and integration) against a
  Postgres 18 service.
- **TEST-2 (Debuggability) — Non-trivial frontend logic has a unit test.** Compilers, selectors,
  transforms and DTO shapes are tested in the frontend crates. Gate: CI-BLOCK, `cargo test` over
  the frontend family (`-p frontend_application` and every other `crates/frontend` package) inside
  `cargo xtask mk ci-local-leptos`, the `frontend` job.
- **TEST-3 (Usability) — A schema or DTO change ships a golden fixture and a green schema gate.**
  A change under `contracts/definitions/` comes with its fixture under `contracts/fixtures/`
  and regenerated contract types. Gate: CI-BLOCK, `cargo xtask ci schema-validate` inside
  `cargo xtask ci ci-local-schema`, the `schema` job; `cargo xtask ci verify-codegen-fresh` in the
  same task fails when the generated types are stale.

The map crates have no rule code of their own; the `workspace-members` job runs their tests in one
`cargo test --workspace` run, and `cargo xtask mk wasm-ci` lints the wasm32 ones in the
`wasm-ci` job. The browser gates of the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator) (`cargo xtask mk leptos-gates`) run
outside `ci-local`, nightly or on demand, as described in
[Editor gates](/documentation/runbooks/editor_gates.md).

## Test placement

Unit tests live in sibling files under a `tests/` folder, declared with
`#[cfg(test)] #[path = "tests/<file>.rs"] mod tests;`, rather than an inline `mod tests { … }`
body. This is a convention (CLAUDE.md law 7), not a gate. A test file stays around 1000 lines; see
[File size and complexity](/documentation/standards/coding_standards/file_size_and_complexity.md).
