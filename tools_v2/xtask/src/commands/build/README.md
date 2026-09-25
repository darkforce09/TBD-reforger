# Build and development-server commands

The `cargo xtask mk` lane: named recipes that build, lint, test and serve the website API, the
single-page app and the engine crates, plus three targets that report on or clean the shared cargo
target directory. Developers run it locally, and the CI workflows run several targets by name.

## Contents

```text
tools_v2/xtask/src/commands/build/
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
`tools_v2/xtask/src/core/cargo_target_directory.rs` (the primary checkout's `target/`, shared by
every linked worktree, unless the caller exports another), except `rust-api`, which builds into
`target-dev-api` in the current checkout so a running server never waits on the shared build lock.
Before a `cargo` or `trunk` step, `abi_guard` refuses a target directory stamped by another glibc.

The tests in `tests/recipes.rs` are wired in from `tools_v2/xtask/src/core/cargo_target_directory.rs`,
since half of them cover that module's pin.

## Commands

### mk

- Synopsis: `cargo xtask mk <target> [--dry-run]`; `-n` is the same as `--dry-run`, which prints
  the recipe lines without running them. The flag has no effect on the three targets that
  compute or delete: `mk reclaim-target-ci --dry-run` deletes as it would without it.
- Does: runs one target:

  | Target | Recipe |
  |---|---|
  | `print-cargo-target-dir` | prints the resolved shared target directory |
  | `verify-cargo-target` | checks that the shared target-directory pin is intact and that `rust-build` sets no directory of its own |
  | `reclaim-target-ci` | deletes the primary checkout's `target-ci/`, refusing any other path |
  | `rust-api` | `cargo run --bin api` in `apps/website/api_v2`; stays in the foreground |
  | `rust-build` | `cargo build --all-targets` in `apps/website/api_v2` |
  | `rust-test` | `cargo test --lib --bins` in `apps/website/api_v2`, no database |
  | `rust-fmt` | `cargo fmt --check` in `apps/website/api_v2`, then `cargo fmt --all --check` |
  | `rust-clippy` | `cargo clippy --all-targets -- -D warnings` in `apps/website/api_v2` |
  | `rust-sqlx-prepare` | `cargo sqlx prepare` in `apps/website/api_v2` |
  | `rust-ci` | `rust-fmt`, `rust-clippy`, `rust-build`, `wasm-ci`, then the integration tests against a fresh `rust_it` database; needs `cargo xtask db up` |
  | `wasm-ci` | fmt, clippy (native with all features, and `wasm32-unknown-unknown`) and tests of `website-map-engine` and `website-graphics-engine` |
  | `leptos` | `trunk serve --release` in `apps/website/frontend`; stays in the foreground on :3000 |
  | `leptos-debug` | `trunk serve`, a debug build; stays in the foreground |
  | `leptos-build` | `trunk build --release` into `apps/website/frontend/dist` |
  | `gate-doctor` | `leptos-build`, then `gate doctor` from `developer-tools` |
  | `leptos-gates` | `leptos-build` once, `gate doctor`, `gate editor-suite` and `gate v-suite verify` |
  | `ci-local-leptos` | `website-frontend` fmt, wasm32 clippy of all targets, native tests, then `trunk build --release` |

- Exit codes: 0 done, or a dry run printed; 2 no target, or an unknown one (`--list` with no
  target exits 0); the failing step's own code; 1 an ABI refusal or a spawn error; 127 a tool
  that is not installed; 128 plus the signal number for a step killed by a signal.
- Example: `cargo xtask mk rust-ci --dry-run`

## Boundaries

- Depends on: `tools_v2/xtask/src/core/cargo_target_directory.rs` (the pin, the ABI guard,
  `verify-cargo-target` and `reclaim-target-ci`); `verification_core` for verdicts; cargo, trunk,
  podman and, through `gate`, the `developer-tools` crate; the `tbd_reforger_db` container for
  `rust-ci`.
- Used by:
  - `tools_v2/xtask/src/cli/dispatch.rs`, for `cargo xtask mk`;
  - `tools_v2/xtask/src/commands/ci/task_runner/split_cmd.rs`, whose `help` prints `TARGETS`, and
    `tools_v2/xtask/src/core/cargo_target_directory.rs`, which checks `rust-api` and `rust-build`;
  - `.github/workflows/ci.yml` (`rust-fmt`, `rust-clippy`, `rust-build`, `wasm-ci`,
    `ci-local-leptos`) and `.github/workflows/editor-gates.yml` (`gate-doctor`, `leptos-gates`);
  - people, for the servers and the rest.
- Rules: a target in `TARGETS` must dispatch (`every_advertised_target_dispatches`); only
  `rust-api` sets its own target directory (`only_rust_api_sets_a_private_target_dir`); the
  printed line is rendered from the fields that run (`echo_matches_make`). The rows `rust-ci`,
  `rust-fmt`, `rust-clippy`, `rust-build`, `rust-test`, `wasm-ci`, `ci-local-leptos` and
  `leptos-build` of the `cargo xtask ci` table repeat these recipes, and no test compares the two
  copies, so a change here is made there too.

## Related documentation

- [Local development](/documentation_v2/runbooks/local_development.md) — running the API and the
  app locally.
- [Editor gates](/documentation_v2/runbooks/editor_gates.md) — `mk gate-doctor` and
  `mk leptos-gates`.
