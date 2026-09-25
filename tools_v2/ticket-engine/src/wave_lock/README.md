# Wave lock

The compiler, reader and checker of `.ai/tickets/wave.lock`, the [wave](/documentation_v2/glossary.md#wave)
plan of the open [tickets](/documentation_v2/glossary.md#ticket): which dispatchable tickets can run
together because they own no common files, numbered on from the wave-close commits in git
history. The lock is compiled from the ticket files alone and has one writer.

## Contents

```text
tools_v2/ticket-engine/src/wave_lock/
├── archived_wave_plans.rs  the archived wave plans, read at past revisions
├── collisions.rs           `run`, the `slice-collisions` command: file-disjoint sets of open tickets
├── compiler.rs             `compile` and its variants: the width, the ledger base and floor, numbering
├── history.rs              the `wave N CLOSED` commit rules: the newest close, the highest claim, disavowals
├── mod.rs                  the module tree; re-exports the model, compiler, persistence and check
├── model.rs                `WaveLock`, `LockWave` and `TicketView`, with `dispatchable` and `parked`
├── packing.rs              the greedy that packs open waves, and the reserved emptied wave
├── parking.rs              wave 0, the corpus-wide snapshots, and the carry of emptied waves
├── persistence.rs          render, parse, load, write, and the repacks (`cmd_repack` and others)
├── tests/                  unit tests for packing, numbering, emptied waves and the check
├── ticket_views.rs         `load_views` over every ticket file, `max_concurrent` and `collides`
└── verification.rs         `check_as_errors` and `cmd_check`: the lock compared with a fresh compile
```

## How it works

```text
Corpus::load ──load_views──► TicketView per ticket (children included)
                                  │
previous lock ──► wave 0 baseline, width, pending emptied waves
                                  │
git history ──history──► wave_base (newest close) and floor (highest claim)
                                  ▼
                   compile ──► WaveLock ──render──► .ai/tickets/wave.lock
```

- A ticket is dispatchable when it is work, live (`queued`, `ready`, `running`, `review`) and
  its executor is `claude-code` or unset; it is parked when it is shipped, cancelled or deferred,
  or has another executor.
- Wave 0 is a ledger, not a schedule: the previous lock's wave 0 plus every parked ticket, minus
  anything dispatchable again, in id order.
- Open waves: the greedy takes the dispatchable tickets by `order`, then id, and fills each wave
  with tickets whose `owns` paths do not collide (equal, or one containing the other) and whose
  `depends_on` targets are already packed or done, up to the cap; `pack_last` tickets trail in
  single-ticket waves. A dependency cycle among dispatchable tickets refuses the compile.
- Width: `TBD_MAX_CONCURRENT` when set, else the width the previous lock records, else 8.
- Numbering: `wave_base` is the newest `wave N CLOSED` commit subject reachable from `HEAD` that
  no later commit reverts; open waves number from one past the highest of the base, the highest
  claimed close and every pending emptied label. A shallow clone refuses the derivation.
- Emptied waves: a previous open wave whose tickets are now all shipped or cancelled freezes into
  an `[[emptied]]` entry with its label, and stays until its close commit lands.
  `cargo xtask wave repack --reserve <ids>` records such an entry by hand for a wave that shipped
  one ticket at a time.
- The render is deterministic: a header comment, `version`, `max_concurrent`, `wave_base`,
  `pack_last`, `[[waves]]`, `[[emptied]]` when there are any, then the `owns` and `depends_on`
  snapshots. A missing lock is always a refusal, never an empty plan.
- `check_as_errors` compares the committed lock with the ticket files: version, base, wave 0,
  snapshots, the cap, `pack_last` placement, duplicate or missing tickets, collisions in one
  wave, and the emptied entries; each error names its fix, most of them a repack.
- `archived_wave_plans` reads the tab-separated plans that preceded the lock, only from past
  revisions through `git show`, and a repack on a tree that still holds them builds the first
  lock from them and deletes them.

## Boundaries

- Depends on: the typed corpus (`crate::Corpus::load`) for the ticket views; `crate::StatusName`
  and `crate::store`'s id order; `git` (`log`, `show`, `rev-parse`); `crate::repository`
  (`WAVE_LOCK`, and `documentation`'s `ARCHIVED_WAVE_PLANS`).
- Used by:
  - `cargo xtask wave repack`, `wave check` and `slice-collisions`
    (`tools_v2/xtask/src/commands/wave/`);
  - `ticket ship` and `ticket set-status`, which repack through `repack_quiet`, and
    `ticket check`, which runs `check_as_errors` (both in `tools_v2/ticket-engine/src/`);
  - the platform wave driver in `tools_v2/xtask/src/commands/platform/wave_execution/`
    (the base and close-number rules of `history`, `load`, `repack_quiet`,
    `archived_wave_plans::tickets_at`) and its preflight, and the mod wave driver in
    `tools_v2/xtask/src/commands/mod_ops/wave_execution/execution.rs`;
  - `crate::registry::shipping_status`, which reads statuses through `load_views`. The
    ticketboard's wave lanes parse the lock themselves from `crate::repository::WAVE_LOCK`
    (`apps/ticketboard/src/wave_plan/services/lock_file.rs`).
- Rules:
  - the repack is the only writer, and a compile renders byte for byte the same from the same
    inputs (`compile_render_is_deterministic_and_roundtrips`);
  - overlapping `owns` never share a wave (`overlapping_owns_never_share_a_wave`), and a
    dependent packs after its dependency (`dependent_packs_strictly_after_unshipped_dependency`);
  - an incidental repack keeps the lock's recorded width (`an_incidental_repack_keeps_the_locks_own_width`);
  - numbering follows the highest close claim (`numbering_seats_on_the_highest_claim_not_the_newest_marker`),
    and a shallow clone refuses (`shallow_clone_refuses_base_derivation`);
  - `slice-collisions` reads its facts from the ticket files, never from a table in code
    (`facts_come_from_ticket_files`).

## Related documentation

- [Wave lock command group](/tools_v2/xtask/src/commands/wave/README.md) — `wave repack`,
  `wave check` and `slice-collisions`.
- [Factory waves](/documentation_v2/runbooks/factory_waves/README.md) — how a wave is planned,
  run, landed and closed.
