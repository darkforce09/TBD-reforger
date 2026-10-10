**Status:** live

# Wave planning

How the factory decides which [tickets](/documentation/glossary/n_to_z.md#ticket) run together: the
wave plan, the `owns` lists it is compiled from, the collision analysis, the width of a
[wave](/documentation/glossary/n_to_z.md#wave) and how waves are numbered. Read it before promoting
tickets or before a wave looks wrong in `platform wave status`. The plan lives in the central
ticket manager, project `reforger`; the commands are read-only except `wave repack`, which
rewrites the stored plan.

## Prerequisites

- A clean checkout of `main`; `ttm --project reforger check` exits 0 (see
  [Cold start and preflight](/documentation/runbooks/factory_waves/cold_start_and_preflight.md)).
- Every ticket meant for a wave carries an `owns` list of real file paths, never a bare folder.

## How the plan is built

The plan is stored in the ticket manager and printed by `ttm --project reforger wave show`.
`ttm --project reforger wave repack` compiles it from the tickets; `ttm ship`, `ttm wave close`
and `platform wave land` (through `ttm wave repack`) run the same compiler. The ticket manager
holds the full algorithm; the facts an orchestrator needs are these.

| Fact | Rule |
|---|---|
| Dispatchable | a work ticket whose status is `queued`, `ready`, `running` or `review` and whose executor is `claude-code` or unset. An `idea` ticket never enters a wave; a `deferred` one waits until someone promotes it. |
| Wave 0 | the ledger of parked tickets (shipped, cancelled, deferred, other executors); `platform wave status` skips it. |
| Collision | two tickets collide when one owned path equals or contains another. Colliding tickets never share a wave. |
| Order | tickets pack by `order`, then ticket sequence; a ticket packs after its `depends_on` targets; `pack_last` tickets trail in waves of their own. A dependency cycle refuses the compile. |
| Width | at most the `--cap <n>` given to `wave repack` or `wave collisions`; otherwise the width the stored plan records (`max_concurrent`); otherwise 8. |
| Numbering | open waves number from one past the highest of: the newest close recorded with `ttm wave close` (the plan's wave base), the highest recorded close, and every pending emptied label. `--ledger git` reads the closes from the `wave N CLOSED` commits instead; preflight and the close check that the plan's wave base equals the newest standing marker in git. |
| Emptied wave | an open wave whose tickets have all shipped freezes into a pending emptied entry with its label; `platform wave wave --close` closes the oldest one. |

The width is a limit on the orchestrator's attention, not on disk or CPU: every slice agent returns
a dense report that must be read, checked and acted on. Worktrees make concurrent edits safe, but
only disjoint `owns` lists keep the merges free of conflicts.

## Steps

1. See the next wave's largest file-disjoint set.

   ```bash
   ttm --project reforger wave collisions
   ```

   Expected: `next wave is <n>. Max disjoint dispatch set (<k>, cap <width>):`, then each ticket
   with its title and an `owns:` line, then the tickets that blocked the rest. A warning that a
   dispatchable ticket is missing from the open waves means the plan is stale; run step 4.

2. With some tickets already running, ask which open tickets may join them.

   ```bash
   ttm --project reforger wave collisions <running ticket> <running ticket>
   ```

   Expected: `already in flight (<n>):` with the named tickets, then `may join them (<k>, cap
   <width>):` with the open tickets whose `owns` collide with none of them. A name that is not an
   open ticket prints `warning: <id> is not an open ticket in the lock`; tickets are named by slug
   or legacy number.

3. Check one ticket's collisions before widening its `owns`.

   ```bash
   ttm --project reforger wave collisions --check <ticket>
   ```

   Expected: `<ticket> owns: <paths>`, then `collides with:` and the colliding tickets, or
   `nothing — safe to run alongside anything`. It refuses when the ticket is not an open ticket.

4. After changing a ticket's `owns`, `order`, `depends_on` or status, recompile the plan.

   ```bash
   ttm --project reforger wave repack
   ```

   Expected: `wave plan: <summary>`; the plan changes only where the inputs changed, since the
   compile is deterministic. `--dry-run` prints the result without storing it. Nothing is
   committed: the plan lives in the ticket manager.

5. Confirm the stored plan matches the tickets.

   ```bash
   ttm --project reforger wave check
   ```

   Expected: `wave plan OK`. Every `ERROR:` line names a finding, most often fixed by step 4.

## Ownership rules

- **`owns` is load-bearing.** It is the only thing that keeps two concurrent agents off one file.
  Narrow it to the files the ticket must edit; never widen a row to a bare folder to make it fit.
- **The documents a change updates are files it edits.** Documentation ships in the same commit as
  the code it describes (`CLAUDE.md` law 10), so a slice agent updates the README.md of each folder
  its change reshapes and the feature docs whose behaviour it changes. List those documents in
  `owns` too, so that the collision rule keeps two slices off one README or feature doc.
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

`ttm ship` repacks after every ticket unless told not to. A wave shipped that way dissolves one
ticket per repack, no repack ever sees the whole set shipped, no emptied entry forms and
`wave --close` has nothing to close. Two remedies:

- Prevention: ship a wave's tickets with `ttm --project reforger ship <ticket> --no-repack` and
  run one `ttm --project reforger wave repack` after the last one; that repack sees the full set
  shipped and freezes it. [Running a wave](/documentation/runbooks/factory_waves/running_a_wave.md)
  step 12 gives the order.
- Repair, for a wave that already dissolved: freeze the shipped set at its own label.

  ```bash
  ttm --project reforger wave repack --reserve <ticket> <ticket>
  ```

  Expected: the plan gains a pending emptied entry holding exactly those tickets, and the open
  waves renumber past it. `platform wave wave --close --tickets <ids>` refuses to write a label
  that is still an open wave in the plan and prints this repair with the tickets filled in.

## Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| `wave collisions` prints `warning: <id> is not an open ticket in the lock` | the ticket is shipped, parked, `idea`, or the plan is stale | check the ticket's status with `ttm --project reforger show <ticket>`; promote it or run `ttm --project reforger wave repack` |
| `wave collisions` prints `(none — everything left collides with what is already running)` | every candidate shares a path with a running ticket | wait for a landing, or narrow an `owns` list that is wider than the work |
| `wave check` prints `ERROR:` lines about the plan | the tickets changed after the last repack | `ttm --project reforger wave repack` |
| `wave repack` refuses a cycle | two dispatchable tickets depend on each other | break the `depends_on` edge on one ticket |
| wave numbers jump or repeat | a close was recorded under a higher label, or a marker in git and the recorded closes disagree | numbering follows the highest recorded close; read `git log --grep='CLOSED'` and `ttm --project reforger wave history <n>` before closing by hand, and never type a marker yourself |
| the next wave is wider than the orchestrator can review | the plan records a width the orchestrator cannot read reports for | `ttm --project reforger wave repack --cap <n>` sets the width the plan records |

## Related

- [Ticket manager client](/tools/foundation/ticket_manager_client/README.md) — how the wave
  drivers read the plan and call `wave repack`, `wave check` and `wave close`.
- [Running a wave](/documentation/runbooks/factory_waves/running_a_wave.md) — dispatching the
  set this page computes.
