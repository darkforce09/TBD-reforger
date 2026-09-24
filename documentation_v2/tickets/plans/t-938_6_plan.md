# T-938.6 — Plan

## Context
Satellite L0 is 655 MB RGBA; loaders allocate independently and the first OOM aborts the wasm instance.
The DEM decode peak is T-935.4's. Budgets are measured against the binary formats, hence after T-935.13.

## Approach
1. Verify on main: everon under a 512 MB heap cap → paste the abort.
2. `world_assets/memory_budget.rs` (new, in world_assets/mod.rs): peaks per asset, budget, reserve() → Decision.
3. satellite.rs: consult the budget per mip level; Degrade raises the floor by one and logs.
4. Debug HUD row: reserved / budget + current floor. Record measured peaks in the report.
5. Perturbation: reserve never degrades → floor test red; restore, touch, green.

## Risks
- Browser heap limits vary — default 1536 MB with a query-param override.
- Only satellite degrades in this slice; other assets register peaks so the HUD is honest.

## Verification
- `cargo xtask mk ci-local-leptos`; `cargo xtask mk leptos-gates`
- `cargo xtask platform wave gate --slice T-938.6`
