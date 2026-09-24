# T-921 — History reconstruction stream: plan

## Context

~330 shipped tickets carry thin or empty bodies with no quarantined wall to drain;
440 titles are id-shaped or over the 10-word cap; 53 live tickets lack a main_goal.
The debt pins measure all of it.

## Approach

Reviewed batches of ~20 shipped wall-less tickets in id order: reconstruct main_goal,
context, requirement, current_state, approach, verify (and titles where deficient)
strictly from the ticket's spec doc, subject commits, diffs and SHIPPED_HISTORY.md;
citations name the sources; thin evidence yields thin honest fields, never padding.
Pins shrink by the measured amount in the same commit.

## Risks

- Reconstruction drifting into invention — source-citing rule + operator review per
  batch is the mechanism; when evidence is absent the field stays short, not padded.
- Canonical-form deviations from hand edits — normalize through render_ticket_toml
  before install (the T-919 batch-1 pattern), roundtrip gate proves it.

## Verification

- Per batch: both pin ratchet tests green at the new values; corpus roundtrip N/N;
  check --strict OK; wave.lock byte-identical; porcelain shows only the batch files.
