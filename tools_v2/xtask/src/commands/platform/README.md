# Platform factory commands

The `cargo xtask platform` group: the tools that run the platform factory. They manage slice
worktrees, check a machine before an unattended run, run one slice through the agent CLI with a
token receipt, and drive the platform [wave](/documentation_v2/glossary.md#wave) lifecycle from
worktree creation to landing on `main`. The command center, factory dispatchers and slice agents
run them.

## Contents

```text
tools_v2/xtask/src/commands/platform/
├── cli.rs               the `PlatformCmd` clap enum: the four subcommands and their flags
├── dispatch.rs          routes each `PlatformCmd`; loads the ticket registry for `slice-run`
├── mod.rs               the module tree
├── preflight/           the preflight checks and their probes
├── preflight.rs         `platform preflight`: run-target states and module wiring
├── slice_execution.rs   `platform slice-run`: one slice through the agent CLI, and its run receipt
├── slice_worktree/      the worktree subcommands and their guards
├── slice_worktree.rs    `platform slice-worktree`: usage text and module wiring
├── tests/               unit tests for the preflight run target and `slice-run`
└── wave_execution/      the platform wave driver, `platform wave`
```

## How it works

`tools_v2/xtask/src/cli/mod.rs` mounts `PlatformCmd` as the `platform` group, and `dispatch::run`
sends each variant to its module. `preflight` and `slice-run` parse with clap. `slice-worktree`
and `wave` take their arguments raw, with clap's help flag turned off, and parse them in their own
module, so `--help` reaches their usage text.

Slice worktrees live under `.ai/artifacts/worktrees/<slice>/` on branches `slice/<slice>`, the
one branch shape the tooling creates and deletes itself. The `mod wave` driver
(`tools_v2/xtask/src/commands/mod_ops/`) calls `slice-worktree` in-process for the mod program.

```text
platform preflight ─▶ platform wave prep ─▶ platform slice-run <id>   (agent in the worktree)
                                             platform wave gate --slice <id>
                      platform wave land ◀───┘   (merge, wave gate, drop, repack, push)
```

## Commands

Run each as `cargo xtask platform <subcommand>` from the repository root. An error that escapes
an entry function prints `xtask: <cause>` and exits 1; a clap usage error exits 2.

### slice-worktree

- Synopsis: `platform slice-worktree -- <new <slice> | list | merge <slice> | drop <slice> [--force] | reap>`
- Does: `new` creates the worktree and branch from `main` and links the oracle lanes; `list` runs
  `git worktree list`; `merge` merges a clean slice whose gate verdict is green for its tip;
  `drop` deletes one worktree and branch; `reap` deletes every merged, clean worktree.
- Exit codes: 0 done; 1 a refusal (dirty, unmerged, missing lane, no worktree); 2 no slice id, an
  unknown subcommand (usage printed) or a missing gate verdict.
- Example: `cargo xtask platform slice-worktree -- list`

### preflight

- Synopsis: `platform preflight [--warn]`
- Does: prints one line per check (host bridge, cargo, disk, target folders, the run-target stamp,
  memory, working tree, branch, worktrees, `ticket check`, `wave check`, Postgres on 5434, the API
  on 8080, stray Chrome) and a PASS or BLOCK summary.
- Exit codes: 0 no BLOCK, or any result with `--warn`; 1 at least one BLOCK.
- Example: `cargo xtask platform preflight`

### wave

- Synopsis: `platform wave <subcommand>`, default `status`:
  - `status`, `prep`, `wave`: where the current wave stands and what blocks it; worktrees for the
    next disjoint set (from `cargo xtask slice-collisions`); the wave's shipped count, open
    tickets and verify debt.
  - `gate [<base>]`, `gate --slice <id>`, `gate --migrate-persist [audit|advance]`: the wave gate,
    the slice gate, and the persistent migration database step alone.
  - `test --slice <id> <cargo test arguments>`: cargo test into a per-slice private target folder.
  - `run <cargo arguments> [-- <program arguments>]`: build and launch from the main checkout into
    the stamped run target.
  - `land [--wave] [--bookkeeping] [<ticket id>…]`, `revert <sha>`, `verified <sha>`, `push`,
    `reclaim [--gate-dirs] [--gate-dirs-older-than-days <n>] [--no-slice-dirs]`.
  - `wave --close [--summary <text>] [--tickets <ids>] [--dry-run]`, spelled
    `platform wave wave --close`: the wave-close marker commit.
- Does: drives the factory lifecycle; the wave driver's README describes each subcommand.
- Exit codes: 0 done; 1 a failed gate step, a refusal, or an unknown subcommand, which prints the
  help; 2 a refused base, range or argument.
- Example: `cargo xtask platform wave status`

### slice-run

- Synopsis: `platform slice-run [--fixture <FIXTURE>] [--started <STARTED>] [--dry-run] <ID>`
- Does: resolves the ticket, or a slice through its parent's slice plan, and refuses unless its
  executor is `claude-code` and its spec exists. It then runs the agent command
  (`TBD_SLICE_RUN_AGENT_CMD`, default `claude --print --output-format json`) in the slice
  worktree, or at the root when there is none, and writes a run receipt with the reported tokens
  under `.ai/tickets/metrics/<id>/`. `--fixture` replays a recorded answer; `--dry-run` writes
  nothing.
- Exit codes: 0 receipt written or dry run; 1 a refusal, a failed agent, or an answer without a
  usage object, which writes no receipt.
- Example: `cargo xtask platform slice-run <ticket id> --dry-run`

## Boundaries

- Depends on: `crate::core` (`repository_root`, `repository_layout`, `host_execution`,
  `cargo_target_directory`); `crate::commands::ticket` (`load_registry`); the `ticket_engine`
  (`registry`, `metrics`, `wave_lock`) and `verification_core` crates; `git`, `cargo`, `trunk`,
  the local Postgres container and the agent CLI.
- Used by:
  - `tools_v2/xtask/src/cli/mod.rs` and `tools_v2/xtask/src/cli/dispatch.rs`, which mount the group;
  - `cargo xtask ticket run`, which calls `slice_execution::run_slice` for each ready slice
    (`tools_v2/xtask/src/commands/ticket/execution.rs`);
  - `cargo xtask mod wave`, which calls `slice_worktree::run_at`;
  - the command center, factory dispatchers and slice agents.
- Rules:
  - A slice-run that exits 0 without a usage object fails and writes no receipt
    (`exit_zero_without_usage_fails_and_writes_no_file` in `tests/slice_execution/tests.rs`).
  - A slice whose executor is not `claude-code` is refused
    (`non_claude_code_executor_is_refused`).
  - Unknown run-target provenance blocks preflight (`tests/preflight/run_target_tests.rs`).
  - `slice/<slice>` branches are created and deleted only by `slice-worktree` and the wave drivers.

## Related documentation

- [Factory waves](/documentation_v2/runbooks/factory_waves/README.md) — the platform factory
  procedure `platform wave` automates.
- [Cold start and preflight](/documentation_v2/runbooks/factory_waves/cold_start_and_preflight.md)
  — `preflight`, `reclaim` and `status` at the start of a session.
- [Running a wave](/documentation_v2/runbooks/factory_waves/running_a_wave.md) — `slice-worktree`,
  `slice-run` and `platform wave` step by step.
- [Mod slice workflow](/documentation_v2/runbooks/mod_slice_workflow.md) — the slice worktree
  lifecycle the usage text points to.
