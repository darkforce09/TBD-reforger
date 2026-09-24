# T-941.1 — Plan

## Context
TBD_SafestartManager.c:252-265 arms only for SAFE_START (deliberate :248-250); LOBBY auto-deploy (~250 ms) puts bodies
in the world unshielded. Priority 0.

## Approach
1. Verify on main: read :252-265; record the armed stage set as defect evidence.
2. Widen the arm predicate to LOBBY, BRIEFING, SAFE_START; disarm once on LIVE; no per-body re-apply.
3. Replace the :248-250 comment with a dated decision naming T-941.1.
4. `cargo xtask mod compile`; `cargo xtask mod world-boot`.
5. Perturbation: compare the stage to a string literal → compile red; restore, touch, green.

## Risks
- Late joiners during BRIEFING: the shield applies on spawn, not only on stage change.

## Verification
- `cargo xtask mod compile`; `cargo xtask mod world-boot`
- `cargo xtask platform wave gate --slice T-941.1`
- Checklist: grenade during LOBBY does no damage; damage resumes after GoLive.
