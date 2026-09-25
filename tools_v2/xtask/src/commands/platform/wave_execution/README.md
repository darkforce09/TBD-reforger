# Platform wave driver

The implementation of `cargo xtask platform wave`, the platform factory: it reads the
[wave](/documentation_v2/glossary.md#wave) plan, creates slice worktrees, runs the slice and wave
gates, lands slices on `main`, closes waves and pushes. It also keeps launched binaries and slice
tests off the shared cargo cache.

## Contents

```text
tools_v2/xtask/src/commands/platform/wave_execution/
├── archived_wave_plans.rs  the ticket set a wave held at a past revision, from `ticket_engine`
├── base/                   base derivation for the wave gate: markers, oracles and confirmation
├── base.rs                 wave gate base: the marker grammar and the re-exports of base/
├── changed/                changed-file lists, rustfmt, the wasm scope and `include!` consumers
├── changed.rs              the default diff base and the frontend crate path; re-exports changed/
├── db.rs                   the gate's per-wave integration database and the API test step
├── flush.rs                output capture, git helpers, the run target and stamp, `run`, the dispatch
├── gate/                   the slice gate and the full wave gate
├── gate.rs                 the step runner and the ten shared `xtask verify` steps
├── host.rs                 `hostrun` and `checkrun`: host-bridge argv, gate timeout and output capture
├── land/                   `land`, `revert`, `verified` and the `wave --close` ceremony
├── land.rs                 re-exports of land/ and the close-ceremony contract
├── ledger.rs               wave lock, ticket registry and worktree readers; the current wave
├── lock.rs                 the gate lock that serialises gates across worktrees
├── migrate/                the migration claim-body pin and the persistent database step
├── migrate.rs              the persist-database design; re-exports migrate/
├── mod.rs                  `Ctx`, the help text, output macros, `RunStamp` and the module tree
├── push.rs                 `push`, with the guard that asks git which paths are LFS
├── reclaim/                the orphan build-cache sweep
├── reclaim.rs              the reclaim policy; re-exports reclaim/
├── schema.rs               the gate's schema step: `cargo xtask ci ci-local-schema`
├── status.rs               `status`, `prep` and `wave`, the read-only commands
├── test_cmd.rs             `test --slice <id>`: cargo test into a per-slice private target folder
├── tests/                  unit tests for every module of the driver
├── touch.rs                fingerprint invalidation and the changed-crates clippy step
├── trunk.rs                `trunk build --release` into the gate's private dist and target folders
└── verdict.rs              the gate verdict receipt `land` and `slice-worktree merge` read
```

## How it works

`flush::run` builds a `Ctx` once, and `Ctx::enter` does four things. It moves to the repository
root. It finds the main checkout through `git rev-parse --git-common-dir`. It exports
`CARGO_TARGET_DIR` (inherited, else `<main checkout>/target`) and `TBD_RUN_TARGET_DIR`
(`<shared target>/run-main`). It detects the host bridge. It then dispatches on the first
argument; the default is `status`.

```text
status ─ prep ─▶ (slice agents work in .ai/artifacts/worktrees/<id>)
                  gate --slice <id> ─▶ verdict receipt
land ─▶ merge ready slices ─▶ gate (wave) ─▶ drop worktrees ─▶ repack lock ─▶ push
verified <sha> ─▶ wave --close ─▶ marker commit ─▶ next wave
```

- Tiered gates: a slice pays the cheap gate in its worktree; the full suite runs once on merged
  `main` (see `gate/`). Both take the gate lock at
  `<main checkout>/target/.repository-verification.lock`, so two gates never build into the
  same private folders at once.
- Per-slice landing: `land` merges each slice the moment it is committed, clean, gate-green and
  receipted; `land --wave` waits for the whole wave.
- Private target folders: `test --slice <id>` builds into `$HOME/.cache/tbd-target-<id>`. `run`
  builds into `run-main`, writes a `tbd-built-from` stamp (`<sha> <checkout>`) beside the
  binaries, and refuses to run from a worktree. `cargo xtask platform preflight` blocks on a
  missing or stale stamp.
- Every line a step prints goes through `emit`, so a step's output is captured and shown only on
  failure, in the order it was written.
- `push` runs a plain `git push origin main` when git-lfs is installed. Without it, it pushes with
  `--no-verify` only when no path in the range resolves to `filter: lfs`, and refuses when it
  cannot tell.

## Public surface

- `run`: the `platform wave` entry, called by `tools_v2/xtask/src/commands/platform/dispatch.rs`.
- `RunStamp`, `RUN_STAMP_FILE`, `read_run_stamp` and `resolve_run_target_dir`: read by
  `tools_v2/xtask/src/commands/platform/preflight.rs` and its `preflight/` files.
- `verdict::land_refusal`: called by `slice-worktree merge`
  (`tools_v2/xtask/src/commands/platform/slice_worktree/git_plain.rs`).
- `testcwd`, compiled only for tests: the cwd lock `crate::core::repository_root` takes under test.

## Boundaries

- Depends on:
  - `crate::core` (`repository_root`, `repository_layout`, `host_execution`,
    `cargo_target_directory`);
  - `crate::commands::platform::slice_worktree` (`new`, `drop`);
  - `ticket_engine` (`wave_lock`, `registry`, `metrics`, `repository`) and `verification_core`
    (`lock`, `proc`);
  - `git`, `cargo`, `trunk`, `rustfmt`, `psql` through `podman exec tbd_reforger_db`,
    `sha384sum`, and the `slice-collisions` command.
- Used by: `cargo xtask platform wave`; `tools_v2/xtask/src/commands/platform/preflight/`;
  `tools_v2/xtask/src/commands/platform/slice_worktree/`; the source audit
  `tools_v2/xtask/src/verifications/architecture/wave_gate_sources.rs`, which checks that `gate.rs`
  declares and exports `gate_slice` and `cmd_gate`; `.github/workflows/ci.yml`, whose schema job
  runs the same `ci ci-local-schema` set as the gate's schema step.
- Rules: an unreadable wave lock is a refusal, never an empty plan (`ledger`); `land` refuses a
  slice without a green verdict for its tip sha (`verdict`); a failing step never stops the gate
  early; the tests for each module are under `tests/<module>/`.

## Related documentation

- [Factory waves](/documentation_v2/runbooks/factory_waves/README.md) — the platform factory
  procedure this driver automates.
- [Running a wave](/documentation_v2/runbooks/factory_waves/running_a_wave.md) — every
  subcommand in the order a wave uses it.
- [Known traps](/documentation_v2/runbooks/factory_waves/known_traps.md) — why the driver keeps
  private target folders, the run stamp and the step capture.
