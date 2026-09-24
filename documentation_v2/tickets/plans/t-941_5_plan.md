# T-941.5 — Plan

## Context
TBD_SpectatorComponent.c:61-62 m_fHostMaxRangeM defaults to 0 = unlimited (comment :56-60); no authority clamp.

## Approach
1. Verify on main: read :56-62; record as defect evidence.
2. Default 2000 m; 0 → default; clamp every requested range on the authority.
3. Reword the :56-60 comment with the T-941.5 date.
4. Perturbation: clamp against a string literal → compile red; restore, touch, green.

## Risks
- Existing server configs with 0 change behaviour: documented in the comment and the checklist.

## Verification
- `cargo xtask mod compile`; `cargo xtask mod world-boot`
- `cargo xtask platform wave gate --slice T-941.5`
- Checklist: 10 km request on a 2 km server is held at 2 km; unset attribute yields 2 km.
