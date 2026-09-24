# T-946.12 — reserve a close label for a wave that dissolved id by id

## The defect

`wave --close` needs a label, and the only label its oracle accepts is `ledger_floor + 1` — one past
the newest `wave N CLOSED` marker. In the healthy path that label is already reserved: when one
repack sees a whole open wave landed, `carry_emptied` freezes the wave's label and its exact ticket
set into a pending `[[emptied]]` entry, and `assemble` numbers every open wave past it.

`ticket ship` repacks after **every id**. A wave shipped one ticket at a time therefore never
presents that picture:

| step | what the repack sees | what freezes |
|---|---|---|
| ship id 1 | wave N still holds two live ids | nothing |
| ship id 2 | wave N was re-packed after step 1 and now holds the *next* batch | nothing |
| ship id 3 | wave N's label belongs to unstarted tickets | nothing |

The wave is gated and verified — the gate covers the whole span since the previous marker — but the
lock has lost the membership, and the label the ceremony will ask for now belongs to a live open
wave.

Closing anyway is not cosmetic. Oracle 2 reads the plan at the marker's **parent**; it would find
wave N assigned to unshipped tickets and refuse every later gate with "A wave with open tickets did
not close, so this commit is not a wave boundary". That is what cost wave 236 its marker
(`35328a7b1`, disavowed by `36f462d3f`). The collision guard added at `a596f89e1` refuses the close
instead, which is correct and leaves the wave unclosable.

Measured 2026-09-06 on wave 240 — T-935.6, T-935.9, T-674.1 — where the batch broke mid-way on
T-674.1's missing `created_at` and the per-id path finished it.

## The repair

`cargo xtask wave repack --reserve "<ids>"` freezes an operator-named set of **shipped** ids as a
pending close target at `ledger_floor + 1 + <carried entries>`. The repack stays the only writer of
the lock.

The vouching is for MEMBERSHIP, never for status: every named id is checked shipped-or-cancelled
against the tree, the same meaning `wave --close` re-checks a step later, so a reservation can never
record a target the ceremony would refuse on status grounds. An unknown id, a live id, a duplicate,
or an id already pending is refused and nothing is written.

The entry is self-sustaining. It sits above `wave_base`, so the derived carry keeps it and
`check_as_errors`' re-derivation reproduces it — which matters because `ticket ship` runs a repack on
every id. The post-close repack drops it the moment its marker lands, like any other pending entry.

An empty reservation compiles the lock byte-identically to a plain repack.

## What this does not do

It does not stop the dissolution. `ticket ship --no-repack` plus one repack at the end is still the
batch path that keeps the entries from going missing; this is the repair for waves that already lost
one. The derived rule that would need no vouching at all — freeze the set of tickets whose
`shipped_at` lands in `git rev-list <newest marker>..HEAD` — is a larger change to every repack and
is not attempted here.
