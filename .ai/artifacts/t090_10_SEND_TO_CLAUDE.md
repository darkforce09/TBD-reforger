# Send-off — T-090.5.5 (tree / veg / prop glyphs)

**CWD:** `/home/Samuel/Projects/TBD-Reforger` (`main`)

**Plan:** [`.ai/artifacts/t090_10_map_engine_v2_implementation_plan.md`](t090_10_map_engine_v2_implementation_plan.md) §7 row T-090.5.5  
**Spec:** [`docs/specs/Mission_Creator_Architecture/t090_5_map_object_render_layer.md`](../../docs/specs/Mission_Creator_Architecture/t090_5_map_object_render_layer.md)  
**Glyphs:** [`docs/specs/Mission_Creator_Architecture/t090_world_object_glyphs.md`](../../docs/specs/Mission_Creator_Architecture/t090_world_object_glyphs.md)  
**LOD v2:** [`docs/specs/Mission_Creator_Architecture/t090_render_lod_contract.md`](../../docs/specs/Mission_Creator_Architecture/t090_render_lod_contract.md)  
**Prior:** T-090.5.4 @ `bd481cf1` — [verify log](t090_5_4_verify_log.md)

**Scope:**

- `worldmap/treePropLayer.ts` — IconLayer over worker `visibleInstances` (viewport-streamed)
- Layer ids: `world-trees`, `world-props` (plan §4.2 slots 9–10)
- PH-P2…P5 glyph SVGs + atlas entries; `importanceZoom` overrides; optional `heightM` size cap (1.5×)
- Wire into `useWorldMapLayers`; respect `trees` / `props` / `forest` toggles + lodGates

**Gates:** `make map-glyphs-verify` (R7/GL-G1–G6); vitest LOD3 inversion @ −2 (trees hidden); INSTANCE_BUDGET @ +1/+3; R5 ≥55 fps @ PH-P2 visible band; R8 rotation pick.

**Single lane:** no T-090.9 until 5.5 ships.

**Operator:** `VITE_WORLDMAP_ENABLED=1` + hard refresh.
