# CI task catalog source

The `cargo xtask ci` lane and `cargo xtask help`, with the `cargo xtask mk` lane in `build_lane/`
and the map asset checks the tasks run: one table of named tasks (the local CI replay `ci-local`,
the schema gates, the workspace laws and language bans, the CI workflow helpers and the map asset
composites) and the runner that executes a task's steps. Developers run `ci-local` before pushing,
and the GitHub workflows run single tasks by name.

## Contents

```text
tools/commands/ci_task_catalog/src/
├── api_package_lane.rs  the API's test, lint and build lines over `api_server` and every `crates/api` package, derived from the workspace
├── build_lane/          the `cargo xtask mk` lane: the recipes and the step runner
├── cargo_target_pin.rs  the shared `CARGO_TARGET_DIR` pin, the two checkout roots, the development API's folder, the glibc stamp guard
├── chromium_install.rs  the `ci-chrome` task: installs the pinned Chrome for Testing build
├── ci_scratch_reclaim.rs  the `mk reclaim-target-ci` body: deletes the continuous-integration scratch folder
├── editor_api.rs        the `editor-api-boot`, `verify-codegen-fresh` and `verify-editorconfig` tasks
├── error.rs             `Error`, `Result` and `cause_chain`, the text an in-process step's error prints
├── frontend_package_lane.rs  the frontend family (`frontend_application` and every crates/frontend package) and its format, lint and test lines, derived from the workspace
├── lib.rs               the crate root: module header, `mod` lines and the re-exports
├── prelude.rs           `Error`, `Result`, `cause_chain`, `Task`, `Step`, `Lane` and `TASKS` for glob import
├── task_definitions/    the step macros, the map-lane step lists, the workspace-law steps and the in-process verification adapters
├── task_definitions.rs  `TASKS`: every task with its help line, group, lane and steps
├── task_runner/         the runner, the child environment, `help`, the gate list
├── task_runner.rs       the `Task`, `Step` and `Lane` types, re-exports
├── wasm32_lint_lane.rs  the packages the wasm32 lint covers, derived from the workspace, and the `wasm-ci` row's lint step
├── wave_gate_steps/     the `mk` helper commands the ticket manager's slice and wave gates run (`changed-packages`, `gate-step <name>`, `preflight-api-freshness`, `target-abi-guard`)
└── workspace_member_tests.rs  the `workspace-member-tests` task: one `cargo test --workspace` excluding the API and frontend families
```

## How it works

`TASKS` in `task_definitions.rs` is pure data and `run_task_in` in `task_runner/split_cmd.rs` is its
only interpreter. A step is another task by name (`Step::Task`, so a composite runs the very row
its standalone command runs), a subprocess line, an in-process xtask call or a native Rust step; no
step runs a shell script. The runner prints each step's line before it runs (a native step prints
its own), stops at the first non-zero code and returns that code.

`cargo_target_pin.rs` computes where every build writes: the shared cache `CARGO_TARGET_DIR`
(the variable when set, else the primary checkout's `target/host/` or `target/container/`, by
toolchain environment), the development API's private `dev-api/` under the current checkout's
environment folder, and the glibc stamp guard; the subfolder names are
`repository_layout::build_output`'s. `ci_scratch_reclaim.rs` holds the body of
`cargo xtask mk reclaim-target-ci`.

Each row carries a lane, which `help` prints as a tag:

- `Ci`: this table owns the name and the steps.
- `Alias`: a wrapper on existing `cargo xtask verify …` commands.
- `Borrowed`: a build or database recipe repeated here so the composites run it: the
  `cargo xtask mk` recipes `rust-ci`, `rust-fmt`, `rust-clippy`, `rust-test`, `wasm-ci`,
  `ci-local-leptos` and `leptos-build`, and the `rust-test-it` integration run.

`ci-local` runs, in this order: `rust-ci` (fmt, clippy, `wasm-ci`, `rust-test-it`),
`workspace-member-tests`, `ci-local-leptos`, `ci-local-schema` (`verify-codegen-fresh`,
`schema-validate`), `verify-workspace-laws` (crate tiers, crate anatomy, Tailwind sources),
`verify-language-bans` (no-python, no-node, no-shell) and `verify-file-length`, which warns and
never fails. The browser gates of `cargo xtask mk leptos-gates` are not part of it.

