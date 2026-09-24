# T-654 — Plan

## Context

FNF v4 replaced Eden layers with mode variants; TBD wants conditional inclusion (day/night, player-count bands, base-plus-delta) as a variant predicate on entity subtrees. Schema keys shipped in T-706; the 2026-08-02 attempt is at `salvage/t853-dropped/T-654`.

## Approach

1. Verify on main: `rg -n 'variant' apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_MissionLoader.c` is empty.
2. `TBD_MissionLoader.c`: read the selected variant at launch, evaluate each subtree's predicate, and drop non-matching subtrees before spawn; no predicate means always included.
3. Reuse salvage hunks that match; compile.

## Risks

- Predicate evaluation before spawn must also drop dependent refs (crew of an excluded vehicle) or the spawn logs dangling refs.

## Verification

- `cargo xtask mod compile` · `cargo xtask platform wave gate --slice T-654`
