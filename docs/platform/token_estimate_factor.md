# T-917.5 — Token estimate factor

The declared constant the `diff_loc` token estimator multiplies by
(`cargo xtask ticket estimate-tokens`; spec:
[`t917_ticket_schema_v2.md`](t917_ticket_schema_v2.md) §Estimation ladder):

```
TOKENS_PER_LOC = 150
```

Authority pattern: the Rust constant `xtask/src/estimate_tokens.rs::TOKENS_PER_LOC`
is the value the generator and `ticket check` use; a test asserts this document
quotes it verbatim, and check refuses any `.ai/tickets/estimates/<id>.json` whose
`factor` differs. Doc and code cannot drift silently.

## Derivation — a single measured-once anchor, not a calibration

The only tokens-per-LOC pair on record is this repo's own T-915/T-916 program
(ticketboard GUI + typed ops, ~7 slices, shipped 2026-08-14):

- ~2,400,000 subagent output-tokens consumed (orchestration overhead excluded);
- ~15,000–20,000 LOC changed across the program's commits.

factor = tokens / LOC ≈ 2,400,000 / 16,000 = 150 tokens per LOC changed, rounded.

One program, one agent stack, one week — an anchor with unknown variance, not a
fit. **Status: declared pending calibration.** Recalibrate when run receipts
accumulate under `.ai/tickets/metrics/`; every estimate file records the factor
it used plus its inputs (`loc_changed` + `derived_from_shas`, or the cohort key),
so recalibration is regeneration from recorded inputs — never untraceable
mutation.

## What counts as a changed LOC

`loc_changed` = insertions + deletions summed from `git log --numstat` over the
ticket's exact-id boundary-matched subject commits (the T-917.4 miner's lists),
EXCLUDING:

- any path under `.ai/` (registry/ticket bookkeeping),
- `docs/TICKET_*.md` (generated sync views),
- any `Cargo.lock` (lockfile churn),

because bookkeeping noise is not implementation work. Binary files (numstat `-`)
count zero; merge commits carry no numstat and count zero. A ticket whose subject
commits touch ONLY excluded paths falls through to the `cohort_median` method and
is counted in that method's tally.

## Boundary

Estimates live at `.ai/tickets/estimates/<id>.json` — never inside
`.ai/tickets/metrics/` — and are never summed with measured receipts
(spec §estimates-outside-metrics; the T-913 honesty rule).
