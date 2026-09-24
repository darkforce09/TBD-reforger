# T-131 — Plan

## Context
Roads are parsed (world/roads.rs RoadSegment) but never joined into a graph; the backlog asks for a planning tool
(waypoints, distance, elevation), explicitly not convoy AI. Builds on T-935.6's rkyv road loader.

## Approach
1. New `crates/map-engine-core/src/world/road_graph.rs` (register in `world/mod.rs`): snap segment endpoints
   within 1.5 m into nodes, edges with metre lengths, Dijkstra; tests: synthetic grid, everon fixture connectivity
   per class, 50 ms budget.
2. New `editor/tools/route_planner.rs` (register in `tools/mod.rs`): click → nearest node, path render via the
   existing overlay path, distance readout, elevation sampled from the DEM along the path.
3. Perturbation: snapping tolerance 0 → connectivity test red; restore, `touch`, green.

## Risks
- Everon road classes may be disconnected at bridges; report the count and keep per-class connectivity.
- Native 50 ms budget on the everon graph (888 segments) is generous; if exceeded, switch to A*.

## Verification
- `cargo test -p map-engine-core --all-features world::road_graph` · leptos gates · `cargo xtask platform wave gate --slice T-131`
