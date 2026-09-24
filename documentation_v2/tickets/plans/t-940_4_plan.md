# T-940.4 — Plan

## Context
telemetry.rs:594-598 ignores flat top-level deaths (documented :579-592); the mod's Backend/TBD_ResultsReporter.c emits
the flat shape. Golden: tests/deployments_combat.rs (T-393). Priority 0 data loss.

## Approach
1. Verify on main: post the reporter's flat payload → deaths stored 0; paste the red.
2. telemetry.rs: fold flat counters into the nested structure when the nested block is absent.
3. TBD_ResultsReporter.c: emit the nested block, keep the flat fields one release; `cargo xtask mod compile`.
4. deployments_combat.rs: flat golden + both-shapes-equal case.
5. Perturbation: skip the kills fold → flat golden red; restore, touch, green.

## Risks
- Double counting when both shapes are present: nested wins, flat ignored.

## Verification
- `cargo xtask db test-it`; `cargo xtask mod compile`
- `cargo xtask platform wave gate --slice T-940.4`
