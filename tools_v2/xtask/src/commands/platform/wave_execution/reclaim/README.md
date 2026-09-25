# Build cache reclaim

The implementation of `cargo xtask platform wave reclaim`: it deletes build caches that slices
left behind, and spares every cache that belongs to a slice whose worktree still exists.

## Contents

```text
tools_v2/xtask/src/commands/platform/wave_execution/reclaim/
├── adhoc_token.rs  the slice id in a `tbd-target-<id>` folder name, folder age in days, free disk space
└── du_mb.rs        `cmd_reclaim`: argument parsing, the live-slice set, every sweep and the summary
```

## How it works

`tools_v2/xtask/src/commands/platform/wave_execution/reclaim.rs` states the policy and re-exports
`cmd_reclaim`. `cmd_reclaim` first reads the live slices from `git worktree list` and prints them
as spared, then sweeps:

| Where | What | Default |
|---|---|---|
| `/var/tmp` | folders whose names contain `target`, start `v2-`, or start `t` and a digit and end `-probe` or `-dist` | swept |
| the main checkout | per-slice private folders `target-<slice>` and `target-<slice>-api` | swept; `--no-slice-dirs` skips |
| `$HOME/.cache` | per-slice test folders `tbd-target-<id>` from `platform wave test` | swept; `--no-slice-dirs` skips |
| the main checkout | gate folders `target-gate-*` and `dist-gate-*` | kept; `--gate-dirs` sweeps, `--gate-dirs-older-than-days <n>` only those older than n days |

Each removal prints the folder and its size from `du -sm`, and the run ends with the free space.
When `git worktree list` does not answer, the sweeps of the main checkout and `$HOME/.cache`
refuse rather than treat every slice as finished. An unknown argument exits 2.

## Boundaries

- Depends on: `super::Ctx` (the main checkout root), `git`, `du` and `df`.
- Used by: the `reclaim` dispatch in
  `tools_v2/xtask/src/commands/platform/wave_execution/flush.rs`; `cargo xtask platform preflight`
  names the command when it sees reclaimable caches.
- Rules: a live slice's cache is never removed; the warm gate folders stay unless asked for; the
  tests are in `tools_v2/xtask/src/commands/platform/wave_execution/tests/reclaim/tests.rs`.
