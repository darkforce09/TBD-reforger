# Send-off — T-090.3.1 (Map Engine v2 export core)

**CWD:** `/home/Samuel/Projects/TBD-Reforger` (`main`)

```bash
./scripts/ticket prompt T-090 --slice T-090.3.1
```

**Plan (normative):** [`.ai/artifacts/t090_10_map_engine_v2_implementation_plan.md`](t090_10_map_engine_v2_implementation_plan.md) §3 + §7 row T-090.3.1  
**LOD v2:** [`docs/specs/Mission_Creator_Architecture/t090_render_lod_contract.md`](../../docs/specs/Mission_Creator_Architecture/t090_render_lod_contract.md)  
**Parallel (optional):** T-090.5.1 render scaffold — separate session

**Scope:** Plugin full-world iterate + host post-process → `prefabs.json.gz`, `chunks/{cx}_{cy}.json.gz`, **`roads.json.gz` (Q1 pulled forward)**, census, schema bumps (`render.importanceZoom`). Realize `make map-export` / `map-verify-phase` stubs.

**Do NOT:** extend raster compose; reopen cancelled slices; dual-pyramid pass A2.
