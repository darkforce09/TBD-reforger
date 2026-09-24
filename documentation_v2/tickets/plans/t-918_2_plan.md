# T-918.2 — Provenance rendering: measured vs estimated, never summed

## Context

The registry now carries measured receipts AND marked estimates (stamps via
git_subject/id_interpolation, tokens via diff_loc/cohort_median). The board must
never let the operator mistake maths for measurement: every estimated value
renders visually distinct with its method one hover away, and no UI element sums
across the receipt/estimate boundary.

## Approach

In `apps/ticketboard`: stamps carrying an `estimated[]` entry get the estimate
glyph + per-source tooltip (git_subject / id_interpolation from `estimate_note`);
token figures from `.ai/tickets/estimates/<id>.json` render in their own
estimated column with source + factor + inputs in the tooltip; the metrics
dashboard keeps measured and estimated as separate columns backed by the
structurally separate trees (`metrics/` walkers vs estimate loader) — the
negative assertion (no receipt+estimate arithmetic anywhere) lands as a test.

## Risks

The data layer already forbids mixed sums structurally; the risk is purely
presentational — a column, total, or sort that quietly folds the two populations
together, or a tooltip that drops the method and launders an estimate as fact.
Read-only discipline: `git status --porcelain` must stay unchanged after a full
session (the board never writes the registry outside the xtask verbs).

## Verification

A ticket with estimated stamps shows glyph + source tooltip while a measured one
shows neither; dashboard has separate measured/estimated columns with no
cross-summing cell (negative assertion in tests + screenshot); estimate tooltips
name source, inputs and factor; `git status --porcelain` unchanged after a session
(spec: `docs/platform/t917_ticket_schema_v2.md`).
