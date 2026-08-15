# T-922 — Full-body reconstruction: plan

## Context

The T-919/T-921 drains zeroed the wall, title and goal pins, but their selection arms
(title/goal/context debt) left 962 shipped work tickets missing at least one of the
six ready-tier body fields. The recorded corpus-wide flip of the body rule on shipped
waits on this debt.

## Approach

Reviewed batches of ~20 shipped body-debt tickets in id order (selection: any of
context/requirement/current_state/approach/verify/acceptance empty, instrument
`body_is_debt`). Fill ONLY the missing fields — existing field content is never
overwritten — reconstructed strictly from the ticket's spec, subject commits, diffs
and SHIPPED_HISTORY.md, sources cited in citations. Thin evidence yields thin honest
fields. BODY_DEBT_PIN shrinks by the measured batch amount in the same commit. The
zeroing commit widens `check_ready_tier_body` to shipped, deletes the pin + ratchet
test, and proves strict green.

## Risks

- Padding under pressure to fill six fields on thin tickets — the honesty rule is
  per-field: a genuinely unproven verify stays empty ONLY if the rule permits; where
  evidence truly yields nothing, a one-line honest statement of what the ticket did
  is the floor, never invented specifics.
- Overwriting existing content — clobber guards per field, the T-921 batch-1 pattern.

## Verification

- Per batch: `body_debt_ratchet_pin` green at the new pin; roundtrip N/N; strict OK;
  wave.lock byte-identical; porcelain shows only the batch files + pin const.
- End state: pin deleted, shipped-wide rule green on the live tree.
