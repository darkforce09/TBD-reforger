**Status:** live

# Testing and CI

Runs the repository's gates on a developer machine before a push to `main`, and shows where each
gate runs besides: the local replay `cargo xtask ci ci-local`, the GitHub workflows, the platform
[wave](/documentation/glossary/n_to_z.md#wave) gate and the documentation link check. `ci-local` takes 15 to
40 minutes; a single gate takes seconds to a few minutes. What each `cargo xtask ci` task runs,
step by step, is in the [CI task commands README](/tools/commands/ci_task_catalog/src/README.md); this
runbook does not repeat it.

## Prerequisites

- The Rust toolchain through rustup; the root `rust-toolchain.toml` pins 1.95.0 with rustfmt,
  clippy and the `wasm32-unknown-unknown` target. Check: `rustc --version` from the repository
  root prints `1.95.0`.
- Trunk, for the app build: `trunk --version`.
- The local database for the integration tests. `ci-local` reaches it through
  `cargo xtask db test-it`, which resolves the container runtime itself (the `rust-test-it` step
  runs it in process), on host port 5434. Start it as in
  [Local development](/documentation/runbooks/local_development.md), step 2.
- The Everon elevation raster from Git LFS: the terrain and world-object crates' tests and the `height-labels` schema
  gate decode it. Check: `file assets/terrains/everon/dem/everon-dem-16bit.png` prints
  `PNG image data`; `cargo xtask ci lfs-dem` fetches it.
- `editorconfig-checker` on `PATH`, or Go with its `bin` folder (`~/go/bin` by default) on `PATH`:
  `verify-editorconfig` then installs the pinned v3.4.0 itself with `go install`.
- For the browser gates only: the full Chrome build and the rest of the environment in
  [Editor gates](/documentation/runbooks/editor_gates.md).

## Steps

Run every command from the repository root.

### Replay CI locally

1. Start the local database.

   ```bash
   cargo xtask db up
   ```

   Expected: `cd deploy && podman compose -f compose.dev.yml up -d db` (with the runtime it found),
   then compose starts `tbd_reforger_db` on host port 5434. Without a compose provider it exits
   125 with "looking up compose provider failed"; an existing container then starts with
   `podman start tbd_reforger_db`.

2. Run the whole local gate.

   ```bash
   cargo xtask ci ci-local
   ```

   Expected: each step's command line, then its output, in the order `rust-ci`,
   `workspace-member-tests`, `ci-local-leptos`, `ci-local-schema`, `verify-workspace-laws`,
   `verify-language-bans` and `verify-file-length`; exit 0 when all pass. The run stops at the
   first failing step and exits with its code. The whitespace gate is not a step; run
   `cargo xtask ci verify-editorconfig` for it.

### Run one gate

3. List every task.

   ```bash
   cargo xtask help
   ```

   Expected: the tasks grouped as CI, schema, verify, map, build and db, each with its help line;
   `[alias]` marks a wrapper on a `cargo xtask verify` command and `[borrowed]` a `cargo xtask mk`
   or database recipe repeated here; then one line each for the `cargo xtask mk` and
   `cargo xtask db` lanes.

4. Run one task, here the schema gates that the `schema` job of `ci.yml` runs.

   ```bash
   cargo xtask ci ci-local-schema
   ```

   Expected: `verify-codegen-fresh` and `schema-validate` (three sub-gates: `validate`,
   `map-object-enums` and `type-inventory`) pass, exit 0. `cargo xtask ci <task>` runs any task
   `help` lists; an unknown name prints `xtask ci: no such task: <name>` and exits 2.

5. Run the app lane alone: formatting, clippy for `wasm32-unknown-unknown` and natively, the
   native tests and a release Trunk build.

   ```bash
   cargo xtask mk ci-local-leptos
   ```

   Expected: the five command lines in turn, exit 0. Both clippy runs pass `-D warnings` here and
   in the `frontend` job, so a warning fails, as in the [API](/documentation/glossary/a_to_f.md#api) and engine lanes.

6. Run the API's integration tests on a database of their own.

   ```bash
   cargo xtask db test-it
   ```

   Expected: the API's test binaries pass against a new randomly named database, dropped at the
   end; [Database operations](/documentation/runbooks/database_operations.md#run-the-integration-tests)
   has the options. `api_server` holds ten integration binaries: one per domain group, plus the
   whole-API `route_acceptance`, `contract_parity` and `smoke` suites.

### Run the browser gates

7. Build the app and run the editor smokes. They are not part of `ci-local`.

   ```bash
   cargo xtask mk leptos-gates
   ```

   Expected: the release build, `gate doctor` and the editor suite (selfcheck, editor,
   save-export and undo) pass. The environment, the single-smoke commands and the debug recipe
   are in [Editor gates](/documentation/runbooks/editor_gates.md).

### Check documentation

8. Check every link, backticked repository path and cited `cargo xtask` command.

   ```bash
   cargo xtask verify link-check --with-untracked --path documentation/runbooks
   ```

   Expected: exit 0; 1 lists each failing document with its break count and the first 20 breaks
   as `path:line: rule: message`, and `--report` prints all of them. `--path` repeats and takes
   folders only. No CI job and no `ci-local` step runs it; run it with `--with-untracked` before
   committing documentation so the new files are judged too.

### Gate a wave

9. On merged `main`, before a wave closes, run the wave gate over the wave's range.

   ```bash
   cargo xtask platform wave gate
   ```

   Expected: one `PASS` or `FAIL` line per step (every step runs; a failure shows its last 15
   lines), then `GATE: PASS` and exit 0. The base defaults to the last `wave N CLOSED` commit;
   `--slice <id>` runs the cheap slice gate in a slice worktree instead. The step lists of both
   are in the [wave gate README](/tools/commands/platform_execution/src/wave_execution/gate/README.md),
   and the whole wave procedure in [Factory waves](/documentation/runbooks/factory_waves/README.md).

## Gate matrix

Where each gate runs. "ci-local" means a step of `cargo xtask ci ci-local`; the job names are those
of `.github/workflows/ci.yml` unless the row names another workflow. In the last column "wave" is
the wave gate, "slice" the slice gate (a workspace `cargo check`, formatting of the changed files
and the tests of the changed frontend crates) and "both" the two. Rule ids (FMT-2, LANG-1, TEST-1
and the rest) are those of the [coding standards](/documentation/standards/coding_standards/README.md).

| Gate | Command | ci-local | GitHub | Wave gate |
|---|---|---|---|---|
| whitespace (FMT-2) | `cargo xtask ci verify-editorconfig` | no | `editorconfig` | no |
| no Python (LANG-2, LANG-3) | `cargo xtask verify no-python` | in `verify-language-bans` | `language-gates` | wave |
| no Node scripts | `cargo xtask verify no-node` | in `verify-language-bans` | `language-gates` | wave |
| no shell or Make (LANG-1) | `cargo xtask verify no-shell` | in `verify-language-bans` | `language-gates` | wave |
| workspace laws (WS-1, WS-2, WS-5): crate tiers with the firewalls, crate anatomy, Tailwind sources | `cargo xtask ci verify-workspace-laws` | yes | `language-gates`, one step per law | wave |
| test-file reachability, frontend layering (WS-3, WS-4) | `cargo xtask verify test-file-reachability`, `cargo xtask verify frontend-layering` | no | no | no |
| file length advice (SIZE-3; warns, never fails) | `cargo xtask verify file-length` | in `verify-file-length` | `language-gates` | wave |
| no `SELECT *` | `cargo xtask verify no-select-star` | no | no | no |
| Rust formatting | `cargo xtask mk rust-fmt` | in `rust-ci` | `api` | changed files (both) |
| API clippy, `-D warnings` (`api_server` and every other `crates/api` package) | `cargo xtask mk rust-clippy` | in `rust-ci` | `api` | wave |
| workspace compile check | `cargo check --workspace` | no | no | slice |
| wasm32 clippy `-D warnings` of every crate declaring `targets = "wasm32"` outside `crates/frontend` (derived from the workspace); the offline service worker is gated with the frontend family | `cargo xtask mk wasm-ci` | in `rust-ci` | `wasm-ci` | wave |
| API tests with Postgres (TEST-1; `api_server` and every other `crates/api` package) | `cargo xtask ci rust-test-it`; `cargo xtask db test-it` | in `rust-ci` | `api` (`cargo xtask ci api-test`) | wave |
| tests of every workspace member without a dedicated task (derived from `Cargo.toml`, so a new member is tested by default; the API crates run with the API tests; the binary-only `xtask` and `developer_tools` packages build there) | `cargo xtask ci workspace-member-tests` | yes | `workspace-members` | wave |
| frontend family, the app `frontend_application` and the offline service worker among it: fmt, clippy `-D warnings` (wasm32 and native), tests, Trunk build (TEST-2) | `cargo xtask mk ci-local-leptos` | yes | `frontend` | wave: clippy and tests, Trunk when a crate the app compiles changed; slice: tests of the changed frontend crates |
| generated contract types current | `cargo xtask ci verify-codegen-fresh` | in `ci-local-schema` | `schema` | wave |
| schema validation (TEST-3, ENF-4) | `cargo xtask ci schema-validate` | in `ci-local-schema` | `schema` | wave |
| map goldens: map-object golden, glyph atlas, height labels (need the LFS elevation raster) | `cargo xtask ci schema-map-goldens` | no | no | no |
| registry aliases | `cargo xtask verify object-registry-aliases` | no | no | no |
| ticket registry | `cargo xtask ticket check --strict` | no | no | no; `cargo xtask platform preflight` checks the registry |
| mod boot verdict self-test | `cargo xtask mod world-boot --selftest` | no | no | no |
| mod compile and world boot | `cargo xtask mod compile`, `cargo xtask mod world-boot` | no | `mod-gates.yml`, nightly on a self-hosted runner with the dedicated server | no |
| editor smokes | `cargo xtask mk leptos-gates` | no | `editor-gates.yml`, nightly and on demand | no |
| mortar calculator offline: pack download, service-worker reload, native solution | `cargo xtask mk mortar-offline-gate` | no | no; needs the local Everon tile index and the recorded catalog reads | no |
| links, backticked paths and cited commands in the documentation | `cargo xtask verify link-check` | no | no | no |

`ci.yml` runs on every push and pull request to `main` with no path filter; `editor-gates.yml`
and `mod-gates.yml` run nightly and on manual dispatch. The walls between the crates, with the
law that holds each, are in [Crate boundary rules](/documentation/standards/crate_boundary_rules.md).
The `[borrowed]` tasks repeat the `cargo xtask mk` recipes of the same name, and no test compares
the two copies, so a recipe change goes into both. Several coding-standards rules have no
automated gate, among them the Enfusion log policy and the authority comments (ENF-1, ENF-2),
checked in Workbench.

## Verify

```bash
cargo xtask ci ci-local
```

Expected: every step passes and the command exits 0. A change to the app or the map crates also
passes step 7; a change to documentation passes step 8.

GitHub's branch protection rule for `main` names its required status checks by the job names of
`.github/workflows/ci.yml`: the API job is `api (Rust 1.95 + Postgres 18)` and the app's job
`frontend (Leptos SPA)`; the one-time update of the rule to these names is step G1 of the
[S2 operator steps](/documentation/archive/restructure_agent_briefs/s2_a2_to_a6.md#oc-deploy-operator-steps).

## Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| `db up` exits 125 with "looking up compose provider failed" | podman has no compose provider | install `podman-compose` or the `docker-compose` plugin, or `podman start tbd_reforger_db` for an existing container |
| `rust-test-it` fails at `podman exec tbd_reforger_db` | the container is not running, or the machine has docker but no podman; the steps name podman | step 1; on a docker-only machine run `cargo xtask db test-it` for the integration tests |
| `verify-editorconfig` fails before checking anything | `editorconfig-checker` is absent, and Go is missing or its `bin` folder is not on `PATH` | install the checker, or Go with its `bin` folder on `PATH` |
| the terrain or world-object tests or `height-labels` fail to decode the elevation raster | the raster is an LFS pointer file | `cargo xtask ci lfs-dem` |
| `cargo fmt` or clippy fails on files the change never touched | another session's uncommitted work in the same tree | judge the failure by path; gate a clean checkout of `main` |
| `xtask ci: no such task: <name>` | the name is a `cargo xtask mk` or `cargo xtask verify` command, not a `ci` task | `cargo xtask help` lists the `ci` tasks and names the other lanes |
| `link-check` exits 2 | `--path` named a file | pass the file's folder |
| the wave gate prints `FAIL (TIMEOUT after <n>s)` | a step outran `TBD_GATE_TIMEOUT` (1200 s by default) | raise `TBD_GATE_TIMEOUT` or look for a hung step |

## Related

- [CI task catalog](/tools/commands/ci_task_catalog/src/README.md) — every `cargo xtask ci` task and
  its steps.
- [Verify command group](/tools/xtask/src/commands/verify/README.md) — what each
  `cargo xtask verify` gate checks, and the crate that holds it.
- [Editor gates](/documentation/runbooks/editor_gates.md) — the browser gates in detail.
- [Local development](/documentation/runbooks/local_development.md) — the local stack the gates
  run against.
- [Database operations](/documentation/runbooks/database_operations.md) — the integration test
  database and its options.
- [Factory waves](/documentation/runbooks/factory_waves/README.md) — the wave procedure the wave
  gate belongs to.
- [Coding standards](/documentation/standards/coding_standards/README.md) and
  [Crate boundary rules](/documentation/standards/crate_boundary_rules.md) — the rules the
  gates enforce.
