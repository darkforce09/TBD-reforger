# Send-off — T-090.8.1 (forest / rock mass render)

**CWD:** `/home/Samuel/Projects/TBD-Reforger` (`main`)

**Plan:** [`.ai/artifacts/t090_10_map_engine_v2_implementation_plan.md`](t090_10_map_engine_v2_implementation_plan.md) §7 row T-090.8.1  
**Spec:** [`docs/specs/Mission_Creator_Architecture/t090_8_forest_vegetation_regions.md`](../../docs/specs/Mission_Creator_Architecture/t090_8_forest_vegetation_regions.md)  
**LOD v2:** [`docs/specs/Mission_Creator_Architecture/t090_render_lod_contract.md`](../../docs/specs/Mission_Creator_Architecture/t090_render_lod_contract.md)  
**Prior:** T-090.5.3 @ `155651b9` — [verify log](t090_5_3_verify_log.md)

**Scope:**

- `worldmap/forestMass.ts` — marching squares from `objects/density/*.bin` TBDD grids (worker or core)
- `worldmap/landCoverRegions.ts` — `forest-regions.json.gz` Path B hulls (36 regions)
- Deck layers: `world-forest` fill + `world-forest-outline` (plan §4.2 slots)
- Wire into `useWorldMapLayers` behind `VITE_WORLDMAP_ENABLED=1`
- **No individual tree glyphs** — forest polygons only @ deckZoom ≤ +1 (LOD3: trees hidden below 0)

**Gates:** F3 (@ −2 polygons, no tree icons), F4/F5, LOD3-v2 vitest, N11 P2b budgets.

**Single lane:** no T-090.5.4 until 8.1 ships.

**Data ready:** 625 TBDD grids, 36 forest regions, 501k trees in chunks (not rendered as icons until T-090.5.5).

**Operator:** `VITE_WORLDMAP_ENABLED=1` + hard refresh after deploy.
