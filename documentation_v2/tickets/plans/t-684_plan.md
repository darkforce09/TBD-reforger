# T-684 — Plan

## Context

Launch parameters (OFCRA's 29 Params, FNF's single PerformanceTweaks) are chosen at launch, not authoring time; TBD has no document object for them. Schema keys landed in T-706; this is the mod reader plus selection at launch.

## Approach

1. Read `framework_synthesis` C.3/C.5 evidence and the DTAS `description.ext` pairing bugs before designing.
2. `TBD_MissionLoader.c`: bind `params[]` (title, values, default, consuming symbol); new `Backend/TBD_MissionParams.c`: resolve the launch selection and expose `Get(symbol)` to consumers.
3. Compile; document the launch-selection surface (server config / lobby) used.

## Risks

- Without a lobby UI the selection comes from server config; state that honestly rather than promising a UI.

## Verification

- `cargo xtask mod compile` · `cargo xtask platform wave gate --slice T-684`
