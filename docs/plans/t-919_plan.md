# T-919 — Wall triage drain: plan

## Context

T-917.3 quarantined 694 over-cap summaries verbatim into `migration_legacy[]` behind a
shrink-only pin. The prose is preserved but uncategorised; the ten typed body fields
exist empty on those tickets. Pass 2 is semantic work — deliberately kept out of the
mechanical migration.

## Approach

AI batches of 20–30 tickets per commit: each wall decomposed from `migration_legacy[]`
into context / requirement / current_state / approach / verify / citations one-liners
(≤30 words per line, citations ≤8), `migration_legacy` deleted in the same edit,
`MIGRATION_LEGACY_PIN` shrunk by exactly the batch size in the same commit. Walls carry
recognisable FIX:/ACCEPTANCE:/Repro: idioms; content is reorganised, never invented —
lines that fit no field verbatim land in `notes`. Batches are operator-reviewed
commits; the stream runs until the pin hits zero.

## Risks

- Semantic misfiling (requirement vs context vs approach) — the anti-blend definitions
  are the sorting authority; when genuinely ambiguous, `notes` wins over a wrong bucket.
- Losing content in the move — per-ticket rule: every non-whitespace token of the wall
  must appear in some field or `notes`; spot-checked per batch.

## Verification

- Per batch: pin shrinks by exactly the batch size; `cargo test -p tbd-tickets` green
  (pin test enforces both directions); remaining quarantined files still pass the
  reversibility join-proof; `cargo xtask ticket check --strict` prints `check OK`
  (word caps bind on the new lines).
- wave.lock byte-identical per batch (body fields are not lock inputs).