Every workspace member is tested. The frontend crates go with `frontend_application`:
`frontend_package_lane.rs` derives the frontend family, the app followed by every member under
crates/frontend in path order (the offline service worker among them), and the four cargo lines of
`ci-local-leptos` (format, wasm32 clippy, native clippy, native tests) name each of them with `-p`,
the row through native steps and the `mk` recipe through the same derivation. The API crates go
with `api_server`: `api_package_lane.rs` derives `api_server`, every other member under
`crates/api` and every member that uses an API crate (through `database_operations`'
`api_test_packages`, the list `cargo xtask db test-it` runs over), and `api-test`, `rust-test` and
`rust-clippy` run one cargo line naming each of them with `-p`. `workspace-member-tests` runs one
`cargo test --workspace` with an `--exclude` for every package of both families, so every other
member is tested from the moment the root manifest names it. An excluded package that is no
member, or a workspace that cannot be read, fails the task.

Every crate that ships to the browser is linted for `wasm32-unknown-unknown`.
`wasm32_lint_lane.rs` derives the set from the workspace: each member whose
`[package.metadata.layout]` declares `targets = "wasm32"`, and every crate of the frontend family.
`ci-local-leptos` lints the frontend family with every target; the `wasm-ci` recipe and row lint
the rest in one `cargo clippy --target wasm32-unknown-unknown`.

## Commands

### ci

- Synopsis: `cargo xtask ci [<task>]`
- Does: runs one task of the table below; with no task, prints the same listing as `help`.

  | Task | Group, lane | Runs |
  |---|---|---|
  | `ci-local` | CI, ci | the composite above; needs `cargo xtask db up` first |
  | `ci-local-schema` | CI, ci | `verify-codegen-fresh`, `schema-validate` |
  | `ci-chrome` | CI, ci | the Chrome for Testing version from `tools/browser_testing/browser_gate_suites/gate-env.json` (or `CHROME_VERSION`), its system libraries through `sudo apt-get`, unpacked into `~/cft`; prints `CHROME_HEADLESS_SHELL=` and appends it to `GITHUB_ENV` when set |
  | `editor-api-boot` | CI, ci | builds and starts the `api-server` binary of `api_server` in its own session, logging to `/tmp/api.log`, and waits up to 60 s for `GET /healthz` on `PORT` (8080); the API keeps running |
  | `schema-validate` | schema, ci | `schema validate`, `map-object-enums`, `type-inventory`, in process |
  | `schema-map-goldens` | schema, ci | on demand: `schema map-object-golden`, `map-glyphs`, `height-labels`, in process (needs the LFS Everon DEM) |
  | `schema-codegen` | schema, ci | `schema codegen`: regenerates the contract types from `contracts/definitions/` |
  | `verify-codegen-fresh` | schema, ci | regenerates the contract outputs in memory and compares them with the files |
  | `verify-editorconfig` | verify, ci | `editorconfig-checker` from the root, installing the pinned v3.4.0 with `go install` when absent |
  | `verify-language-bans` | verify, alias | `verify no-python`, `verify no-node` and `verify no-shell`, in process |
  | `verify-file-length` | verify, alias | `verify file-length`: warns about long production files, never fails |
  | `verify-workspace-laws` | verify, alias | `verify crate-tiers`, `verify crate-anatomy` and `verify tailwind-sources`, in process |
  | `verify-terrain` | verify, ci | `schema terrain-manifest` and `schema terrain-alignment` for Everon |
  | `verify-terrain-strict` | verify, ci | the same with `terrain-alignment --strict` |
  | `map-water-everon` | map, ci | restores the pre-water Everon ortho from `assets/scratch/`, resets the water metadata, analyses and composites the water, rebuilds the satellite container and tile pyramid, then verifies all three |
  | `map-cartographic-everon` | map, ci | builds the cartographic ortho and its tile pyramid, patches the manifest, then `map-cartographic-verify` |
  | `map-cartographic-verify` | map, ci | `map verify-pyramid --terrain everon --view-map` |
  | `lfs-dem`, `lfs-sat` | map, ci | `git lfs pull` of the Everon elevation raster or satellite container |
  | `api-test` | build, ci | `cargo test -p api_server -p <every crates/api package>`, honouring `TEST_DATABASE_URL` |
  | `workspace-member-tests` | build, ci | one `cargo test --workspace` with an `--exclude` for every package of the API and frontend families, derived from the root `Cargo.toml` |
  | `test` | build, ci | `rust-test` |
  | `build` | build, ci | `cargo build --release --bin api-server` in `crates/api/api_server`, then `leptos-build` |
  | `rust-ci`, `rust-fmt`, `rust-clippy`, `rust-test`, `wasm-ci`, `ci-local-leptos`, `leptos-build` | build, borrowed | the `cargo xtask mk` recipe of the same name, spelled as lines (`rust-clippy` and `rust-test` as the derived API line over `api_server` and every `crates/api` package; the `wasm-ci` lint as the derived wasm32 line; the cargo lines of `ci-local-leptos` as the derived frontend lines) |
  | `rust-test-it` | db, borrowed | `cargo xtask db test-it` in process: the API's tests against a fresh database on port 5434, then that run's databases dropped |

