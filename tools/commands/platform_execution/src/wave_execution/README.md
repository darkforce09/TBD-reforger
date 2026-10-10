# Platform wave driver

The implementation of `cargo xtask platform wave`, the platform factory: it reads the
[wave](/documentation/glossary/n_to_z.md#wave) plan from the central ticket manager (`ttm wave
show`), creates slice worktrees, runs the slice and wave
gates, lands slices on `main`, closes waves and pushes. It also keeps launched binaries and slice
tests off the shared cargo cache.

## Contents

```text
tools/commands/platform_execution/src/wave_execution/
├── base/                   base derivation for the wave gate: the marker ledger, oracles and confirmation
├── base.rs                 wave gate base: what each oracle proves, and the re-exports of base/
├── changed/                changed-file lists, rustfmt, the wasm scope and `include!` consumers
├── changed.rs              the default diff base and the frontend crate path; re-exports changed/
├── db.rs                   the gate's per-wave integration database and the API test step
├── flush.rs                output capture, git helpers, the run target and stamp, `run`, the dispatch
├── gate/                   the slice gate and the full wave gate
├── gate.rs                 the step runner and the ten shared `xtask verify` steps
├── host.rs                 `hostrun` and `checkrun`: host-bridge argv, gate timeout and output capture
├── land/                   `land`, `revert`, `verified` and the `wave --close` ceremony
├── land.rs                 re-exports of land/ and the close-ceremony contract
├── ledger.rs               wave plan, ticket completion and worktree readers; the current wave; verify debt
├── lock.rs                 the gate lock that serialises gates across worktrees
├── migrate/                the persistent database migration step
├── migrate.rs              the persist-database design; re-exports migrate/
├── mod.rs                  `Ctx` (with the ticket manager and its wave plan snapshot), the help text, output macros, `RunStamp` and the module tree
├── push.rs                 `push`, with the guard that asks git which paths are LFS
├── reclaim/                the orphan build-cache sweep
├── reclaim.rs              the reclaim policy; re-exports reclaim/
├── schema.rs               the gate's schema step: the sub-gates of `cargo xtask ci schema-validate`
├── status.rs               `status`, `prep` and `wave`, the read-only commands
├── test_cmd.rs             `test --slice <id>`: cargo test into a per-slice private target folder
├── touch.rs                fingerprint invalidation before the gates' cargo steps
├── trunk.rs                `trunk build --release` into the gate's private dist and target folders
└── verdict.rs              the gate verdict receipt `land` and `slice-worktree merge` read
```

## How it works

`flush::run` first drops an inherited `CARGO_TARGET_DIR`, saying so on stderr
(`disown_ambient_target_dir`), because a value set in the container and read by host cargo
poisons the cache it names. It then builds a `Ctx` once, and `Ctx::enter` does four things. It
moves to the repository root. It finds the main checkout through `git rev-parse --git-common-dir`.
It exports `CARGO_TARGET_DIR` as `<main checkout>/target` and `TBD_RUN_TARGET_DIR` (an inherited
value, else `<main checkout>/target/run-main`). It detects the host bridge and binds the ticket
manager client (`TBD_TTM_BIN`, `TBD_TTM_PROJECT`); the wave plan is read once on first use and
read again after every land, repack and close. It then dispatches on the first argument; the
default is `status`.

```text
status ─ prep ─▶ (slice agents work in .ai/artifacts/worktrees/<id>)
                  gate --slice <id> ─▶ verdict receipt
land ─▶ merge ready slices + ttm land ─▶ gate (wave) ─▶ drop worktrees ─▶ ttm wave repack ─▶ push
verified <sha> ─▶ wave --close ─▶ marker commit ─▶ ttm wave close ─▶ next wave
```

- Tiered gates: a slice pays the cheap gate in its worktree; the full suite runs once on merged
  `main` (see `gate/`). Both take the gate lock at
  `<main checkout>/target/.repository-verification.lock`, so two gates never build into the
  same private folders at once.
- Per-slice landing: `land` merges each slice the moment it is committed, clean, gate-green and
  receipted; `land --wave` waits for the whole wave.
- Private target folders: each gate step that builds gets its own `CARGO_TARGET_DIR` (or trunk
  dist folder) under `<main checkout>/target/` — `gate-check`, `gate-schema`, `gate-trunk`,
  `gate-dist-frontend`, `gate-api`, `gate-frontend`, `gate-tools`, and
  `gate-slice-frontend-<slice>` per slice — named in `tools/foundation/repository_layout/src/build_output.rs`;
  `TBD_GATE_CHECK_TARGET`, `TBD_GATE_SCHEMA_TARGET`, `TBD_GATE_TRUNK_TARGET` and
  `TBD_GATE_TRUNK_DIST` override the first four. `test --slice <id>` builds into
  `$HOME/.cache/tbd-target-<id>`. `run`
  builds into `run-main`, writes a `tbd-built-from` stamp (`<sha> <checkout>`) beside the
  binaries, and refuses to run from a worktree. `cargo xtask platform preflight` blocks on a
  missing or stale stamp.
- Every line a step prints goes through `emit`, so a step's output is captured and shown only on
  failure, in the order it was written.
- `push` runs a plain `git push origin main` when git-lfs is installed. Without it, it pushes with
  `--no-verify` only when no path in the range resolves to `filter: lfs`, and refuses when it
  cannot tell.

## Public surface

- `run`: the `platform wave` entry, called by `tools/commands/platform_execution/src/platform_dispatch.rs`.
- `RunStamp`, `RUN_STAMP_FILE`, `read_run_stamp` and `resolve_run_target_dir`: read by
  `tools/commands/platform_execution/src/preflight.rs` and its `preflight/` files.
- `verdict::land_refusal`: called by `slice-worktree merge`
  (`tools/commands/platform_execution/src/slice_worktree/git_plain.rs`).

## Boundaries

- Depends on:
  - `repository_layout` (with its `build_output` subfolders and `RUN_TARGET_SUBDIR`),
    `process_runner::host_execution` and `ci_task_catalog::cargo_target_pin` (the glibc
    guard);
  - `crate::slice_worktree` (`new`, `drop`);
  - `ticket_manager_client` (the wave plan, ticket statuses, landings, repacks, the close record
    and the wave history oracle 2 reads) and `verification_core` (`lock`, `proc`);
  - `git`, `cargo`, `trunk`, `rustfmt`, `psql` through `podman exec tbd_reforger_db`,
    `sha384sum`, and `ttm` (through the client, and `ttm wave collisions` for `prep`).
- Used by: `cargo xtask platform wave`; `tools/commands/platform_execution/src/preflight/`;
  `tools/commands/platform_execution/src/slice_worktree/`; `.github/workflows/ci.yml`, whose schema job runs the
  same `ci schema-validate` sub-gates as the gate's schema step.
- Rules: a wave plan the ticket manager cannot give is a refusal, never an empty plan (`ledger`); `land` refuses a
  slice without a green verdict for its tip sha (`verdict`); a failing step never stops the gate
  early.

## Related documentation

- [Factory waves](/documentation/runbooks/factory_waves/README.md) — the platform factory
  procedure this driver automates.
- [Running a wave](/documentation/runbooks/factory_waves/running_a_wave.md) — every
  subcommand in the order a wave uses it.
- [Known traps](/documentation/runbooks/factory_waves/known_traps.md) — why the driver keeps
  private target folders, the run stamp and the step capture.
