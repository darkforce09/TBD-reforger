# Send-off — T-090.5.2 (roads + buildings live)

**CWD:** `/home/Samuel/Projects/TBD-Reforger` (`main`)

**Plan:** [`.ai/artifacts/t090_10_map_engine_v2_implementation_plan.md`](t090_10_map_engine_v2_implementation_plan.md) §7 row T-090.5.2  
**Spec:** [`docs/specs/Mission_Creator_Architecture/t090_5_map_object_render_layer.md`](../../docs/specs/Mission_Creator_Architecture/t090_5_map_object_render_layer.md)  
**Glyphs:** [`docs/specs/Mission_Creator_Architecture/t090_world_object_glyphs.md`](../../docs/specs/Mission_Creator_Architecture/t090_world_object_glyphs.md)  
**LOD v2:** [`docs/specs/Mission_Creator_Architecture/t090_render_lod_contract.md`](../../docs/specs/Mission_Creator_Architecture/t090_render_lod_contract.md)  
**Prior:** T-090.5.1 shipped @ `589ded9e` — [verify log](t090_5_1_verify_log.md)

**Scope:**

- `worldmap/roadLayer.ts` — PathLayer from `roads.json.gz` (766 segments)
- `worldmap/buildingLayer.ts` — PolygonLayer OBB rects from P1 chunks
- `layers/worldGlyphAtlas.ts` + P1 `building-*` SVG set + `build-glyph-atlas.mjs`
- Wire into `useWorldMapLayers` / TacticalMap insertion point (behind `VITE_WORLDMAP_ENABLED=1`)
- LOD vitest: road classes per band, buildings ≥ −2.5

**Gates:** R1–R4 + R7 (`make map-glyphs-verify` GL-G1…G6); manual Z1–Z6; ≥55 fps @ PH-P1 data (R5).

**Single lane:** no T-090.5.3 until 5.2 ships.

**Spine ready:** `styleModes`, `lodGates`, `chunkMath`, `worldLayerPrefs`, worker skeleton — do not rewrite.
