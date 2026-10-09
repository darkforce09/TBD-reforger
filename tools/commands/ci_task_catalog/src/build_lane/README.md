# Build and development-server commands

The `cargo xtask mk` lane: named recipes that build, lint, test and serve the website API, the
single-page app and the engine crates, plus two targets that report on or clean the shared cargo
target directory. Developers run it locally, and the CI workflows run several targets by name.

## Contents

```text
tools/commands/ci_task_catalog/src/build_lane/
├── mod.rs      the module tree
├── recipes/    the `mk` entry point, every recipe's step list and the step runner
└── recipes.rs  the `Step` type with its echo line, the `TARGETS` list and the recipe folder constants
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

## Commands

### mk

- Synopsis: `cargo xtask mk <target> [--dry-run]`; `-n` is the same as `--dry-run`, which prints
  the recipe lines without running them. The flag has no effect on the two targets that
  compute or delete: `mk reclaim-target-ci --dry-run` deletes as it would without it.
- Does: runs one target:

  | Target | Recipe |
  |---|---|
  | `print-cargo-target-dir` | prints the resolved shared target directory |
  | `reclaim-target-ci` | deletes the primary checkout's `target/ci/` and the retired root-level `target-ci/`, refusing any other path |
  | `rust-api` | `cargo run --bin api-server` in `crates/api/api_server`; stays in the foreground |
  | `rust-build` | `cargo build -p api_server -p <every crates/api package> --all-targets`, the packages derived from the workspace |
  | `rust-test` | `cargo test -p api_server -p <every crates/api package> --lib --bins`, no database |
  | `rust-fmt` | `cargo fmt --check` in `crates/api/api_server`, then `cargo fmt --all --check` |
  | `rust-clippy` | `cargo clippy -p api_server -p <every crates/api package> --all-targets -- -D warnings` |
  | `rust-ci` | `rust-fmt`, `rust-clippy`, `wasm-ci`, then `cargo xtask db test-it` in process: the API's complete integration suite against a fresh database; needs `cargo xtask db up` |
  | `wasm-ci` | clippy for `wasm32-unknown-unknown` of every package `wasm32_lint_lane.rs` derives outside the frontend family (the family's, the offline service worker's among it, is `ci-local-leptos`'s) |
  | `leptos` | `trunk serve --release` in `crates/frontend/shell/frontend_application`; stays in the foreground on :3000 |
  | `leptos-debug` | `trunk serve`, a debug build; stays in the foreground |
  | `leptos-build` | `trunk build --release` into `crates/frontend/shell/frontend_application/dist` |
  | `gate-doctor` | `leptos-build`, then `gate doctor` from `developer_tools` |
  | `leptos-gates` | `leptos-build` once, `gate doctor`, then `gate editor-suite` (selfcheck, editor, save-export, undo) |
  | `mortar-offline-gate` | `leptos-build`, then `gate mortar-offline`: the mortar calculator's offline pack, a reload with the server gone, and the page's solution against the native one; needs the Everon tile index and the recorded catalog reads |
  | `ballistics-wasm-agreement` | `leptos-build`, then `gate ballistics-agreement`: the seeded agreement cases solved by the browser bench `/debug/ballistics-agreement` against the native solves, one `case ballistics_wasm_agreement_<id>` line each; needs the recorded catalog reads |
  | `ci-local-leptos` | fmt, clippy of all targets with `-D warnings` for wasm32 and natively, and native tests over the frontend family (`frontend_application` and every crates/frontend package, the offline service worker among them, derived from the workspace), then `trunk build --release` |

- Exit codes: 0 done, or a dry run printed; 2 no target, or an unknown one (`--list` with no
  target exits 0); the failing step's own code; 1 an ABI refusal or a spawn error; 127 a tool
  that is not installed; 128 plus the signal number for a step killed by a signal.
- Example: `cargo xtask mk rust-ci --dry-run`

## Boundaries

- Depends on: `tools/commands/ci_task_catalog/src/cargo_target_pin.rs` (the pin, the ABI guard) and
  `tools/commands/ci_task_catalog/src/ci_scratch_reclaim.rs` (`reclaim-target-ci`); `verification_core` for verdicts; cargo, trunk
  and, through `gate`, the `developer_tools` crate; the database lane's `db test-it`
  (`tools/commands/database_operations/src/local_database/test_it.rs`) for `rust-ci`'s integration tests, which
  resolves the container runtime and reaches the `tbd_reforger_db` container.
- Used by:
  - `tools/xtask/src/cli/dispatch.rs`, for `cargo xtask mk`;
  - `tools/commands/ci_task_catalog/src/task_runner/split_cmd.rs`, whose `help` prints `TARGETS`;
  - `.github/workflows/ci.yml` (`rust-fmt`, `rust-clippy`, `wasm-ci`, `ci-local-leptos`) and `.github/workflows/editor-gates.yml` (`gate-doctor`, `leptos-gates`);
  - people, for the servers and the rest.
- Rules: a target in `TARGETS` must dispatch; only `rust-api` sets its own target directory; the
  printed line is rendered from the fields that run; no recipe line names a container runtime, so
  the database is reached through the database lane only. The rows `rust-ci`, `rust-fmt`,
  `rust-clippy`, `rust-test`, `wasm-ci`, `ci-local-leptos`, `leptos-build` and `rust-test-it` of
  the `cargo xtask ci` table repeat these recipes, so a change here is made there too.

## Related documentation

- [Local development](/documentation/runbooks/local_development.md) — running the API and the
  app locally.
- [Database operations](/documentation/runbooks/database_operations.md) — the database the
  API and `rust-ci` run against.
- [Editor gates](/documentation/runbooks/editor_gates.md) — `mk gate-doctor` and
  `mk leptos-gates`.
- [Editor capture](/documentation/runbooks/editor_capture.md) — screenshots of the app that
  `mk leptos` or `mk leptos-debug` serves.
