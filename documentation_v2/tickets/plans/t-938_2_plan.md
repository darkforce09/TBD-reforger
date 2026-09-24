# T-938.2 — Plan

## Context
The audit claims world_host.rs:454-525 allocates 10+ buffers per chunk crossing. UNVERIFIED on main.
world_host.rs is owned by T-935.3/.11; this slice packs after T-935.11.

## Approach
1. Add an allocation counter behind a debug flag on the crossing path.
2. Cross three chunk boundaries in the dev editor; record allocations per crossing.
3. If < 3 per crossing: report, keep only the counter, stop.
4. Else: ring of staging buffers per lane sized from the measured max; re-measure; paste before/after.
5. Perturbation (only if implemented): ring of one → reuse test red; restore, touch, green.

## Risks
- Measuring after T-935.11 means the binary loader path, not the gz-JSON one the audit read.

## Verification
- `cargo xtask mk ci-local-leptos`
- `cargo xtask platform wave gate --slice T-938.2`
