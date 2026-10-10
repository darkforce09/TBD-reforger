# CI task catalog

The `ci_task_catalog` crate: the `cargo xtask ci`, `cargo xtask help` and `cargo xtask mk` lanes.
It holds the table of named CI tasks (the local CI replay `ci-local`, the schema gates, the
verifications and the map composites) with the runner that executes them, the `mk` recipes that
build, test and serve the API, the single-page app and the engine crates, the shared cargo target
pin, the map asset checks the schema and terrain tasks run, and the `mk` helpers the central
ticket manager's slice and wave gates run (`wave_gate_steps/`, named as steps in
[`ticket_manager_execution.toml`](/ticket_manager_execution.toml)). Developers run `ci-local`
before pushing; the GitHub workflows run single tasks and recipes by name.

## Contents

```text
tools/commands/ci_task_catalog/
├── Cargo.toml  the `ci_task_catalog` library package: the check, schema and database crates, `map_asset_verification` for the map asset checks, layout tier 8
└── src/        the task table and runner, the build lane, the target pin, the map asset steps, the wave gate steps and the errors
```

## How it works

The xtask binary parses the command line and calls the crate: `ci <task>` reaches
`task_runner::run`, `help` reaches `task_runner::help`, and `mk <target>` reaches
`build_lane::recipes::run` with its raw arguments. A task's steps are other tasks by name,
subprocess lines with inherited stdio, or in-process calls into the check, schema and database
crates; the runner stops at the first red step and returns its exit code. An in-process step that
cannot run prints `xtask: ` and its error with every cause (`cause_chain`), the line the binary
prints for the same verb.

Every child `cargo` the lanes start gets the shared `CARGO_TARGET_DIR` from `cargo_target_pin`
(the primary checkout's `target/`, shared by every linked worktree, unless the caller exports
another).

The commands themselves are described in the
[source README](/tools/commands/ci_task_catalog/src/README.md) (`ci`, `help`) and the
[build lane README](/tools/commands/ci_task_catalog/src/build_lane/README.md) (`mk`).

## Boundaries

- Depends on: `repository_checks`, `schema_tooling`, `database_operations`, `repository_laws`,
  `repository_layout`, `process_runner`, `verification_core`, `libc`, `serde`, `serde_json`,
  `thiserror`; `map_asset_verification` for the map asset checks; cargo, trunk, git, git-lfs, go,
  curl, unzip and apt-get as subprocesses.
- Used by: the xtask binary's `ci`, `help`, `mk` and `schema` groups; the GitHub workflows,
  through `cargo xtask ci` and `cargo xtask mk`; the central ticket manager's runner, through the
  `cargo xtask mk` helpers its configuration names (`changed-packages`, `gate-step <name>`,
  `preflight-api-freshness`, `target-abi-guard`).
- Rules: tier 8 of `tools/commands` (`cargo xtask verify crate-tiers`); the crate never reads the
  command line; a composite runs the very rows it names; every workspace member is tested by some
  CI task (the API family by `api-test`, the frontend family by `ci-local-leptos`, every other
  member by `workspace-member-tests`).

## Related documentation

- [Testing and CI](/documentation/runbooks/testing_and_ci.md) — running `ci-local` and single
  tasks, and where each gate runs.
- [Local development](/documentation/runbooks/local_development.md) — the local stack the `mk`
  recipes build and serve.
