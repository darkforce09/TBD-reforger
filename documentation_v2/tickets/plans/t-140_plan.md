# T-140 — Plan

## Context
compile.rs emits the mission with no size accounting; console clients have an unmeasured payload ceiling. The
backlog asks for the curve (size vs slots/entities) and a policy (client sync vs server-only bulk).

## Approach
1. New `crates/map-engine-core/src/mission/payload_budget.rs` (register in `mission/mod.rs`): `measure(&Value) ->
   Measure {bytes, slots, entities, vehicles}`, `Budget` table with info/warn/error thresholds, `diagnose(&Measure)`.
2. `mission/compile.rs` compile_export: push the PAYLOAD-BUDGET diagnostic; payload untouched.
3. Test: scale the largest golden 1×/4×/16× in memory, assert monotone bytes and the expected tiers.
4. Perturbation: halve the error threshold in the test's expectation → red; restore, `touch`, green.

## Risks
- Thresholds are a guess until measured on a console; name them as provisional constants and put the
  measured numbers in the report so the operator can pin them.

## Verification
- `cargo test -p map-engine-core --all-features mission::payload_budget`
- `cargo xtask platform wave gate --slice T-140`
