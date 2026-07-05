# Send-off — T-090.5.4 (sea-band + DEM contours)

**CWD:** `/home/Samuel/Projects/TBD-Reforger` (`main`)

**Plan:** [`.ai/artifacts/t090_10_map_engine_v2_implementation_plan.md`](t090_10_map_engine_v2_implementation_plan.md) §7 row T-090.5.4  
**Spec:** [`docs/specs/Mission_Creator_Architecture/t090_5_map_object_render_layer.md`](../../docs/specs/Mission_Creator_Architecture/t090_5_map_object_render_layer.md)  
**LOD v2:** [`docs/specs/Mission_Creator_Architecture/t090_render_lod_contract.md`](../../docs/specs/Mission_Creator_Architecture/t090_render_lod_contract.md)  
**Prior:** T-090.8.1 @ `e28d073a` — [verify log](t090_8_1_verify_log.md)

**Scope:**

- `worldmap/seaBand.ts` — DEM → ocean/shore polygons (pure; runs in worker)
- `worldmap/contours.ts` — DEM → iso polylines per interval band (N3 ladder)
- Layer builders: `world-sea`, `world-contours` (plan §4.2 slots 2, 5)
- Wire into `useWorldMapLayers`; reuse worker + existing DEM loader

**Gates:** vitest on pure fns (known DEM fixtures); manual shoreline vs water-composite visual; contour interval ladder per §N3; perf off main thread.

**Single lane:** no T-090.5.5 until 5.4 ships.

**Operator:** `VITE_WORLDMAP_ENABLED=1` + hard refresh.
