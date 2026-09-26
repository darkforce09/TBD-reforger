# Platform factory preflight checks

The checks behind `cargo xtask platform preflight`: whether this machine and checkout can run an
unattended factory [wave](/documentation_v2/glossary/n_to_z.md#wave) before one starts.

## Contents

```text
tools_v2/xtask/src/commands/platform/preflight/
├── execution.rs  `run`: every check in order, the BLOCK and WARN tally, and the exit code
└── ok.rs         the line printers and every probe: bridge, disk, memory, git, ports, processes, run target
```

## How it works

`tools_v2/xtask/src/commands/platform/preflight.rs` holds the run-target states and declares both
files. `run` moves to the repository root (or `TBD_PREFLIGHT_ROOT`) and prints one line per
check: ✓ for fine, ✗ BLOCK for a stop, ! WARN for a risk.

| Check | BLOCK when | WARN when |
|---|---|---|
| host bridge, cargo | in a container with a dead `distrobox-host-exec`; `cargo --version` fails | |
| disk, reclaimable | under 20 GB free | 20 to 40 GB free; orphan build caches in `/var/tmp` |
| `CARGO_TARGET_DIR`, per-worktree target | a worktree built into its own `target/` | the variable is unset |
| run target | binaries under the run target with no `tbd-built-from` stamp, or one naming another sha or checkout | |
| memory, swap | under 1024 MiB available | swap 70 % used or more |
| working tree, branch, remote | dirty tree; not on `main` | commits not pushed |
| worktrees | | stale worktrees; worktrees idle over `TBD_IDLE_WORKTREE_MIN` minutes |
| ticket check, wave lock | `cargo xtask ticket check` or `cargo xtask wave check` fails | |
| postgres :5434, API :8080 | | database down; `/healthz` not 200, or the API process older than the newest API commit |
| trunk serve, chrome | | stray Chrome processes |

The summary is `PREFLIGHT: PASS (<n> warn)` or `PREFLIGHT: <n> BLOCK, <n> warn — DO NOT START`.
The run target and its stamp come from
`crate::commands::platform::wave_execution::resolve_run_target_dir`.

## Boundaries

- Depends on: `crate::commands::platform::wave_execution` (`RunStamp`, `read_run_stamp`,
  `resolve_run_target_dir`), `ticket_engine` (the wave lock), `git`, `df`, `pgrep`, `curl`, `ss`
  and `/proc`.
- Used by: `tools_v2/xtask/src/commands/platform/preflight.rs`, which re-exports `run` to the
  `platform` dispatch.
- Rules: a missing or unreadable stamp blocks like a stale one, so unknown provenance is never
  green (`tools_v2/xtask/src/commands/platform/tests/preflight/run_target_tests.rs`); `--warn`
  changes only the exit code, never which lines print.

## Related documentation

- [Cold start and preflight](/documentation_v2/runbooks/factory_waves/cold_start_and_preflight.md) — where preflight sits in
  an orchestrating session, and what to do about each BLOCK.
