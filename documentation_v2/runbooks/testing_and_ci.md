**Status:** live

# Testing and CI

Runs the repository's gates on a developer machine before a push to `main`, and shows where each
gate runs besides: the local replay `cargo xtask ci ci-local`, the GitHub workflows, the platform
[wave](/documentation_v2/glossary/n_to_z.md#wave) gate and the documentation gates. `ci-local` takes 15 to
40 minutes; a single gate takes seconds to a few minutes. What each `cargo xtask ci` task runs,
step by step, is in the [CI task commands README](/tools_v2/xtask/src/commands/ci/README.md); this
runbook does not repeat it.

## Prerequisites

- The Rust toolchain through rustup; the root `rust-toolchain.toml` pins 1.95.0 with rustfmt,
  clippy and the `wasm32-unknown-unknown` target. Check: `rustc --version` from the repository
  root prints `1.95.0`.
- Trunk, for the app build: `trunk --version`.
- The local database for the integration tests. `ci-local` reaches it through
  `podman exec tbd_reforger_db` (the `rust-test-it` steps name podman and the container directly)
  and on host port 5434. Start it as in
  [Local development](/documentation_v2/runbooks/local_development.md), step 2.
- The Everon elevation raster from Git LFS: the map engine's tests and the `height-labels` schema
  gate decode it. Check: `file assets_v2/terrains/everon/dem/everon-dem-16bit.png` prints
  `PNG image data`; `cargo xtask ci lfs-dem` fetches it.
- `editorconfig-checker` on `PATH`, or Go with its `bin` folder (`~/go/bin` by default) on `PATH`:
  `verify-editorconfig` then installs the pinned v3.4.0 itself with `go install`.
- For the browser gates only: the full Chrome build and the rest of the environment in
  [Editor gates](/documentation_v2/runbooks/editor_gates.md).

## Steps

Run every command from the repository root.

### Replay CI locally

1. Start the local database.

   ```bash
   cargo xtask db up
   ```

   Expected: `cd apps/website/api_v2 && podman compose up -d db` (with the runtime it found),
   then compose starts `tbd_reforger_db` on host port 5434. Without a compose provider it exits
   125 with "looking up compose provider failed"; an existing container then starts with
   `podman start tbd_reforger_db`.

2. Run the whole local gate.

   ```bash
   cargo xtask ci ci-local
   ```

   Expected: each step's command line, then its output, in the frozen order
   `verify-editorconfig`, `verify-no-python`, `verify-no-node`, `verify-no-shell`,
   `verify-ci-shell`, `verify-engine-layers`, `rust-ci`, `verify-coding-standards`,
   `ci-local-leptos`, `ci-local-schema`, `verify-staging-compose-paths`,
   `verify-mission-rest-size-limits` and `cargo xtask verify ci-schema-parity`; exit 0 when all
   pass. The run stops at the first failing step and exits with its code. The order is pinned by
   `ci_local_step_set_is_frozen` in `tools_v2/xtask/src/commands/ci/tests/task_runner.rs`.

### Run one gate

3. List every task.

   ```bash
   cargo xtask help
   ```

   Expected: the tasks grouped as CI, schema, verify, map, build and db, each with its help line;
   `[alias]` marks a wrapper on a `cargo xtask verify` command and `[borrowed]` a `cargo xtask mk`
   or database recipe repeated here; then one line each for the `cargo xtask mk` and
   `cargo xtask db` lanes.

4. Run one task, here the schema and citation gates that the `schema` job of `ci.yml` runs.

   ```bash
   cargo xtask ci ci-local-schema
   ```

   Expected: `verify-codegen-fresh`, `schema-validate` (six sub-gates) and `verify-citations`
   pass, exit 0. `cargo xtask ci <task>` runs any task `help` lists; an unknown name prints
   `xtask ci: no such task: <name>` and exits 2.

5. Run the app lane alone: formatting, clippy for `wasm32-unknown-unknown`, the native tests and a
   release Trunk build.

   ```bash
   cargo xtask mk ci-local-leptos
   ```

   Expected: the four command lines in turn, exit 0. Clippy runs without `-D warnings` here and in
   the `website-frontend` job, so a warning prints and does not fail; the [API](/documentation_v2/glossary/a_to_f.md#api) and engine lanes
   deny warnings.

6. Run the API's integration tests on a database of their own.

   ```bash
   cargo xtask db test-it
   ```

   Expected: the API's test binaries pass against a new randomly named database, dropped at the
   end; [Database operations](/documentation_v2/runbooks/database_operations.md#run-the-integration-tests)
   has the options.

### Run the browser gates

7. Build the app and run the editor smokes and the frozen DOM comparison. They are not part of
   `ci-local`.

   ```bash
   cargo xtask mk leptos-gates
   ```

   Expected: the release build, `gate doctor`, the editor suite (21 smokes) and `v-suite verify`
   pass. The environment, the single-smoke commands and the debug recipe are in
   [Editor gates](/documentation_v2/runbooks/editor_gates.md).

### Check documentation

8. Check that every folder has a README whose Contents lists exactly its children, here with the
   new files of a change included.

   ```bash
   cargo xtask verify readme-coverage --with-untracked --path documentation_v2/runbooks
   ```

   Expected: one verdict per judged README and exit 0; 1 when a README is missing or its Contents
   disagrees with the folder; 2 when a check could not run, such as a `--path` that names a file.
   `--path` repeats and takes folders only; without `--with-untracked` the gate judges committed
   files alone, as a later CI run would.

9. Check that Markdown sits where it belongs and that no live document passes 500 lines.

   ```bash
   cargo xtask verify markdown-placement --with-untracked --path documentation_v2/runbooks
   ```

   Expected: exit 0; 1 names each misplaced or oversized file.

10. Check every link, backticked repository path and cited `cargo xtask` command.

    ```bash
    cargo xtask verify link-check --with-untracked --path documentation_v2/runbooks
    ```

    Expected: exit 0; 1 lists each failing document with its break count and the first 20 breaks
    as `path:line: rule: message`, and `--report` prints all of them. No `ci-local` step and no
    workflow runs the three documentation gates, so run them before committing documentation.

### Gate a wave

11. On merged `main`, before a wave closes, run the wave gate over the wave's range.

    ```bash
    cargo xtask platform wave gate
    ```

    Expected: one `PASS` or `FAIL` line per step (every step runs; a failure shows its last 15
    lines), then `GATE: PASS` and exit 0. The base defaults to the last `wave N CLOSED` commit;
    `--slice <id>` runs the cheap slice gate in a slice worktree instead. The step lists of both
    are in the [wave gate README](/tools_v2/xtask/src/commands/platform/wave_execution/gate/README.md),
    and the whole wave procedure in [Factory waves](/documentation_v2/runbooks/factory_waves/README.md).

## Gate matrix

Where each gate runs. "ci-local" means a step of `cargo xtask ci ci-local`; the job names are those
of `.github/workflows/ci.yml` unless the row names another workflow; "both" in the last column means
the wave gate and the slice gate. Rule ids (FMT-2, LANG-1, TEST-1 and the rest) are those of the
[coding standards](/documentation_v2/standards/coding_standards/README.md).

| Gate | Command | ci-local | GitHub | Wave gate |
|---|---|---|---|---|
| whitespace (FMT-2) | `cargo xtask ci verify-editorconfig` | yes | `editorconfig` | no |
| no Python (LANG-2, LANG-3) | `cargo xtask verify no-python` | yes | `language-gates` | both |
| no Node scripts | `cargo xtask verify no-node` | yes | `language-gates` | wave |
| no shell or Make (LANG-1) | `cargo xtask verify no-shell` | yes | `language-gates` | wave |
| workflow `run:` lines | `cargo xtask verify ci-shell` | yes | `language-gates` | wave |
| engine layers | `cargo xtask verify engine-layers` | yes | `language-gates` | no |
| file length (SIZE-3) | `cargo xtask verify file-length` | in `verify-coding-standards` | `language-gates` | no |
| no Markdown under `docs` folders | `cargo xtask ci verify-doc-layout` | in `verify-coding-standards` | no | no |
| no `SELECT *` | `cargo xtask verify no-select-star` | in `verify-coding-standards` | no | no |
| `@route` tags (GO-7) | `cargo xtask verify route-tags` | in `verify-coding-standards` | no | both |
| Rust formatting | `cargo xtask mk rust-fmt` | in `rust-ci` | `website-api` | changed files |
| API clippy, `-D warnings` | `cargo xtask mk rust-clippy` | in `rust-ci` | `website-api` | changed crates (slice); API (wave) |
| API build | `cargo xtask mk rust-build` | in `rust-ci` | `website-api` | `cargo check` |
| map and graphics engines: fmt, clippy `-D warnings` (host and wasm32), tests | `cargo xtask mk wasm-ci` | in `rust-ci` | `map-engine` | clippy and tests (wave) |
| API tests with Postgres (TEST-1) | `cargo xtask ci rust-test-it`; `cargo xtask db test-it` | in `rust-ci` | `website-api` (`cargo xtask ci website-api-test`) | wave |
| developer-tools library tests | `cargo xtask ci developer-tools-test` | no | `website-api` | wave, with the xtask tests |
| app: fmt, clippy (wasm32), tests, Trunk build (TEST-2) | `cargo xtask mk ci-local-leptos` | yes | `website-frontend` | wasm32 check, clippy and tests; Trunk when the app changed |
| generated contract types current | `cargo xtask ci verify-codegen-fresh` | in `ci-local-schema` | `schema`; `contracts.yml` | no |
| schema validation (TEST-3, ENF-4) | `cargo xtask ci schema-validate` | in `ci-local-schema` | `schema`; `schema.yml` runs `cargo xtask schema validate` only | both |
| `@contract` citations (TS-6, ENF-3) | `cargo xtask ci verify-citations` | in `ci-local-schema` | `schema`; `contracts.yml` | both, with the schema step |
| staging compose path | `cargo xtask verify staging-compose-paths` | yes | `mod-gates-hosted` | both |
| mission REST size limits | `cargo xtask verify mission-rest-size-limits` | yes | `mod-gates-hosted` | both |
| CI schema parity | `cargo xtask verify ci-schema-parity` | yes, called directly | `mod-gates-hosted` | both |
| seeds, registry aliases and comment contracts | `cargo xtask verify wiki-seeds`, `faction-library-seeds`, `object-registry-aliases`, `destroy-target-diagnostics`, `results-reporter-identity-comments`, `player-identity-comments` | no | no | both |
| ticket registry | `cargo xtask ticket check --strict` | no | `language-gates` | wave, with the wave lock check |
| mod boot verdict self-test | `cargo xtask mod world-boot --selftest` | no | `mod-gates-hosted` | no |
| mod compile and world boot | `cargo xtask mod compile`, `cargo xtask mod world-boot` | no | `mod-gates.yml`, nightly on a self-hosted runner with the dedicated server | no |
| editor smokes and DOM comparison | `cargo xtask mk leptos-gates` | no | `editor-gates.yml`, nightly, on demand and on pull requests touching the app, the map engine or developer-tools | no |
| documentation gates | `cargo xtask verify readme-coverage`, `markdown-placement`, `link-check` | no | no | no |

`ci.yml` runs on every push and pull request to `main` with no path filter; `contracts.yml` and
`schema.yml` run only when their paths change. The `schema` job must run
`cargo xtask ci ci-local-schema`, which `cargo xtask verify ci-schema-parity` enforces. The
engine-layers rules, one row per rule, are in
[Engine boundary rules](/documentation_v2/standards/engine_boundary_rules.md). The `[borrowed]`
tasks repeat the `cargo xtask mk` recipes of the same name, and no test compares the two copies, so a
recipe change goes into both. Two coding-standards rules have no automated gate: the Enfusion log
policy and the authority comments (ENF-1, ENF-2), checked in Workbench.

## Verify

```bash
cargo xtask ci ci-local
```

Expected: every step passes and the command exits 0. A change to the app or the map engine also
passes step 7; a change to documentation passes steps 8 to 10.

## Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| `db up` exits 125 with "looking up compose provider failed" | podman has no compose provider | install `podman-compose` or the `docker-compose` plugin, or `podman start tbd_reforger_db` for an existing container |
| `rust-test-it` fails at `podman exec tbd_reforger_db` | the container is not running, or the machine has docker but no podman; the steps name podman | step 1; on a docker-only machine run `cargo xtask db test-it` for the integration tests |
| `verify-editorconfig` fails before checking anything | `editorconfig-checker` is absent, and Go is missing or its `bin` folder is not on `PATH` | install the checker, or Go with its `bin` folder on `PATH` |
| the map-engine tests or `height-labels` fail to decode the elevation raster | the raster is an LFS pointer file | `cargo xtask ci lfs-dem` |
| `cargo fmt` or clippy fails on files the change never touched | another session's uncommitted work in the same tree | judge the failure by path; gate a clean checkout of `main` |
| `xtask ci: no such task: <name>` | the name is a `cargo xtask mk` or `cargo xtask verify` command, not a `ci` task | `cargo xtask help` lists the `ci` tasks and names the other lanes |
| a documentation gate exits 2 with "--path takes a folder" | `--path` named a file | pass the file's folder |
| `schema: height-labels SKIP in this tree` in the wave gate | the worktree holds the raster as an LFS pointer | expected in a worktree; on `main` with the real raster the sub-gate runs |
| the wave gate prints `FAIL (TIMEOUT after <n>s)` | a step outran `TBD_GATE_TIMEOUT` (1200 s by default) | raise `TBD_GATE_TIMEOUT` or look for a hung step |

## Related

- [CI task commands](/tools_v2/xtask/src/commands/ci/README.md) — every `cargo xtask ci` task and
  its steps.
- [Repository verifications](/tools_v2/xtask/src/verifications/README.md) — what each
  `cargo xtask verify` gate checks.
- [Editor gates](/documentation_v2/runbooks/editor_gates.md) — the browser gates in detail.
- [Local development](/documentation_v2/runbooks/local_development.md) — the local stack the gates
  run against.
- [Database operations](/documentation_v2/runbooks/database_operations.md) — the integration test
  database and its options.
- [Factory waves](/documentation_v2/runbooks/factory_waves/README.md) — the wave procedure the wave
  gate belongs to.
- [Coding standards](/documentation_v2/standards/coding_standards/README.md) and
  [Engine boundary rules](/documentation_v2/standards/engine_boundary_rules.md) — the rules the
  gates enforce.
