# Platform factory commands

The `cargo xtask platform` group: the tools that run the platform factory. They manage slice
worktrees, check a machine before an unattended run, run one slice through the agent CLI with a
token receipt, and drive the platform [wave](/documentation/glossary/n_to_z.md#wave) lifecycle from
the next dispatch set to landing on `main`. The
[orchestrator](/documentation/glossary/n_to_z.md#orchestrator), factory dispatchers and slice agents
run them.

## Contents

```text
tools/commands/platform_execution/src/
├── error.rs              the crate's `Error` and `Result`: why a platform command could not run
├── lib.rs                the crate root: the module tree and the public entries
├── platform_command.rs   the `PlatformCmd` clap enum: the four subcommands and their flags
├── platform_dispatch.rs  routes each `PlatformCmd`; loads the ticket registry for `slice-run`
├── prelude.rs            the names a caller imports with one `use`
├── preflight/            the preflight checks and their probes
├── preflight.rs          `platform preflight`: run-target states and module wiring
├── slice_execution.rs    `platform slice-run`: one slice through the agent CLI, and its run receipt
├── slice_worktree/       the worktree subcommands and their guards
├── slice_worktree.rs     `platform slice-worktree`: usage text and module wiring
└── wave_execution/       the platform wave driver, `platform wave`
```

## How it works

`tools/xtask/src/cli/mod.rs` mounts `PlatformCmd` as the `platform` group, and the crate's `run`
(`platform_dispatch.rs`) sends each variant to its module. `preflight` and `slice-run` parse with clap. `slice-worktree`
and `wave` take their arguments raw, with clap's help flag turned off, and parse them in their own
module, so `--help` reaches their usage text.

Slice worktrees live under `.ai/artifacts/worktrees/<slice>/` on branches `slice/<slice>`, the
one branch shape the tooling creates and deletes itself. The `mod wave` driver
(`tools/commands/mod_operations/src/`) calls `slice_worktree::run_at` in-process for the mod
program. Every child process runs through `process_runner`: captured runs in their own process
group, and the steps that share the operator's terminal (`git merge`, `git push`, `cargo run`) in
this process's group, so Ctrl-C reaches them.

```text
platform preflight ─▶ platform wave prep   (prints the next disjoint set)
                      platform slice-worktree -- new <id>
                      platform slice-run <id>   (agent in the worktree)
                      platform wave gate --slice <id>
                      platform wave land   (merge, wave gate, drop, repack, push)
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
  - `status`, `prep`, `wave`: where the current wave stands and what blocks it; the next
    disjoint dispatch set (from `cargo xtask slice-collisions`), printed with the
    `slice-worktree -- new <TICKET>` line that creates each worktree, since `prep` creates none;
    the wave's shipped count, open tickets and verify debt.
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

- Depends on: `repository_layout` (with its `build_output` subfolders), `repository_laws` (the
  workspace members), `process_runner`, `ci_task_catalog` (the glibc guard, the schema gate list,
  the member package list), `time_source`; the `ticket_registry` (`load_registry`), `ticket_metrics`, `ticket_wave_lock`, `ticket_model` and
  `verification_core` crates; `git`, `cargo`, `trunk`, the local Postgres container and the agent
  CLI.
- Used by:
  - `tools/xtask/src/cli/mod.rs` and `tools/xtask/src/cli/dispatch.rs`, which mount the group;
  - `cargo xtask ticket run`, which calls `slice_execution::run_slice` for each ready slice
    (`tools/xtask/src/commands/ticket/execution.rs`);
  - `cargo xtask mod wave`, which calls `slice_worktree::run_at`
    (`tools/commands/mod_operations/src/wave_execution/`);
  - the orchestrator, factory dispatchers and slice agents.
- Rules:
  - A slice-run that exits 0 without a usage object fails and writes no receipt.
  - A slice whose executor is not `claude-code` is refused.
  - Unknown run-target provenance blocks preflight.
  - `slice/<slice>` branches are created and deleted only by `slice-worktree` and the wave drivers.

## Related documentation

- [Factory waves](/documentation/runbooks/factory_waves/README.md) — the platform factory
  procedure `platform wave` automates.
- [Cold start and preflight](/documentation/runbooks/factory_waves/cold_start_and_preflight.md)
  — `preflight`, `reclaim` and `status` at the start of a session.
- [Running a wave](/documentation/runbooks/factory_waves/running_a_wave.md) — `slice-worktree`,
  `slice-run` and `platform wave` step by step.
- [Mod slice workflow](/documentation/runbooks/mod_slice_workflow.md) — the slice worktree
  lifecycle the usage text points to.
- [Taking a ticket from idea to shipped](/documentation/runbooks/ticket_run_pipeline.md) — how
  `ticket run` hands each ready ticket to `slice-run`, and where the run receipt goes.
