# T-938.5 — Plan

## Context
building_viewshed.rs:251-268 (via :156-195) fires ~31k synchronous raycasts per level (r=25 m, cell 0.25);
dem/sample.rs:492-535 is O((R/C)^2) with no clamp. Call path: canvas/gestures.rs:1293-1325 →
los_tool::place_viewshed (:904). los_tool.rs is SIZE-3 allowlisted — call-site edits only.

## Approach
1. Verify on main: time place_viewshed at r=25 on a golden building → paste the blocking ms.
2. Both loops become resumable batch iterators with a cancel token; caps r ≤ 400 m, cells ≤ 250k.
3. `tools/viewshed_scheduler.rs` (new, in tools/mod.rs): one job per tool, ≤ 4 ms batches, cancel on new.
4. los_tool.rs:904 submits to the scheduler; perturbation: iterator skips its last row → red; restore, touch, green.

## Risks
- Partial results drawn mid-job — draw only completed bands.
- los_tool.rs length — no new logic there, or the file-length gate fails.

## Verification
- `cargo test -p map-engine-core --all-features`; `cargo xtask mk ci-local-leptos`
- `cargo xtask platform wave gate --slice T-938.5`
