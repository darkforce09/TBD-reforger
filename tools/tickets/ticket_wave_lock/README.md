# Wave lock

The `ticket_wave_lock` crate: the compiler, reader, writer and checker of `.ai/tickets/wave.lock`,
the [wave](/documentation/glossary/n_to_z.md#wave) plan that says which dispatchable
[tickets](/documentation/glossary/n_to_z.md#ticket) run together because they own no common
files. It also reads the `wave N CLOSED` ledger in git history and prints the slice collision
report. `ticket_registry` repacks and checks the lock through it, and xtask mounts it as the
`wave` command group, `slice-collisions` and the base of the platform and mod wave drivers.

## Contents

```text
tools/tickets/ticket_wave_lock/
├── Cargo.toml  the `ticket_wave_lock` library package: `ticket_model`, `repository_layout`, `process_runner`, layout tier 3
└── src/        the lock model, compiler, packing, persistence, check, close-marker history and collision report
```

## How it works

Every function takes the checkout root. The compiler reads every ticket file through
`ticket_model`'s typed corpus, the previous lock and the `wave N CLOSED` commits, and produces a
`WaveLock`; the repack renders it to the lock file, the lock's only writer.

```text
.ai/tickets/T-*.toml ──load_views──► TicketView per ticket
previous wave.lock   ──load────────► wave 0 baseline, recorded width, pending emptied waves
git log              ──history─────► wave_base (newest standing close), floor (highest claim)
                                          │
                                          ▼
                     compile ──► WaveLock ──render/write──► .ai/tickets/wave.lock
                                          │
                     check_as_errors ◄────┘ (the committed lock against a fresh derivation)
```

The lock holds four parts: wave 0, the ledger of parked tickets; the open waves, groups of
dispatchable tickets whose `owns` paths do not overlap, numbered past the close-marker ledger;
the `[[emptied]]` waves whose tickets have all shipped and that wait for their close commit; and
the corpus-wide `owns`, `depends_on` and `pack_last` snapshots. `src/README.md` describes each
module and the numbering rules.

## Getting started

Run from the repository root:

```bash
cargo test -p ticket_wave_lock   # packing, numbering, emptied waves, the check and the collision facts
cargo xtask wave repack          # recompile and write .ai/tickets/wave.lock
cargo xtask wave check           # compare the committed lock with the ticket files and git history
cargo xtask slice-collisions     # the largest file-disjoint set of open tickets
```

The tests build scratch git repositories under the system temporary folder and need `git` on
`PATH`; `facts_come_from_ticket_files` also reads the live `.ai/tickets/` tree of the checkout.

## Configuration

- `TBD_MAX_CONCURRENT` (optional): the most tickets one open wave may hold. Unset or empty, a
  repack keeps the width the previous lock records, and a tree with no lock uses 8
  (`src/compiler.rs`, `src/ticket_views.rs`).
- Git history: the `wave N CLOSED` commit subjects reachable from `HEAD` number the open waves; a
  shallow clone refuses the repack and the check (`src/history.rs`).
- No feature flags.

## Public surface

- At the crate root: `WaveLock`, `LockWave`, `TicketView`, `LOCK_VERSION`, `lock_path`; the
  compiler `compile`, `compile_with_cap` and `compile_reserving`; the file calls `load`,
  `parse`, `render`, `write`, `missing_lock_error`, `repack_quiet`, `repack_reserving` and
  `cmd_repack`; `load_views`, `max_concurrent` and `collides`; the check `check_as_errors` and
  `cmd_check`; `Error` and `Result`.
- `history`: the `wave N CLOSED` subject rules, `newest_close_base`, `max_close_claim`, and the
  `git_in` and `git_in_lossy` readers.
- `archived_wave_plans`: `tickets_at`, the plans that preceded the lock read at past revisions,
  and the one-shot build of a first lock from them.
- `collisions`: `run`, the `slice-collisions` command.
- `prelude`: `WaveLock`, `LockWave`, `TicketView`, `load`, `load_views` and `repack_quiet`.

## Boundaries

- Depends on: `ticket_model` (the typed corpus, `TicketId`, `StatusName`, the id order key, the
  archived plan paths); `repository_layout` (`WAVE_LOCK`); `process_runner` (the `git` runs);
  `serde`, `toml` and `thiserror`; the `git` program.
- Used by:
  - `ticket_registry`: `ticket ship` and `ticket set-status` repack through `repack_quiet`,
    `ticket check` folds in `check_as_errors` and `missing_lock_error`, and the shipping status
    reads `load_views`;
  - xtask: `wave repack`, `wave check` and `slice-collisions` (`tools/xtask/src/commands/wave/`),
    the platform wave driver (`tools/commands/platform_execution/src/wave_execution/`: the `history`
    rules, `load`, `load_views`, `repack_quiet`, `archived_wave_plans::tickets_at`) and its
    preflight, and the mod wave driver
    (`tools/commands/mod_operations/src/wave_execution/execution.rs`).
  - The ticketboard does not link it: its wave lanes parse the lock themselves from
    `repository_layout::WAVE_LOCK` (`tools/tickets/ticketboard_model/src/wave_plan/services/lock_file.rs`).
- Rules: tier 3 of `tools/tickets`, depending on no ticket crate above `ticket_model`
  (`tools/checks/repository_checks/src/tests/tooling_dependency_boundaries.rs`; `cargo xtask verify crate-tiers`);
  the repack is the only writer and renders byte for byte the same from the same inputs
  (`compile_render_is_deterministic_and_roundtrips`); a missing lock is a refusal
  (`missing_lock_is_a_did_not_run_refusal`); a `TicketId` serialises as its bare string, so the
  lock text never carries the type.

## Related documentation

- [Wave lock source](/tools/tickets/ticket_wave_lock/src/README.md) — the modules, the numbering
  rules and the emptied waves.
- [Wave command group](/tools/xtask/src/commands/wave/README.md) — `wave repack`, `wave check`
  and `slice-collisions`.
- [Factory waves](/documentation/runbooks/factory_waves/README.md) — how a wave is planned, run,
  landed and closed.
- [Ticket crates](/tools/tickets/README.md) — the ticket crates and their tiers.
