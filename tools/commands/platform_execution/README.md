# Platform execution

The `platform_execution` crate: the platform factory behind `cargo xtask platform`. It drives the
platform [wave](/documentation/glossary/n_to_z.md#wave) lifecycle (worktree preparation, the cheap
per-slice gate and the full wave gate, per-slice landing on `main`, the wave close and the guarded
push), runs one slice through the agent CLI and writes its run receipt, creates, merges, drops and
reaps the per-slice git worktrees, and checks a machine before an unattended run. The
[orchestrator](/documentation/glossary/n_to_z.md#orchestrator), the factory dispatchers and the
slice agents run it.

## Contents

```text
tools/commands/platform_execution/
├── Cargo.toml  the `platform_execution` library package: `ci_task_catalog`, the ticket crates and the tool foundations, layout tier 9
└── src/        the wave driver, the slice runner, the slice worktree lifecycle, the preflight and the errors
```

## How it works

The xtask binary parses the command line and calls the crate: `platform <subcommand>` reaches
`run` with the parsed `PlatformCmd`; `ticket run` calls `slice_execution::run_slice` for each
ready slice; the mod wave driver calls `slice_worktree::run_at` in-process with the root it holds.
`slice-worktree` and `wave` take their arguments raw and parse them in their own modules, so their
usage text answers `--help`.

The wave driver changes the working directory to the checkout root at entry and resolves the
shared `CARGO_TARGET_DIR`, the private gate folders and the run target once. Gate steps run
through the container-to-host bridge, each step's output captured and shown only on failure; a
step that cannot run is red, never a pass. `land` merges a slice only when a gate verdict receipt
records a pass for its exact head. Every child process runs through `process_runner`.

The commands themselves are described in the
[source README](/tools/commands/platform_execution/src/README.md) and the
[wave driver README](/tools/commands/platform_execution/src/wave_execution/README.md).

## Boundaries

- Depends on: `ci_task_catalog` (the schema gate list, the member package list, the glibc stamp
  guard), `ticket_model`, `ticket_metrics`, `ticket_registry`, `ticket_wave_lock`,
  `process_runner`, `repository_layout`, `repository_laws`, `verification_core`, `time_source`,
  `clap`, `regex`, `serde`, `serde_json`, `thiserror`, `walkdir`; `git`, `cargo`, `trunk`, the
  local Postgres container and the agent CLI as subprocesses.
- Used by: the xtask binary's `platform` group, its `ticket run` and its mod wave driver.
- Rules: tier 9 of `tools/commands` (`cargo xtask verify crate-tiers`); a slice-run that exits 0
  without a usage object writes no receipt (`exit_zero_without_usage_fails_and_writes_no_file`);
  `land` refuses a slice without a green verdict at its head (`cmd_land_refuses_before_it_merges`);
  the schema step's build stamp covers xtask's whole build closure
  (`the_stamp_roots_are_the_whole_build_closure_of_xtask_and_nothing_else`).

## Related documentation

- [Factory waves](/documentation/runbooks/factory_waves/README.md) — the platform factory
  procedure `platform wave` automates.
- [Running a wave](/documentation/runbooks/factory_waves/running_a_wave.md) — `slice-worktree`,
  `slice-run` and `platform wave` step by step.
- [Taking a ticket from idea to shipped](/documentation/runbooks/ticket_run_pipeline.md) — how
  `ticket run` hands each ready ticket to `slice-run`.
