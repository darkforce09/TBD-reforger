# CI task catalog source

The `cargo xtask ci` lane and `cargo xtask help`, with the `cargo xtask mk` lane in `build_lane/`
and the CI workflow and map asset checks the tasks run: one table of named tasks (the local CI replay
`ci-local`, the schema gates, the language, coding-standard and documentation verifications, the
CI workflow helpers and the map asset composites) and the runner that executes a task's steps.
Developers run `ci-local` before pushing, and the GitHub workflows run single tasks by name.

## Contents

```text
tools/commands/ci_task_catalog/src/
├── api_package_lane.rs  the API's test, lint and build lines over `api` and every `crates/api` package, derived from the workspace
├── build_lane/          the `cargo xtask mk` lane: the recipes, the step runner and their tests
├── cargo_target_pin.rs  the shared `CARGO_TARGET_DIR` pin, the two checkout roots, the development API's folder, the glibc stamp guard
├── cargo_target_verification.rs  the `mk verify-cargo-target` and `mk reclaim-target-ci` bodies over the pin and the recipes
├── chromium_install.rs  the `ci-chrome` task: installs the pinned Chrome for Testing build
├── editor_api.rs        the `editor-api-boot`, `verify-codegen-fresh` and `verify-editorconfig` tasks
├── error.rs             `Error`, `Result` and `cause_chain`, the text an in-process step's error prints
├── frontend_package_lane.rs  the frontend family (`frontend` and every crates/frontend package) and its format, lint and test lines, derived from the workspace
├── lib.rs               the crate root: module header, `mod` lines and the re-exports
├── prelude.rs           `Error`, `Result`, `cause_chain`, `Task`, `Step`, `Lane` and `TASKS` for glob import
├── task_definitions/    the step macros, the map-lane step lists and the in-process verification adapters
├── task_definitions.rs  `TASKS`: every task with its help line, group, lane and steps
├── task_runner/         the runner, the child environment, `help`, the gate list
├── task_runner.rs       the `Task`, `Step` and `Lane` types, re-exports
├── tests/               unit tests for the frozen `ci-local` set, composite failure, `help`, parity, member coverage, the API lines, the frontend lines, the wasm32 lint and the target pin
├── wasm32_lint_lane.rs  the packages the wasm32 lint covers, derived from the workspace, and the `wasm-ci` row's lint step
├── workflow_checks/     the `verify ci-shell` and `verify ci-schema-parity` gates
└── workspace_member_tests.rs  the `workspace-member-tests` task: `cargo test -p` for every member no dedicated task tests
```

## How it works

`TASKS` in `task_definitions.rs` is pure data and `run_task_in` in `task_runner/split_cmd.rs` is its
only interpreter. A step is another task by name (`Step::Task`, so a composite runs the very row
its standalone command runs), a subprocess line, an in-process xtask call or a native Rust step; no
step runs a shell script. The runner prints each step's line before it
runs (a native step prints its own), stops at the first non-zero code and returns that code.
The binary hands `run` its clap command tree, which the in-process `link-check` step judges
`cargo xtask` citations against, together with the recipe and task tables; the lane never reads
the command line itself.

