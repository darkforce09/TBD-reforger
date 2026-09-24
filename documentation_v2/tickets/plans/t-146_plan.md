# T-146 — Plan
## Context
T-150 (universal registry export) is shipped and `dock_right.rs` already holds `registry_items` (the raw `/registry` rows, `RegistryItem` in `core/dto.rs`); `asset_catalog.rs:146` `build_catalog_tree(items, side)` consumes them for the per-faction tree (T-809). Vehicles and crates from those rows still do not reach the Asset Browser as drag-placeable leaves — the Eden F1 parity gap.

## Approach
1. In `apps/website/frontend/src/editor/arsenal/asset_catalog.rs`, extend `build_catalog_tree` to emit vehicle and crate leaves from the registry rows (kind-keyed nodes beside characters), keeping search and the empty-state copy intact.
2. Route the new leaves through the existing palette-drag place path — no second placement lane.
3. Native tests next to the existing catalog tests: a fixture with characters + vehicles + crates yields the expected tree; search hits the new leaves.

## Risks
- Registry rows for vehicles may lack the fields the place path expects; fallback is to place with the registry prefab id and let attributes fill later.
- `asset_catalog.rs` is a SIZE-3 allowlisted file (T-905) — new code goes in a sibling module if it does not fit.

## Verification
- `cargo xtask mk leptos-gates`
- `cargo xtask platform wave gate --slice T-146`
