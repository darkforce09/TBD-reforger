# T-678 — Plan

## Context

Four GRP attrs (combat mode, behaviour, formation, speed mode) have zero readers in the mod; T-677 opens the AI spawn gate this slice needs and packs first (order 4340).

## Approach

1. Verify on main: `rg -n 'combatMode|speedMode' apps/mod` is empty.
2. New `AI/TBD_GroupState.c`: after T-677's spawn path yields an AI group, apply the four attrs through the Reforger AI group API; absent attrs leave engine defaults.
3. Compile; record the exact API calls used in the report for the human checklist.

## Risks

- Formation/speed API names may differ by Reforger version; pin the version in the report.

## Verification

- `cargo xtask mod compile` · `cargo xtask platform wave gate --slice T-678`
