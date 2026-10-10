# Wave gate steps

The repository-specific halves of the slice and wave gates the central ticket manager runs
(`ttm wave gate`, configured by [`ticket_manager_execution.toml`](/ticket_manager_execution.toml)).
The ticket manager owns the generic runner (worktrees, the gate lock, step capture, verdict
receipts, base derivation, landing, the close ceremony); this folder holds what only this
repository's Rust code can compute, as `cargo xtask mk` helper commands.

## Contents

```text
tools/commands/ci_task_catalog/src/wave_gate_steps/
├── mod.rs                    helper dispatch (`mk changed-packages`, `mk gate-step <name>`, …), output macros
├── step_context.rs           the step context from the runner's environment, git reads, the gate lock probe
├── host.rs                   the container-to-host bridge of a step's own children
├── changed.rs                the change-scoped readers' constants and re-exports
├── changed/                  changed files, rustfmt per edition, the wasm scope, include inputs
├── changed_packages.rs       `mk changed-packages`: files, packages and the scopes, as JSON
├── touch.rs                  fingerprint invalidation (`touch-changed`, `touch-workspace`)
├── clippy_lanes.rs           the clippy lanes and the frontend and workspace test steps
├── gate_database.rs          the per-wave gate database and `test-api` (skips are red)
├── migration_persistence.rs  the persistent migration database step; its parts in migration_persistence/
├── trunk_build.rs            `trunk build --release` into the gate's private folders
├── schema_step.rs            the `ci schema-validate` sub-gates with a content-stamped xtask build
└── api_freshness.rs          `mk preflight-api-freshness`: the API on :8080 is up and current
```

## Commands

Each prints its own output and exits 0 (pass), 1 (red) or 2 (refused argument).

- `cargo xtask mk changed-packages [--range <range>]` — JSON with `committed_files`,
  `working_tree_files`, `rust_files`, `package_dirs`, `packages` and `scopes.wasm_scope` /
  `scopes.frontend_tests` (`touched`, `detail`). The runner reads the scopes for steps with
  `when_scope`.
- `cargo xtask mk gate-step <name>` — one of `touch-changed`, `touch-workspace`, `fmt-changed`,
  `frontend-tests-changed --slice <slice>`, `clippy-native`, `clippy-wasm32`, `clippy-frontend`,
  `clippy-frontend-native`, `clippy-tools`, `gate-database`, `migrate-persist <audit|advance>`,
  `test-api`, `test-frontend`, `test-workspace-members`, `trunk-build`, `schema`. The range comes
  from `--range` or `TTM_GATE_RANGE`, the slice from `--slice` or `TTM_SLICE`.
- `cargo xtask mk preflight-api-freshness` — the preflight check of the API on `:8080`.
- `cargo xtask mk target-abi-guard <folder>` — refuses a build folder stamped by the other glibc
  (the run lane's guard).

## How it works

The runner exports `TTM_ROOT`, `TTM_MAIN_ROOT`, `TTM_GATE_RANGE`, `TTM_GATE_BASE`, `TTM_SLICE`,
`TTM_WAVE`, `TTM_GATE_LOCK`, `TTM_GATE_TIMEOUT` and `CARGO_TARGET_DIR` (the shared folder of the
toolchain environment the step runs in) to every step, across the host bridge too. A step run by
hand falls back to git for the checkouts and to `target/<environment>/` for the folders. Each
build-heavy step builds into its own `target/<environment>/gate-*` folder.

## Boundaries

- Depends on: `repository_laws` (workspace members, manifests), `repository_layout`
  (`build_output`), `process_runner`, `verification_core`, the sibling lanes of this crate
  (`frontend_package_lane`, `wasm32_lint_lane`, `api_package_lane`, `workspace_member_tests`,
  `task_runner`, `cargo_target_pin`); `git`, `cargo`, `rustfmt`, `trunk`, `podman exec
  tbd_reforger_db psql`, `sha384sum`.
- Used by: `build_lane::recipes::run` (the `mk` verb) and, through it, the ticket manager's runner.
- Rules: a step that could not examine its input is red; the destructive database steps refuse
  unless a gate holds the lock (`TTM_GATE_LOCK`, probed) or the operator set
  `TBD_GATE_ALLOW_UNSERIALISED=1`; a database test that skipped is red.
