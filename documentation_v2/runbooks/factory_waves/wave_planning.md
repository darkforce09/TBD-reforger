**Status:** live

# Wave planning

How the factory decides which [tickets](/documentation_v2/glossary.md#ticket) run together: the
wave lock, the `owns` lists it is compiled from, the collision analysis, the width of a
[wave](/documentation_v2/glossary.md#wave) and how waves are numbered. Read it before promoting
tickets or before a wave looks wrong in `platform wave status`. The commands are read-only except
`wave repack`, which rewrites `.ai/tickets/wave.lock`.

## Prerequisites

- A clean checkout of `main`; `cargo xtask ticket check` exits 0 (see
  [Cold start and preflight](/documentation_v2/runbooks/factory_waves/cold_start_and_preflight.md)).
- Every ticket meant for a wave carries an `owns` list of real file paths, never a bare folder.

## How the plan is built

The plan is `.ai/tickets/wave.lock`. `cargo xtask wave repack` compiles it from the ticket files
and is its only writer; `ticket ship`, `ticket set-status`, `platform wave land` and
`platform wave wave --close` call the same compiler. The
[wave lock README](/tools_v2/ticket-engine/src/wave_lock/README.md) holds the full algorithm; the
facts an orchestrator needs are these.

| Fact | Rule |
|---|---|
| Dispatchable | a work ticket whose status is `queued`, `ready`, `running` or `review` and whose executor is `claude-code` or unset. An `idea` ticket never enters a wave; a `deferred` one waits until someone promotes it. |
| Wave 0 | the ledger of parked tickets (shipped, cancelled, deferred, other executors); `platform wave status` skips it. |
| Collision | two tickets collide when one owned path equals or contains another. Colliding tickets never share a wave. |
| Order | tickets pack by `order`, then id; a ticket packs after its `depends_on` targets; `pack_last` tickets trail in waves of their own. A dependency cycle refuses the compile. |
| Width | at most `TBD_MAX_CONCURRENT` tickets per wave when that variable is set; otherwise the width the committed lock records (`max_concurrent`); otherwise 8. |
| Numbering | open waves number from one past the highest of: the newest `wave N CLOSED` commit that no later `git revert` names, the highest close any marker claims, and every pending emptied label. A shallow clone refuses the derivation. |
| Emptied wave | an open wave whose tickets have all shipped freezes into an `[[emptied]]` entry with its label; `platform wave wave --close` closes the oldest one. |

The width is a limit on the orchestrator's attention, not on disk or CPU: every slice agent returns
a dense report that must be read, checked and acted on. Worktrees make concurrent edits safe, but
only disjoint `owns` lists keep the merges free of conflicts.

## Steps

1. See the next wave's largest file-disjoint set.

   ```bash
   cargo xtask slice-collisions
   ```

   Expected: `next wave is <n>. Max disjoint dispatch set (<k>, cap <width>):`, then each ticket
   with its title and an `owns:` line, then the tickets that blocked the rest. A warning that a
   dispatchable ticket is missing from the open waves means the lock is stale; run step 4.

2. With some tickets already running, ask which open tickets may join them.

   ```bash
   cargo xtask slice-collisions <running ticket id> <running ticket id>
   ```

   Expected: `already in flight (<n>):` with the named tickets, then `may join them (<k>, cap
   <width>):` with the open tickets whose `owns` collide with none of them. A name that is not an
   open ticket prints `warning: <id> is not an open ticket in the lock`.

3. Check one ticket's collisions before widening its `owns`.

   ```bash
   cargo xtask slice-collisions --check <ticket id>
   ```

   Expected: `<ticket id> owns: <paths>`, then `collides with:` and the colliding ids, or
   `nothing — safe to run alongside anything`. Exit 1 when the id is not an open ticket.

4. After editing a ticket's `owns`, `order`, `depends_on` or status by hand, recompile the lock.

   ```bash
   cargo xtask wave repack
   ```

   Expected: `wrote .ai/tickets/wave.lock: <summary>`; the lock changes only where the inputs
   changed, since the render is deterministic. Commit it with the ticket files that caused it.

5. Confirm the committed lock matches the tickets.

   ```bash
   cargo xtask wave check
   ```

   Expected: `wave.lock OK: <summary>`. Every `ERROR:` line names its fix, most often step 4.

## Ownership rules

- **`owns` is load-bearing.** It is the only thing that keeps two concurrent agents off one file.
  Narrow it to the files the ticket must edit; never widen a row to a bare folder to make it fit.
- **A missing file is fixed on the ticket, not in the brief.** When a ticket plainly must edit a
  file its `owns` omits, the orchestrator adds the file to the ticket, runs step 3 to confirm the
  wider list still collides with nothing in flight, then steps 4 and 5.
- **`owns` constrains what an agent is told, not what it does.** An agent that edits a file
  outside its list must say so in its report; the orchestrator then checks by hand that the
  sibling merges compose, rather than trusting a green gate that only saw the merged result.
- **The contended files are sequenced across waves.** The files many tickets touch (the large
  Mission Creator modules, the mission compiler, the event handlers) sit in different waves rather
  than being shared inside one; the collision rule does this by itself as long as `owns` is exact.

## Waves that shipped one ticket at a time

`ticket ship` repacks after every id unless told not to. A wave shipped that way dissolves one
ticket per repack, no repack ever sees the whole set shipped, no `[[emptied]]` entry forms and
`wave --close` has nothing to close. Two remedies:

- Prevention: ship a wave's tickets with `ticket ship <ticket id> --no-repack` and run one
  `cargo xtask wave repack` after the last one; that repack sees the full set shipped and freezes
  it. [Running a wave](/documentation_v2/runbooks/factory_waves/running_a_wave.md) step 12 gives
  the order.
- Repair, for a wave that already dissolved: freeze the shipped set at its own label.

  ```bash
  cargo xtask wave repack --reserve "<ticket id> <ticket id>"
  ```

  Expected: the lock gains a pending `[[emptied]]` entry holding exactly those ids, and the open
  waves renumber past it. `platform wave wave --close --tickets <ids>` refuses to write a label
  that is still an open wave in the lock and prints this repair with the ids filled in.

## Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| `slice-collisions` prints `warning: <id> is not an open ticket in the lock` | the id is shipped, parked, `idea`, or the lock is stale | check the ticket's status with `cargo xtask ticket show <ticket id>`; promote it or run `cargo xtask wave repack` |
| `slice-collisions` prints `(none — everything left collides with what is already running)` | every candidate shares a path with a running ticket | wait for a landing, or narrow an `owns` list that is wider than the work |
| `ticket check` or `wave check` prints `ERROR:` lines about the lock | the tickets changed after the last repack | `cargo xtask wave repack`, then commit the lock |
| `wave repack` refuses a cycle | two dispatchable tickets depend on each other | break the `depends_on` edge on one ticket |
| wave numbers jump or repeat | a marker was reverted, or a marker claims a higher wave | numbering follows the highest standing claim; read `git log --grep='CLOSED'` before closing by hand, and never type a marker yourself |
| the next wave is wider than the orchestrator can review | the lock records a width the orchestrator cannot read reports for | set `TBD_MAX_CONCURRENT` for the repack that sets the width, then commit the lock |

## Related

- [Wave lock command group](/tools_v2/xtask/src/commands/wave/README.md) — `wave repack`,
  `wave check` and `slice-collisions`.
- [Wave lock](/tools_v2/ticket-engine/src/wave_lock/README.md) — the compiler, the numbering and
  the emptied waves in full.
- [Ticket registry](/.ai/tickets/README.md) — the ticket fields the plan is compiled from.
- [Running a wave](/documentation_v2/runbooks/factory_waves/running_a_wave.md) — dispatching the
  set this page computes.
