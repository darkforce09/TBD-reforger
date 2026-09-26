**Status:** live

# Token estimate factor

The declared constant the `diff_loc` token estimator multiplies changed lines by, the one
measurement behind it, and the paths whose churn it leaves out. A shipped
[ticket](/documentation_v2/glossary/n_to_z.md#ticket) with no measured run receipt gets its token count
from this factor, so everyone who reads the ticketboard's token totals or recalibrates the factor
reads this first.

```text
TOKENS_PER_LOC = 150
```

## Where it lives

- Code: the constant `TOKENS_PER_LOC` in
  [`tools_v2/ticket-engine/src/metrics/estimates/model.rs`](/tools_v2/ticket-engine/src/metrics/estimates/README.md),
  the exclusion rule `is_excluded_path` in `git_changes.rs` beside it, and the excluded prefixes
  `NUMSTAT_EXCLUDED_PREFIXES` and `RETIRED_QUEUE_VIEW_PREFIX` in
  `tools_v2/ticket-engine/src/repository.rs`.
- Entry: `cargo xtask ticket stamp-sha <id> <sha>`, which writes `.ai/tickets/estimates/<id>.json`
  for a shipped ticket that has neither a run receipt nor an estimate; `cargo xtask ticket check`,
  which enforces the estimate rules.
- Related features: the design of the estimation ladder in the frozen
  [ticket schema v2 spec](/documentation_v2/tickets/specs/t917_ticket_schema_v2.md) (its
  "Estimation ladder" section); the [ticket engine documentation](/documentation_v2/tools_v2/ticket-engine/README.md).

## Behaviour

### The constant and its pin

The Rust constant is the value the estimator and `ticket check` use. This document quotes it
verbatim, and `factor_constant_is_pinned_in_the_doc`
(`tools_v2/ticket-engine/src/metrics/estimates/tests/estimate_provenance_tests.rs`) fails when the
two differ, or when this document stops naming the three excluded paths below. `ticket check`
refuses any `.ai/tickets/estimates/<id>.json` whose `factor` differs from the constant, with a
message that names this document: recalibrating is regenerating, never hand-editing a file.

### Derivation: one anchor, not a calibration

The only tokens-per-line pair on record is this repository's own ticketboard program (the
desktop viewer and its typed operations, about seven slices over one week):

- about 2,400,000 sub-agent output tokens, orchestration overhead excluded;
- about 15,000 to 20,000 lines changed across the program's commits.

factor = tokens / lines ≈ 2,400,000 / 16,000 = 150 tokens per changed line, rounded.

One program, one agent stack, one week: an anchor with unknown variance, not a fit. **Status:
declared pending calibration.** Recalibrate once run receipts accumulate under
`.ai/tickets/metrics/<id>/`; no receipt is committed yet. Every estimate file records the factor
it used and its inputs (`loc_changed` and `derived_from_shas`, or the cohort key), so recalibrating is
regenerating from recorded inputs, never an untraceable change.

### What counts as a changed line

`loc_changed` is insertions plus deletions, summed from `git log --numstat` over the ticket's
subject commits (the commits whose subject names the ticket's exact id), excluding:

- any path under `.ai/` (registry and ticket bookkeeping);
- `docs/TICKET_*.md` (the Markdown queue views that commits in history carry; no command writes
  one, but the walk over old commits still meets them, and only `.md` files match);
- any `Cargo.lock` (lockfile churn),

because bookkeeping noise is not implementation work. A binary file (numstat `-`) counts zero, and
a merge commit carries no numstat and counts zero. A ticket whose subject commits touch only
excluded paths falls through to the `cohort_median` source and is counted in that source's tally.

### Estimates stay apart from measurements

Estimates live at `.ai/tickets/estimates/<id>.json`, never inside `.ai/tickets/metrics/<id>/`,
and are never summed with measured receipts: `ticket check` refuses an estimate for a ticket that has a
receipt, and the ticketboard shows the two totals apart. Measured and estimated never mix.

## Data

- `.ai/tickets/estimates/<id>.json`: one file per shipped ticket without a receipt, validated
  against `.ai/tickets/estimates.schema.json`; its fields per source and the cohort ladder are in
  the [estimates README](/tools_v2/ticket-engine/src/metrics/estimates/README.md#how-it-works).
- `.ai/tickets/metrics/<id>/`: the measured run receipts that replace an estimate when they land.

## Design

The factor turns a count every shipped ticket has (its changed lines) into the unit the run
receipts measure (tokens), so the registry's token totals cover history that predates the
receipts. The exclusions keep bookkeeping, generated views and lockfiles from inflating that
count.

## Open work

- [T-1142 — Decide whether the ticketboard imports the ticket-engine logic it copies](/.ai/tickets/T-1142.toml)
  (idea, no plan): the ticketboard's copy of the estimate validation gains the factor rule, or
  gives way to the engine's own.
- [T-1140 — Remove ticket-engine duplicate id helpers and test-only public functions](/.ai/tickets/T-1140.toml)
  (idea, no plan): `run_estimates`, the whole-registry writing pass that only tests call, stops
  being public.

## Decisions

- One declared constant pinned in code and here, rather than a fitted model: with a single
  measurement, a fit would claim precision the data does not have.
- Estimates are regenerated from recorded inputs, never edited: the factor can change without
  losing how each number was made.
- Bookkeeping paths are excluded by prefix, and the queue-view prefix stays although no command
  writes it: a commit in history keeps the paths it was made with.