- Exit codes: 0 the task passed, or the listing printed; the first failing step's code otherwise
  (1 for an in-process step that returned an error; 127 a tool that could not start; 128 plus
  the signal number for a child killed by a signal); 2 an unknown task.
- Example: `cargo xtask ci ci-local-schema`

### help

- Synopsis: `cargo xtask help`
- Does: prints every task grouped as CI, schema, verify, map, build and db, with its help line
  and its `[alias]` or `[borrowed]` tag, then the `cargo xtask mk` targets and the
  `cargo xtask db` commands.
- Exit codes: 0.
- Example: `cargo xtask help`

## Boundaries

- Depends on: `repository_checks`, the `database_operations` and `schema_tooling` crates and
  `map_asset_verification`, called in process; the `map` binary of `developer_tools`, cargo,
  trunk, podman, git-lfs, go, curl, unzip and apt-get as subprocesses; `TARGETS` of
  `tools/commands/ci_task_catalog/src/build_lane/recipes.rs` and `LANE_COMMANDS` of
  `tools/commands/database_operations/src/local_database.rs` for `help`;
  `repository_laws::workspace_members` for the members `workspace-member-tests` checks its
  exclusions against.
- Used by:
  - `tools/xtask/src/cli/dispatch.rs`, for `ci`, `help` and `mk`;
  - `tools/xtask/src/commands/schema/dispatch.rs`, whose `schema list-gates` prints the
    `schema-validate` step names, and `wave_gate_steps/schema_step.rs`, which checks the wave
    gate's list against them;
  - `wave_gate_steps/clippy_lanes.rs`, whose `test-workspace-members` gate step runs
    `workspace_test_argv`;
  - the central ticket manager's slice and wave gates, which run the `wave_gate_steps` helpers
    through `cargo xtask mk` ([`ticket_manager_execution.toml`](/ticket_manager_execution.toml));
  - `.github/workflows/ci.yml` (`api-test`, `workspace-member-tests`, `ci-local-schema`,
    `verify-editorconfig`, `verify-language-bans`, and `rust-fmt`, `rust-clippy`, `wasm-ci` and
    `ci-local-leptos` through `mk`) and `.github/workflows/editor-gates.yml` (`ci-chrome`,
    `editor-api-boot`).
- Rules: every `Step::Task` names an existing row, and the runner refuses one that does not; a
  subprocess line carries no shell syntax beyond a leading `cd <dir> && `; no step names a
  container runtime itself; the borrowed rows repeat the `mk` recipes, so a recipe change is made
  in both places.

## Related documentation

- [Testing and CI](/documentation/runbooks/testing_and_ci.md) — running `ci-local` and single
  tasks, and where each gate runs: `ci-local`, the workflows and the wave gate.
- [Local development](/documentation/runbooks/local_development.md) — running `ci-local` and
  its prerequisites.
- [Editor gates](/documentation/runbooks/editor_gates.md) — the browser gates that run outside
  `ci-local`.