`cargo_target_pin.rs` computes where every build writes: the shared cache `CARGO_TARGET_DIR`
(the variable when set, else the primary checkout's `target/`), the development API's private
`target/dev-api` under the current checkout, and the glibc stamp guard; the subfolder names are
`repository_layout::build_output`'s. `cargo_target_verification.rs` holds the bodies of
`cargo xtask mk verify-cargo-target` (the pin's source text, its value with the variable unset, the
worktree-local reversal, and the private-folder rule over the `rust-build` and `rust-api` recipes)
and `cargo xtask mk reclaim-target-ci`; the `build_lane` recipes dispatch to both.

Each row carries a lane, which `help` prints as a tag:

- `Ci`: this table owns the name and the steps.
- `Alias`: a one-line wrapper on an existing `cargo xtask verify …` command.
- `Borrowed`: a build or database recipe repeated here so the composites run it: the
  `cargo xtask mk` recipes `rust-ci`, `rust-fmt`, `rust-clippy`, `rust-build`, `rust-test`,
  `wasm-ci`, `ci-local-leptos` and `leptos-build`, and the `rust-test-it` integration run.

`ci-local` runs, in this order: `verify-editorconfig`, `verify-no-python`, `verify-no-node`,
`verify-no-shell`, `verify-ci-shell`, `verify-workspace-laws`, `rust-ci`,
`workspace-member-tests`, `verify-coding-standards`,
`verify-documentation`, `ci-local-leptos`, `ci-local-schema`, `verify-staging-compose-paths`,
`verify-mission-rest-size-limits`, and `cargo xtask verify ci-schema-parity` in process.
`ci_local_step_set_is_frozen` in `tests/task_runner.rs` fails when a step is added, dropped or
moved. The browser gates of `cargo xtask mk leptos-gates` are not part of it.

Every workspace member is tested. `DEDICATED_TEST_TASKS` in `workspace_member_tests.rs` names the
members a dedicated task tests (`api` by `api-test`, `frontend` by `ci-local-leptos`, and
`offline_service_worker` by `wasm-ci`). The frontend crates go with `frontend`:
`frontend_package_lane.rs` derives the frontend family, the app followed by every member under
crates/frontend in path order, and the four cargo lines of `ci-local-leptos` (format, wasm32
clippy, native clippy, native tests) name each of them with `-p`, the row through native steps and
the `mk` recipe through the same derivation; `ci_local_leptos_recipe_and_ci_task_row_run_the_same_lines`
pins the two spellings together. The API crates go with
`api`: `api_package_lane.rs` derives `api`, every member under `crates/api` and every member that uses an
API crate, such as `staging_fixtures` (through
`database_operations`' `api_test_packages`, the list `cargo xtask db test-it` runs over), and
`api-test`, `rust-test`, `rust-clippy` and `rust-build` run one cargo line naming each of them with
`-p`, as the `mk` recipes of the same names do. `workspace-member-tests`
reads the root `Cargo.toml` workspace and runs `cargo test -p <package>` once for every other
member, so a member the workspace gains is tested from the moment the manifest names it (the
binary-only `xtask` and `developer_tools` packages among them, whose run builds every binary). A dedicated entry that names no member, or a workspace that
cannot be read, fails the task. `every_dedicated_test_task_tests_its_package`,
`ci_local_tests_every_workspace_member` and `the_ci_workflow_tests_every_workspace_member` fail
when a dedicated task stops testing its package, or when `ci-local` or `.github/workflows/ci.yml`
leaves a member untested.

Every crate that ships to the browser is linted for `wasm32-unknown-unknown`.
`wasm32_lint_lane.rs` derives the set from the workspace: each member whose
`[package.metadata.layout]` declares `targets = "wasm32"`, the `frontend` and
`offline_service_worker` applications, and every crate of the frontend family (a `targets = "any"`
one included). `ci-local-leptos` lints the frontend family with every target; the `wasm-ci` recipe and row
lint the rest in one `cargo clippy --target wasm32-unknown-unknown`, the row through a native step
that derives the same line. `wasm_ci_and_the_frontend_lane_partition_the_lint` fails when the two lanes
stop covering the set, and `wasm_ci_lints_every_derived_wasm32_package` when the recipe line drifts
from it.

## Commands

### ci

- Synopsis: `cargo xtask ci [<task>]`
- Does: runs one task of the table below; with no task, prints the same listing as `help`.

  | Task | Group, lane | Runs |
  |---|---|---|
  | `ci-local` | CI, ci | the composite above; needs `cargo xtask db up` first |
  | `ci-local-schema` | CI, ci | `verify-codegen-fresh`, `schema-validate`, `verify-citations` |
  | `ci-chrome` | CI, ci | the Chrome for Testing version from `tools/browser_testing/browser_gate_suites/gate-env.json` (or `CHROME_VERSION`), its system libraries through `sudo apt-get`, unpacked into `~/cft`; prints `CHROME_HEADLESS_SHELL=` and appends it to `GITHUB_ENV` when set |
  | `editor-api-boot` | CI, ci | builds and starts the `api` binary of `api` in its own session, logging to `/tmp/api.log`, and waits up to 60 s for `GET /healthz` on `PORT` (8080); the API keeps running |
  | `schema-validate` | schema, ci | `schema validate`, `map-object-golden`, `map-glyphs`, `height-labels`, `map-object-enums`, `type-inventory`, in process |
  | `schema-codegen` | schema, ci | `schema codegen`: regenerates the contract types from `contracts/definitions/` |
  | `verify-citations` | schema, ci | `schema citations`: the `@contract` citations in code |
  | `verify-codegen-fresh` | schema, ci | regenerates the contract outputs in memory and compares them with the files |
  | `verify-coding-standards` | verify, ci | `verify file-length`, `verify enfusion-comments` (the pinned mod Scripts roots), `verify no-select-star` and `verify route-tags`, in process |
  | `verify-documentation` | verify, ci | `verify readme-coverage`, `verify link-check` and `verify markdown-placement` over the committed tree, in process |
  | `verify-editorconfig` | verify, ci | `editorconfig-checker` from the root, installing the pinned v3.4.0 with `go install` when absent |
  | `verify-no-python`, `verify-no-node`, `verify-no-shell`, `verify-ci-shell`, `verify-staging-compose-paths`, `verify-mission-rest-size-limits` | verify, alias | the `cargo xtask verify` command of the same name |
  | `verify-terrain` | verify, ci | `schema terrain-manifest` and `schema terrain-alignment` for Everon |
  | `verify-terrain-strict` | verify, ci | the same with `terrain-alignment --strict` |
  | `map-water-everon` | map, ci | restores the pre-water Everon ortho from `assets/scratch/`, resets the water metadata, analyses and composites the water, rebuilds the satellite container and tile pyramid, then verifies all three |
  | `map-cartographic-everon` | map, ci | builds the cartographic ortho and its tile pyramid, patches the manifest, then `map-cartographic-verify` |
  | `map-cartographic-verify` | map, ci | `map verify-pyramid --terrain everon --view-map` |
  | `lfs-dem`, `lfs-sat` | map, ci | `git lfs pull` of the Everon elevation raster or satellite container |
  | `api-test` | build, ci | `cargo test -p api -p <every crates/api package>`, honouring `TEST_DATABASE_URL` |
  | `workspace-member-tests` | build, ci | `cargo test -p <package>`, one run per workspace member outside `DEDICATED_TEST_TASKS`, the API package family and the frontend family, derived from the root `Cargo.toml`; every package runs, and the exit is the first red package's code |
  | `test` | build, ci | `rust-test` |
  | `build` | build, ci | `cargo build --release --bin api` in `apps/api`, then `leptos-build` |
  | `rust-ci`, `rust-fmt`, `rust-clippy`, `rust-build`, `rust-test`, `wasm-ci`, `ci-local-leptos`, `leptos-build` | build, borrowed | the `cargo xtask mk` recipe of the same name, spelled as lines (`rust-clippy`, `rust-build` and `rust-test` as the derived API line over `api` and every `crates/api` package; the cargo lines of `ci-local-leptos` as the derived frontend lines over `frontend` and every crates/frontend package) |
  | `rust-test-it` | db, borrowed | `cargo xtask db test-it` in process: the API's tests against a fresh database on port 5434, then that run's databases dropped |

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

- Depends on: the check crates `repository_checks`, `mod_script_checks` and
  `documentation_checks`, the `database_operations`, `deployment` and `schema_tooling` crates,
  `map_asset_verification` and this crate's `workflow_checks/`, called in process; the
  `map` binary of `developer_tools`, cargo, trunk, podman, git-lfs, go, curl, unzip and apt-get as
  subprocesses; `TARGETS` of `tools/commands/ci_task_catalog/src/build_lane/recipes.rs` and `LANE_COMMANDS` of
  `tools/commands/database_operations/src/local_database.rs` for `help`;
  `repository_laws::workspace_members` for the members `workspace-member-tests`
  derives.
- Used by:
  - `tools/xtask/src/cli/dispatch.rs`, for `ci`, `help` and `mk`;
  - `tools/xtask/src/commands/schema/dispatch.rs`, whose `schema list-gates` prints the
    `schema-validate` step names, and `tools/commands/platform_execution/src/wave_execution/schema.rs`,
    which checks the wave gate's list against them;
  - `tools/commands/ci_task_catalog/src/workflow_checks/schema_parity/source_audit.rs`, which reads the
    `ci-local`, `ci-local-schema` and `verify-mission-rest-size-limits` rows;
  - `tools/commands/platform_execution/src/wave_execution/gate/gate_dispatch.rs`, whose
    `test workspace members` step derives its packages through `member_packages_except`;
  - `.github/workflows/ci.yml` (`api-test`, `workspace-member-tests`, `wasm-ci` and `ci-local-leptos` through `mk`,
    `ci-local-schema`, `verify-editorconfig`; its `language-gates` job runs the `verify` commands of the
    `verify-documentation` row one step each), `.github/workflows/contracts.yml` (`verify-codegen-fresh`) and
    `.github/workflows/editor-gates.yml` (`ci-chrome`, `editor-api-boot`).
- Rules: the `ci-local` step set changes only together with `ci_local_step_set_is_frozen`;
  `cargo xtask verify ci-schema-parity` requires that `ci-local` call it directly, that
  `ci-local-schema` run `schema-validate` and `verify-citations`, and that the `ci.yml` schema job
  run `cargo xtask ci ci-local-schema`;
  every `Step::Task` names an existing row (`every_composite_step_resolves`); a subprocess line
  carries no shell syntax beyond a leading `cd <dir> && ` (`cmd_lines_are_shell_free`); no step
  names a container runtime itself (`no_task_step_names_a_bare_container_runtime`); the
  borrowed rows repeat the `mk` recipes with no test comparing the two, so a recipe change is made
  in both places.

## Related documentation

- [Testing and CI](/documentation/runbooks/testing_and_ci.md) — running `ci-local` and single
  tasks, and where each gate runs: `ci-local`, the workflows and the wave gate.
- [Local development](/documentation/runbooks/local_development.md) — running `ci-local` and
  its prerequisites.
- [Editor gates](/documentation/runbooks/editor_gates.md) — the browser gates that run outside
  `ci-local`.
