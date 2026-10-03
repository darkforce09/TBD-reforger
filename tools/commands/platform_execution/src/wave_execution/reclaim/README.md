# Build cache reclaim

The implementation of `cargo xtask platform wave reclaim`: it deletes build caches that slices
left behind, and spares every cache that belongs to a slice whose worktree still exists.

## Contents

```text
tools/commands/platform_execution/src/wave_execution/reclaim/
├── adhoc_token.rs           the slice id in a `tbd-target-<id>` folder name, folder age in days, free disk space
├── build_output_folders.rs  the `target/` report, the opt-in `target/gate-*` sweep, the retired root-level folders
└── reclaim_command.rs       `cmd_reclaim`: argument parsing, the live-slice set, the slice sweeps and the summary
```

## How it works

`tools/commands/platform_execution/src/wave_execution/reclaim.rs` states the policy and re-exports
`cmd_reclaim`. `cmd_reclaim` first reads the live slices from `git worktree list` and prints them
as spared, then sweeps:

| Where | What | Default |
|---|---|---|
| `/var/tmp` | folders whose names contain `target`, start `v2-`, or start `t` and a digit and end `-probe` or `-dist` | swept |
| the main checkout | retired root-level build folders `target-dev-api`, `target-ci`, `target-dev-mcpd`, `target-mk-db-selftest`, `target-gate-*` and `dist-gate-*`, which no tool writes | swept |
| the main checkout | per-slice private folders `target-<slice>` and `target-<slice>-api` | swept; `--no-slice-dirs` skips |
| `$HOME/.cache` | per-slice test folders `tbd-target-<id>` from `platform wave test` | swept; `--no-slice-dirs` skips |
| the main checkout's `target/` | the shared cache, `target/dev-api`, `target/ci`, `target/dev-mcpd` and `target/db-selftest` | measured and kept (`cargo xtask mk reclaim-target-ci` deletes `target/ci`) |
| the main checkout's `target/` | gate folders `target/gate-*` | kept; `--gate-dirs` sweeps, `--gate-dirs-older-than-days <n>` only those older than n days |

Each removal prints the folder and its size from `du -sm`, and the run ends with the free space.
When `git worktree list` does not answer, the sweeps of the main checkout and `$HOME/.cache`
refuse rather than treat every slice as finished. An unknown argument exits 2.

## Boundaries

- Depends on: `super::Ctx` (the main checkout root), `git`, `du` and `df`.
- Used by: the `reclaim` dispatch in
  `tools/commands/platform_execution/src/wave_execution/flush.rs`; `cargo xtask platform preflight`
  names the command when it sees reclaimable caches.
- Rules: a live slice's cache is never removed; the warm gate folders stay unless asked for; the
  shared cache and the development API build are never removed; the folder names come from
  `tools/foundation/repository_layout/src/build_output.rs`; the tests are in `tools/commands/platform_execution/src/wave_execution/tests/reclaim/tests.rs`.

## Related documentation

- [Cold start and preflight](/documentation/runbooks/factory_waves/cold_start_and_preflight.md) — when to reclaim in an
  orchestrating session.
