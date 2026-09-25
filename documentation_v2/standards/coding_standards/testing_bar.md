**Status:** live

# Testing bar

Rules TEST-1 to TEST-3: the least testing a change of each layer ships with. How to run each suite,
and which job runs it where, is in [Testing and CI](/documentation_v2/runbooks/testing_and_ci.md);
the integration test database and its options are in
[Database operations](/documentation_v2/runbooks/database_operations.md#run-the-integration-tests).

## Rules

- **TEST-1 (Debuggability) — A change to a handler's behaviour ships with the
  [API](/documentation_v2/glossary.md#api)'s tests green against Postgres.** The integration tests
  in `apps/website/api_v2/tests/` run against a real database; a clean compile is not proof of the
  HTTP contract. Locally: `cargo xtask db test-it` (a new randomly named database, dropped at the
  end) or `cargo xtask ci rust-test-it`, the `ci-local` step, both after `cargo xtask db up`. Gate:
  CI-BLOCK, the `website-api` job of `.github/workflows/ci.yml`, whose test step runs
  `cargo xtask ci website-api-test` (the API's `cargo test`, unit and integration) against a
  Postgres 18 service.
- **TEST-2 (Debuggability) — Non-trivial frontend logic has a unit test.** Compilers, selectors,
  transforms and DTO shapes are tested in `website-frontend`. Gate: CI-BLOCK,
  `cargo test -p website-frontend` inside `cargo xtask mk ci-local-leptos`, the `website-frontend`
  job.
- **TEST-3 (Usability) — A schema or DTO change ships a golden fixture and a green schema gate.**
  A change under `contracts_v2/definitions/` comes with its fixture under `contracts_v2/fixtures/`
  and regenerated contract types. Gate: CI-BLOCK, `cargo xtask ci schema-validate` inside
  `cargo xtask ci ci-local-schema`, the `schema` job; `cargo xtask ci verify-codegen-fresh` in the
  same task fails when the generated types are stale.

The map and graphics engines have no rule code of their own; `cargo xtask mk wasm-ci` runs their
tests in the `map-engine` job. The browser gates of the
[Mission Creator](/documentation_v2/glossary.md#mission-creator) (`cargo xtask mk leptos-gates`) run
outside `ci-local`, as described in [Editor gates](/documentation_v2/runbooks/editor_gates.md).

## Test placement

Unit tests live in sibling files under a `tests/` folder, declared with
`#[cfg(test)] #[path = "tests/<file>.rs"] mod tests;`; an inline `mod tests { … }` body is
forbidden (CLAUDE.md law 7). Three test suites hold the rule, each in the crate's own
`cargo test` (CI-BLOCK):

- the API: `no_inline_test_modules` in
  [architecture_rules.rs](/apps/website/api_v2/src/tests/architecture_rules.rs);
- the app's `src/v2/` tree: `v2_production_files_meet_the_documentation_standard` in
  `apps/website/frontend/src/v2/tests/doc_audit/mod.rs`;
- the four `tools_v2` crates: `tooling_test_modules_live_in_separate_files` in
  `tools_v2/xtask/src/tests/tooling_dependency_boundaries.rs`.

The map engine, the graphics engine, the ticketboard and the fleet host agent are unenforced. A
test file may hold 1000 lines; see
[File size and complexity](/documentation_v2/standards/coding_standards/file_size_and_complexity.md).
