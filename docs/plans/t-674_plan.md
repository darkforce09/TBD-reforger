# T-674 — Plan

## Context

Program closure for the T-216 slot-identity gap. T-674.1 (engine emit, `flatten.rs`, order 4310) and T-674.2 (mod reader + validator 1.3, order 4311) carry the code; this ticket owns `TBD_MissionSlotStruct.c` jointly with T-674.2 and therefore packs after it (order 4312).

## Approach

1. After T-674.2 ships, confirm the struct fields, loader binding and validator entry landed (`rg -n 'callsign|leaderSlotId' apps/mod/tbd-framework/Scripts/Game/TBD/Backend/`).
2. Compile a fixture mission with every identity key through the API and run `cargo xtask mod compile`; the in-game spawn check goes on the human checklist.
3. No code unless T-674.2 reports `found_not_fixed` against `TBD_MissionSlotStruct.c`; then fix it here.

## Risks

- The salvage commit 113108a1 overlaps T-675; whichever family lands first must check subsumption before reusing it.

## Verification

- `cargo xtask mod compile` · `cargo xtask platform wave gate --slice T-674` · `cargo xtask ticket check`
