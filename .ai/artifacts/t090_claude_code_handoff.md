# T-090 — Claude Code handoff

**Updated:** 2026-06-30 · **Executor:** claude-code · **Commit on:** `main`

**Program hub:** [`docs/specs/Mission_Creator_Architecture/t090_091_map_terrain_program.md`](../../docs/specs/Mission_Creator_Architecture/t090_091_map_terrain_program.md)

---

## Shipped: T-090.3.0 @ `b342c35`

Enumeration proven on real Everon. Ops log: [`.ai/artifacts/map_export_everon.json`](map_export_everon.json)

| Gate | Result |
|------|--------|
| K1/K1b/K2/K5/K6/K7 | PASS |
| K3 | FAIL → **T-090.1** closes this |
| K4 | FAIL on gate; real `.topo` source found → **T-090.1.1** |

Harness: `scripts/map-assets/verify-spike-*.mjs`, `verify-spike-all.sh`, `TBD_TerrainWorldExportPlugin.c`

**Infra:** MCP shell tooling hardened @ `e7e7232` — [`docs/mod/MCP_TOOLING.md`](../../docs/mod/MCP_TOOLING.md). Use `mcp-call.sh` for Workbench probes (no raw JSON-RPC workaround).

---

## Active slice: T-090.1 — Satellite basemap

**Handoff:** [`.ai/artifacts/t090_1_claude_code_handoff.md`](t090_1_claude_code_handoff.md)  
**Spec:** [`t090_091_map_terrain_program.md`](../../docs/specs/Mission_Creator_Architecture/t090_091_map_terrain_program.md) *(planned `t090_1_aligned_basemap.md` was never created — superseded by the program hub)*

### Blocker chain

**T-090.1** (active) → **T-090.1.1** Map → **T-090.2** → **T-090.3** → {**.4/.6/.8**} → **T-090.5** → **T-090.9** → **T-090.7**

### Key frontend paths

| Area | Path |
|------|------|
| Basemap today | `apps/website/frontend/src/features/tactical-map/layers/useBaseMapLayer.ts` |
| Terrains | `apps/website/frontend/src/features/tactical-map/coords/terrains.ts` |
| Map host | `apps/website/frontend/src/features/tactical-map/TacticalMap.tsx` |
| Manifest schema | `packages/tbd-schema/schema/terrain-manifest.schema.json` |
| Dev static | `make map-assets-link` |
