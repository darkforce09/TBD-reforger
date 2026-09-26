# CI task commands

The `cargo xtask ci` lane and `cargo xtask help`: one table of named tasks (the local CI replay
`ci-local`, the schema gates, the language, coding-standard and documentation verifications, the
CI workflow helpers and the map asset composites) and the runner that executes a task's steps.
Developers run `ci-local` before pushing, and the GitHub workflows run single tasks by name.

## Contents

```text
tools_v2/xtask/src/commands/ci/
├── chromium_install.rs  the `ci-chrome` task: installs the pinned Chrome for Testing build
├── editor_api.rs        the `editor-api-boot`, `verify-codegen-fresh` and `verify-editorconfig` tasks
├── mod.rs               the module tree
├── task_definitions/    the step macros and the in-process verification adapters
├── task_definitions.rs  `TASKS`: every task with its help line, group, lane and steps
├── task_runner/         the runner, the child environment, `help`, the gate list
├── task_runner.rs       the `Task`, `Step` and `Lane` types, re-exports
└── tests/               unit tests for the frozen `ci-local` set, composite failure, `help` and parity
```

## How it works

`TASKS` in `task_definitions.rs` is pure data and `run_task_in` in `task_runner/split_cmd.rs` is its
only interpreter. A step is another task by name (`Step::Task`, so a composite runs the very row
its standalone command runs), a subprocess line, an in-process xtask call, a native Rust step, or
a `/bin/sh -c` script, which only borrowed rows use. The runner prints each step's line before it
runs (a native step prints its own), stops at the first non-zero code and returns that code.

Each row carries a lane, which `help` prints as a tag:

- `Ci`: this table owns the name and the steps.
- `Alias`: a one-line wrapper on an existing `cargo xtask verify …` command.
- `Borrowed`: a build or database recipe repeated here so the composites run it: the
  `cargo xtask mk` recipes `rust-ci`, `rust-fmt`, `rust-clippy`, `rust-build`, `rust-test`,
  `wasm-ci`, `ci-local-leptos` and `leptos-build`, and the `rust-test-it` integration run.

`ci-local` runs, in this order: `verify-editorconfig`, `verify-no-python`, `verify-no-node`,
`verify-no-shell`, `verify-ci-shell`, `verify-engine-layers`, `rust-ci`, `verify-coding-standards`,
`verify-documentation`, `ci-local-leptos`, `ci-local-schema`, `verify-staging-compose-paths`,
`verify-mission-rest-size-limits`, and `cargo xtask verify ci-schema-parity` in process.
`ci_local_step_set_is_frozen` in `tests/task_runner.rs` fails when a step is added, dropped or
moved. The browser gates of `cargo xtask mk leptos-gates` are not part of it.

## Commands

### ci

- Synopsis: `cargo xtask ci [<task>]`
- Does: runs one task of the table below; with no task, prints the same listing as `help`.

  | Task | Group, lane | Runs |
  |---|---|---|
  | `ci-local` | CI, ci | the composite above; needs `cargo xtask db up` first |
  | `ci-local-schema` | CI, ci | `verify-codegen-fresh`, `schema-validate`, `verify-citations` |
  | `ci-chrome` | CI, ci | the Chrome for Testing version from `tools_v2/developer-tools/gate-env.json` (or `CHROME_VERSION`), its system libraries through `sudo apt-get`, unpacked into `~/cft`; prints `CHROME_HEADLESS_SHELL=` and appends it to `GITHUB_ENV` when set |
  | `editor-api-boot` | CI, ci | builds and starts the `api` binary of `website-api` in its own session, logging to `/tmp/api.log`, and waits up to 60 s for `GET /healthz` on `PORT` (8080); the API keeps running |
  | `schema-validate` | schema, ci | `schema validate`, `map-object-golden`, `map-glyphs`, `height-labels`, `map-object-enums`, `type-inventory`, in process |
  | `schema-codegen` | schema, ci | `schema codegen`: regenerates the contract types from `contracts_v2/definitions/` |
  | `verify-citations` | schema, ci | `schema citations`: the `@contract` citations in code |
  | `verify-codegen-fresh` | schema, ci | regenerates the contract outputs in memory and compares them with the files |
  | `verify-coding-standards` | verify, ci | `verify file-length`, `verify no-select-star` and `verify route-tags`, in process |
  | `verify-documentation` | verify, ci | `verify readme-coverage`, `verify link-check` and `verify markdown-placement` over the committed tree, in process |
  | `verify-editorconfig` | verify, ci | `editorconfig-checker` from the root, installing the pinned v3.4.0 with `go install` when absent |
  | `verify-no-python`, `verify-no-node`, `verify-no-shell`, `verify-ci-shell`, `verify-engine-layers`, `verify-staging-compose-paths`, `verify-mission-rest-size-limits` | verify, alias | the `cargo xtask verify` command of the same name |
  | `verify-terrain` | verify, ci | `schema terrain-manifest` and `schema terrain-alignment` for Everon |
  | `verify-terrain-strict` | verify, ci | the same with `terrain-alignment --strict` |
  | `map-water-everon` | map, ci | restores the pre-water Everon ortho from `assets_v2/scratch/`, resets the water metadata, analyses and composites the water, rebuilds the satellite container and tile pyramid, then verifies all three |
  | `map-cartographic-everon` | map, ci | builds the cartographic ortho and its tile pyramid, patches the manifest, then `map-cartographic-verify` |
  | `map-cartographic-verify` | map, ci | `map verify-pyramid --terrain everon --view-map` |
  | `lfs-dem`, `lfs-sat` | map, ci | `git lfs pull` of the Everon elevation raster or satellite container |
  | `website-api-test` | build, ci | `cargo test` in `apps/website/api_v2`, honouring `TEST_DATABASE_URL` |
  | `developer-tools-test` | build, ci | `cargo test -p developer-tools --lib` |
  | `test` | build, ci | `rust-test` |
  | `build` | build, ci | `cargo build --release --bin api` in `apps/website/api_v2`, then `leptos-build` |
  | `rust-ci`, `rust-fmt`, `rust-clippy`, `rust-build`, `rust-test`, `wasm-ci`, `ci-local-leptos`, `leptos-build` | build, borrowed | the `cargo xtask mk` recipe of the same name, spelled as lines |
  | `rust-test-it` | db, borrowed | drops and creates `rust_it` in `tbd_reforger_db`, runs the API's tests against it on port 5434, then drops every `rust_it` database |

