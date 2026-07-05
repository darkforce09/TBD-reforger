# Send-off — T-090.5.1 (render spine scaffold)

**CWD:** `/home/Samuel/Projects/TBD-Reforger` (`main`)

```bash
./scripts/ticket prompt T-090 --slice T-090.5.1
```

**Plan:** [`.ai/artifacts/t090_10_map_engine_v2_implementation_plan.md`](t090_10_map_engine_v2_implementation_plan.md) §7 row T-090.5.1  
**Spec:** [`docs/specs/Mission_Creator_Architecture/t090_5_map_object_render_layer.md`](../../docs/specs/Mission_Creator_Architecture/t090_5_map_object_render_layer.md)  
**LOD v2:** [`docs/specs/Mission_Creator_Architecture/t090_render_lod_contract.md`](../../docs/specs/Mission_Creator_Architecture/t090_render_lod_contract.md)  
**Prior:** T-090.3.2 shipped @ `a055df95` — [verify log](t090_3_2_verify_log.md)

**Scope (scaffold only — no vector drawing):**

- `worldmap/` skeleton: `styleModes`, `lodGates`, `chunkMath` (pure + vitest)
- Satellite `opacity` prop on basemap layer
- 3-way `mapStyle` + `worldLayerPrefs` migration (`basemapView` shim)
- Worker + client skeleton (`workers/worldObjects.worker.ts`, `worldObjectsClient.ts`)
- Feature flag `worldmap.enabled` (default off — zero regression)

**Gates:** vitest (styleModes, lodGates table, chunkMath, prefs migration); FE build/lint; manual M1–M3 (sat unchanged @ styles-off, style switch, flag off).

**Single lane:** no T-090.5.2 until 5.1 ships.

**Export data ready:** Everon manifest has P1+P2 — 361 prefabs, 507k instances, 625 TBDD density grids, 36 forest regions. Do not touch export pipeline this slice.
