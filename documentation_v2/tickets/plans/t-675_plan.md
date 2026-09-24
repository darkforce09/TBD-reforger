# T-675 — Plan

## Context

Program closure for the T-216 ledger sixth row (vehicle roster). T-675.1 emits `vehicles[]` (order 4320, after T-674.1 on `flatten.rs`); T-675.2 reads it and seats crew (order 4321, after T-674.2 on the loader/spawn files). This ticket shares `TBD_SpawnManager.c` with T-675.2 and packs last (order 4322).

## Approach

1. After T-675.2 ships, compile a fixture mission with an authored roster through the API and run `cargo xtask mod compile`.
2. Confirm the ledger comment in `flatten.rs` reads closed for the sixth row (via T-675.1); in-game seating goes on the human checklist.
3. No code unless T-675.2 reports `found_not_fixed` against `TBD_SpawnManager.c`.

## Risks

- Salvage commit 113108a1 is shared with T-674; check subsumption before reuse.

## Verification

- `cargo xtask mod compile` · `cargo xtask platform wave gate --slice T-675` · `cargo xtask ticket check`
