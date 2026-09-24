# T-917.6 — Ship gate ON + stamp-sha verb + plan ready-gate

## Context

S.2–S.5 made the tree gate-satisfiable (v2 types, quarantine, stamp backfill,
token estimates); the T-917 capstone turns the hard requirement ON. The gate must
land WITH `stamp-sha` or every future ship wedges between the ship edit and the
SHA stamp (the SHA does not exist until the operator commits), and with the plan
ready-gate so nothing goes ready without its own plan document.

## Approach

Ship-gate rule in `xtask/src/check.rs` composing with the T-917.4 coherence and
T-917.5 estimates rules (no double-reporting): shipped ⇒ created_at +
completed_at + SHA-shaped shipped_at (or marked-absent with note) + receipt XOR
estimate. `ops::ship` refuses created_at-less tickets pre-write; new typed
`ops::stamp_sha` + `ticket stamp-sha` verb writes shipped_at and auto-generates
the diff_loc estimate (cohort fallback) reusing the T-917.5 machinery. Plan
ready-gate in check + `ops::mark_ready` (default `docs/plans/<id>_plan.md`,
file-on-disk refusal). Fix the 4 branch-shaped shipped_at strays honestly
(delete + re-mine); write plans for the live ready set; strict prints honesty
counters.

## Risks

The ship→commit→stamp window is transiently gate-red by design — mutator
preflights must not deadlock it (`stamp-sha` skips the full-check preflight,
documented); a wrongly-scoped gate arm would red 1000+ shipped tickets, so every
arm is measured against the live tree before landing; wave.lock must stay
byte-identical (stamps/markers/plans are not lock inputs).

## Verification

`cargo test -p xtask -p tbd-tickets` (gate red/green per arm, stamp-sha scratch
cycle, ship refusal corpus-untouched, plan-gate red/green, counter math);
`cargo xtask ticket check --strict` prints the two honesty counter lines + check
OK on the live tree; `git diff --stat .ai/tickets/wave.lock` empty; the corpus
roundtrip test stays N/N byte-identical
(spec: `docs/platform/t917_ticket_schema_v2.md`).
