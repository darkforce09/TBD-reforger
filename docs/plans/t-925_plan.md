# T-925 — Plan

## Context

T-923 automated the close ceremony, but its target is unreachable: `current_wave` names
the first wave holding an unshipped ticket (which always refuses), and a fully-shipped
wave's label dissolves into wave 0 at the ship-hook repack before close can see it.
Operator decision 2026-08-16: close targets the most recently emptied label. Wave 133
cannot close until this lands.

## Approach

Lock gains a repack-owned `[[emptied]]` section (label `n` + the frozen ticket set):
when repack observes that a previous-lock open wave's tickets are now all shipped, it
appends that label and set — the same carry-forward class as the wave-0 baseline. Open
waves number past `max(wave_base, highest pending emptied)` so labels never collide.
`cmd_wave_close` retargets from `current_wave` to the OLDEST pending emptied label
(the marker oracle only accepts base+1, so a queue drains in order; with one pending —
the normal case — that is exactly "the most recently emptied"). All existing
validations (all-shipped, verifier-at-HEAD, gate) run against the frozen set; the T-923
ceremony then commits marker `n`, repack drops the entry (`n` ≤ new base) and the lock
refresh rides the close. No pending entry ⇒ close refuses honestly. `current_wave`
itself is untouched (dispatch semantics stay).

## Risks

Emptied entries cannot be recomputed from scratch (they derive from the previous lock,
like the baseline) — `wave check` validates invariants instead: labels sorted, above
`wave_base`, disjoint from open labels, sets nonempty and all-shipped in the current
tree. Partial ships must not record (only a fully-shipped previous wave empties).
Multiple pending labels must number-collide with nothing — pinned by the numbering rule
above and tests.

## Verification

Fabricated-repo tests: ship a full wave → repack records `[[emptied]]` with the frozen
set and renumbers open waves past it; close ceremony closes that label end-to-end
(marker accepted, base advances, entry dropped, check green); partial ship records
nothing; two pending labels drain oldest-first and the second close works; no-pending
close refuses. `cargo test -p xtask` green; `ticket check --strict` prints `check OK`;
`platform wave gate --slice T-925` prints `SLICE GATE: PASS`.
