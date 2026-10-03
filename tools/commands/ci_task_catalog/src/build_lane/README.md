# Build and development-server commands

The `cargo xtask mk` lane: named recipes that build, lint, test and serve the website API, the
single-page app and the engine crates, plus three targets that report on or clean the shared cargo
target directory. Developers run it locally, and the CI workflows run several targets by name.

## Contents

```text
tools/commands/ci_task_catalog/src/build_lane/
├── mod.rs      the module tree
├── recipes/    the `mk` entry point, every recipe's step list and the step runner
├── recipes.rs  the `Step` type with its echo line, the `TARGETS` list and the recipe folder constants
└── tests/      unit tests for the recipe echoes, the target-dir pin, reclaim and target dispatch
```

## How it works

`cargo xtask mk` reaches `run` in `recipes/execution.rs` with its raw arguments; clap does not
parse them, so `TARGETS` in `recipes.rs` is the one list of names, and `mk` with no target prints
it. A recipe is a list of `Step`s, each a working folder, recipe-level environment and an argv.
The runner prints each step's line before running it and stops at the first failure. Every child
`cargo` and `trunk` gets the shared `CARGO_TARGET_DIR` from
`tools/commands/ci_task_catalog/src/cargo_target_pin.rs` (the primary checkout's `target/`, shared by
every linked worktree, unless the caller exports another), except `rust-api`, which builds into
`target/dev-api` in the current checkout so a running server never waits on the shared build lock.
Before a `cargo` or `trunk` step, `abi_guard` refuses a target directory stamped by another glibc.

The tests in `tests/recipes.rs` are wired in from `recipes.rs`; the target-directory checks the
recipes dispatch to are tested beside them, in `tools/commands/ci_task_catalog/src/tests/`.

## Commands

### mk

- Synopsis: `cargo xtask mk <target> [--dry-run]`; `-n` is the same as `--dry-run`, which prints
  the recipe lines without running them. The flag has no effect on the three targets that
  compute or delete: `mk reclaim-target-ci --dry-run` deletes as it would without it.
- Does: runs one target:

  | Target | Recipe |
  |---|---|
  | `print-cargo-target-dir` | prints the resolved shared target directory |
  | `verify-cargo-target` | checks that the shared target-directory pin is intact, that `rust-build` sets no directory of its own and that `rust-api` builds into this checkout's `target/dev-api` |
  | `reclaim-target-ci` | deletes the primary checkout's `target/ci/` and the retired root-level `target-ci/`, refusing any other path |
  | `rust-api` | `cargo run --bin api` in `apps/api`; stays in the foreground |
  | `rust-build` | `cargo build -p api -p <every crates/api package> --all-targets`, the packages derived from the workspace |
  | `rust-test` | `cargo test -p api -p <every crates/api package> --lib --bins`, no database |
  | `rust-fmt` | `cargo fmt --check` in `apps/api`, then `cargo fmt --all --check` |
  | `rust-clippy` | `cargo clippy -p api -p <every crates/api package> --all-targets -- -D warnings` |
  | `rust-ci` | `rust-fmt`, `rust-clippy`, `rust-build`, `wasm-ci`, then `cargo xtask db test-it` in process: the API's complete integration suite against a fresh database; needs `cargo xtask db up` |
  | `wasm-ci` | fmt, native clippy with all features and tests of `offline_service_worker`; clippy for `wasm32-unknown-unknown` of every package `wasm32_lint_lane.rs` derives (the frontend's is `ci-local-leptos`'s) |
  | `leptos` | `trunk serve --release` in `apps/frontend`; stays in the foreground on :3000 |
  | `leptos-debug` | `trunk serve`, a debug build; stays in the foreground |
  | `leptos-build` | `trunk build --release` into `apps/frontend/dist` |
  | `gate-doctor` | `leptos-build`, then `gate doctor` from `developer_tools` |
  | `leptos-gates` | `leptos-build` once, `gate doctor`, `gate editor-suite` and `gate v-suite verify` |
  | `mortar-offline-gate` | `leptos-build`, then `gate mortar-offline`: the mortar calculator's offline pack, a reload with the server gone, and the page's solution against the native one; needs the Everon tile index and the recorded catalog reads |
  | `ballistics-wasm-agreement` | `leptos-build`, then `gate ballistics-agreement`: the seeded agreement cases solved by the browser bench `/debug/ballistics-agreement` against the native solves, one `case ballistics_wasm_agreement_<id>` line each; needs the recorded catalog reads |
  | `ci-local-leptos` | `frontend` fmt, clippy of all targets with `-D warnings` for wasm32 and natively, native tests, then `trunk build --release` |

- Exit codes: 0 done, or a dry run printed; 2 no target, or an unknown one (`--list` with no
  target exits 0); the failing step's own code; 1 an ABI refusal or a spawn error; 127 a tool
  that is not installed; 128 plus the signal number for a step killed by a signal.
- Example: `cargo xtask mk rust-ci --dry-run`

## Boundaries

- Depends on: `tools/commands/ci_task_catalog/src/cargo_target_pin.rs` (the pin, the ABI guard) and
  `tools/commands/ci_task_catalog/src/cargo_target_verification.rs` (`verify-cargo-target` and `reclaim-target-ci`); `verification_core` for verdicts; cargo, trunk
  and, through `gate`, the `developer_tools` crate; the database lane's `db test-it`
  (`tools/commands/database_operations/src/local_database/test_it.rs`) for `rust-ci`'s integration tests, which
  resolves the container runtime and reaches the `tbd_reforger_db` container.
- Used by:
  - `tools/xtask/src/cli/dispatch.rs`, for `cargo xtask mk`;
  - `tools/commands/ci_task_catalog/src/task_runner/split_cmd.rs`, whose `help` prints `TARGETS`, and
    `tools/commands/ci_task_catalog/src/cargo_target_verification.rs`, which checks `rust-api` and `rust-build`;
  - `.github/workflows/ci.yml` (`rust-fmt`, `rust-clippy`, `rust-build`, `wasm-ci`,
    `ci-local-leptos`) and `.github/workflows/editor-gates.yml` (`gate-doctor`, `leptos-gates`);
  - people, for the servers and the rest.
- Rules: a target in `TARGETS` must dispatch (`every_advertised_target_dispatches`); only
  `rust-api` sets its own target directory (`only_rust_api_sets_a_private_target_dir`); the
  printed line is rendered from the fields that run (`echo_matches_make`); no recipe line names a
  container runtime (`no_recipe_line_names_a_bare_container_runtime`), so the database is reached
  through the database lane only. The rows `rust-ci`, `rust-fmt`, `rust-clippy`, `rust-build`,
  `rust-test`, `wasm-ci`, `ci-local-leptos`, `leptos-build` and `rust-test-it` of the
  `cargo xtask ci` table repeat these recipes. Two pairs are compared by tests (`wasm-ci` in
  `wasm_ci_recipe_and_ci_task_row_run_the_same_lines`, the integration-test step in
  `rust_ci_and_the_ci_rust_test_it_row_run_the_database_lane`); for the others a change here is
  made there too.

## Related documentation

- [Local development](/documentation/runbooks/local_development.md) — running the API and the
  app locally.
- [Database operations](/documentation/runbooks/database_operations.md) — the database the
  API and `rust-ci` run against.
- [Editor gates](/documentation/runbooks/editor_gates.md) — `mk gate-doctor` and
  `mk leptos-gates`.
- [Editor capture](/documentation/runbooks/editor_capture.md) — screenshots of the app that
  `mk leptos` or `mk leptos-debug` serves.
