# Wave lock command group

The `cargo xtask wave` group and the top-level `cargo xtask slice-collisions`: the commands that
compile, check and read `.ai/tickets/wave.lock`, the [wave](/documentation/glossary/n_to_z.md#wave)
plan of the open [tickets](/documentation/glossary/n_to_z.md#ticket). They own the plan only; the
drivers that run a wave are `cargo xtask platform wave` and `cargo xtask mod wave`.

## Contents

```text
tools/xtask/src/commands/wave/
├── cli.rs       the `WaveLockCmd` clap enum: `repack` with `--reserve`, and `check`
├── dispatch.rs  finds the checkout root and calls the ticket_wave_lock command
└── mod.rs       the module tree; re-exports `cmd_check` and `cmd_repack`, and runs `slice-collisions`
```

## How it works

Every command delegates to `ticket_wave_lock`. `repack` compiles the lock from the ticket
files: it packs the dispatchable tickets, in `order` then id order, into waves whose tickets own
no common files, honouring each ticket's `depends_on` and `pack_last`, with at most
`TBD_MAX_CONCURRENT` tickets per wave (when unset, the width the committed lock records, else 8);
it is the only writer of the lock. `check` compiles the same plan and compares
it with the committed lock. `slice-collisions` reads the lock and the ticket files and answers
which open tickets can run together: two tickets collide when one owned path equals or contains
another.

`slice-collisions` is declared in `tools/xtask/src/cli/mod.rs` with raw trailing arguments,
and `mod.rs` passes them to `ticket_wave_lock::collisions::run` with the root from
`repository_layout::prelude::find_repository_root`.

## Commands

Run each from the repository root. An error prints `xtask: <cause>` and exits 1; a clap usage
error exits 2.

### wave repack

- Synopsis: `wave repack [--reserve <IDS>]`
- Does: writes `.ai/tickets/wave.lock` from the ticket files and prints its summary.
  `--reserve "<id> <id>"` records those shipped ids as a pending emptied wave, for a wave whose
  tickets shipped one by one and left no emptied entry for `platform wave wave --close` to close.
- Exit codes: 0 written; 1 the tickets could not be packed.
- Example: `cargo xtask wave repack`

### wave check

- Synopsis: `wave check`
- Does: recompiles the plan from the tickets and prints `wave.lock OK: <summary>`, or one
  `ERROR:` line per difference, each naming its fix. A missing lock is an error, never an empty
  plan. `cargo xtask ticket check` runs the same comparison.
- Exit codes: 0 the lock matches; 1 drift or a missing lock.
- Example: `cargo xtask wave check`

### slice-collisions

- Synopsis: `slice-collisions [<ID>...]`; `slice-collisions --check <ID>`;
  `slice-collisions --repack`
- Does: with no id, prints the next wave's largest file-disjoint set of open tickets; with ids
  already running, prints the open tickets that may join them; `--check <ID>` prints what one
  open ticket collides with; `--repack` is `wave repack`. It warns when a dispatchable ticket is
  missing from the lock's open waves.
- Exit codes: 0 answered; 1 a missing lock, or `--check` without an open ticket id.
- Example: `cargo xtask slice-collisions`

## Boundaries

- Depends on: `ticket_wave_lock` (`cmd_repack`, `cmd_check`, `collisions`); `ticket_model`
  (`TicketId`);
  `tool_test_support`; `git` for `slice-collisions`.
- Used by:
  - `tools/xtask/src/cli/dispatch.rs`, which mounts both commands;
  - `cargo xtask ticket ship` and `ticket set-status`, which repack through `ticket_wave_lock`, and
    `ticket check`, which runs the lock check;
  - the platform preflight and wave gate, which run `wave check`; `platform wave prep`, which
    runs `slice-collisions` (`COLLIDE` in
    `tools/commands/platform_execution/src/wave_execution/mod.rs`); wave landing and closing,
    which repack;
  - the [orchestrator](/documentation/glossary/n_to_z.md#orchestrator) and people planning a wave.
- Rules: the lock has one writer, `wave repack`, and `slice-collisions --repack` calls that same
  writer; the compilation and packing stay in `ticket_wave_lock`, and `mod.rs` delegates to it.

## Related documentation

- [Factory waves](/documentation/runbooks/factory_waves/README.md) — how a wave is planned,
  run, landed and closed.
- [Wave planning](/documentation/runbooks/factory_waves/wave_planning.md) — reading the
  collision analysis, the width rule and repairing a wave that shipped one ticket at a time.
- [Ticket registry](/.ai/tickets/README.md) — the ticket fields (`owns`, `depends_on`,
  `pack_last`) the plan is compiled from.