- Exit codes: 0 the task passed, or the listing printed; the first failing step's code otherwise
  (1 for an in-process step that returned an error; 127 a tool that could not start; 128 plus
  the signal number for a child killed by a signal); 2 an unknown task, or a documentation gate
  that did not run.
- Example: `cargo xtask ci ci-local-schema`

### help

- Synopsis: `cargo xtask help`
- Does: prints every task grouped as CI, schema, verify, map, build and db, with its help line
  and its `[alias]` or `[borrowed]` tag, then the `cargo xtask mk` targets and the
  `cargo xtask db` commands.
- Exit codes: 0.
- Example: `cargo xtask help`

## Boundaries

- Depends on: the verifications under `tools_v2/xtask/src/verifications/` and
  `codegen` in `tools_v2/xtask/src/commands/generate/schema_types.rs`, called in process; the
  `map` binary of `developer-tools`, cargo, trunk, podman, git-lfs, go, curl, unzip and apt-get as
  subprocesses; `TARGETS` of `tools_v2/xtask/src/commands/build/recipes.rs` and `LANE_COMMANDS` of
  `tools_v2/xtask/src/commands/db/operations.rs` for `help`.
- Used by:
  - `tools_v2/xtask/src/cli/dispatch.rs`, for `ci` and `help`;
  - `tools_v2/xtask/src/commands/schema/dispatch.rs`, whose `schema list-gates` prints the
    `schema-validate` step names, and `tools_v2/xtask/src/commands/platform/wave_execution/schema.rs`,
    which checks the wave gate's list against them;
  - `tools_v2/xtask/src/verifications/ci/schema_parity/source_audit.rs`, which reads the
    `ci-local`, `ci-local-schema` and `verify-mission-rest-size-limits` rows;
  - `.github/workflows/ci.yml` (`developer-tools-test`, `website-api-test`, `ci-local-schema`,
    `verify-editorconfig`; its `language-gates` job runs the `verify` commands of the
    `verify-documentation` row one step each), `.github/workflows/contracts.yml` (`verify-codegen-fresh`) and
    `.github/workflows/editor-gates.yml` (`ci-chrome`, `editor-api-boot`).
- Rules: the `ci-local` step set changes only together with `ci_local_step_set_is_frozen`;
  `cargo xtask verify ci-schema-parity` requires that `ci-local` call it directly, that
  `ci-local-schema` run `schema-validate` and `verify-citations`, and that the `ci.yml` schema job
  run `cargo xtask ci ci-local-schema`;
  every `Step::Task` names an existing row (`every_composite_step_resolves`); a subprocess line
  carries no shell syntax beyond a leading `cd <dir> && ` (`cmd_lines_are_shell_free`); the
  borrowed rows repeat the `mk` recipes with no test comparing the two, so a recipe change is made
  in both places.

## Related documentation

- [Testing and CI](/documentation_v2/runbooks/testing_and_ci.md) — running `ci-local` and single
  tasks, and where each gate runs: `ci-local`, the workflows and the wave gate.
- [Local development](/documentation_v2/runbooks/local_development.md) — running `ci-local` and
  its prerequisites.
- [Editor gates](/documentation_v2/runbooks/editor_gates.md) — the browser gates that run outside
  `ci-local`.
