# T-823 — Plan

## Context

The OBJ readout equals `slot_count` (`editor/state/history.rs:350` area; formerly mission_history :289/:532) so drag-placing a vehicle leaves it unchanged while the vehicle really places. The WEST/EAST/IND/TOTAL chips count slots on purpose and must not drift.

## Approach

1. Decide: count slots + vehicles, or rename to SLOTS; pin the decision in the ticket.
2. `history.rs`: implement the chosen readout; chips untouched.
3. Test: place a vehicle → readout reflects the decision.

## Risks

- None material; scope is one readout.

## Verification

- `cargo xtask mk leptos-gates` · `cargo xtask platform wave gate --slice T-823`
