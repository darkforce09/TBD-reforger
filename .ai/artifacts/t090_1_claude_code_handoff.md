# T-090.1 — Claude Code handoff (Satellite basemap)

**Generated:** 2026-06-30 · **Executor:** claude-code · **Commit on:** `main` (`T-090.1:` prefix)  
**Program hub:** [`docs/specs/Mission_Creator_Architecture/t090_091_map_terrain_program.md`](../docs/specs/Mission_Creator_Architecture/t090_091_map_terrain_program.md)  
**Spike ops log:** [`.ai/artifacts/map_export_everon.json`](map_export_everon.json) @ spike `b342c35`

---

## Active slice: T-090.1 — Satellite basemap + tile pyramid

**Spec (read first):** [`t090_1_aligned_basemap.md`](../docs/specs/Mission_Creator_Architecture/t090_1_aligned_basemap.md)  
**Dual-view contract:** [`t090_basemap_dual_view.md`](../docs/specs/Mission_Creator_Architecture/t090_basemap_dual_view.md) (Map tiles = **T-090.1.1**)

### Goal

Close spike **K3**: extract Everon satellite imagery from engine sources → aligned XYZ WebP pyramid under `packages/map-assets/everon/tiles/satellite/{z}/{x}/{y}.webp`, update manifest, wire **Cartesian** `TileLayer` in Mission Creator with **Y inversion** (`tmsY = 2**z - 1 - y`).

### Spike inputs (do not re-discover)

| Item | Value |
|------|-------|
| Satellite source | `system/terrain/defSatMap_BCR.edds` + per-terrain SAP/satellite background |
| Cartographic (next slice) | World Editor **Export Map Data** → `.topo` — **not T-090.1** |
| World bounds | Everon 12800×12800 m, origin bottom-left |
| Enumeration API | `BaseWorld.QueryEntitiesByAABB` (full export later — not this slice) |
| S0 lesson | Confirm loaded world in WE — `wb_state` entity count unreliable |

### K3 exit (this slice)

```bash
test -f packages/map-assets/everon/tiles/satellite/0/0/0.webp
make ci-local-frontend
make verify-terrain
make schema-validate
```

Manual: H1/H2/H2b alignment log in PR or `.ai/artifacts/t090_1_verify_log.md`.

### Files (frontend — from spec)

| File | Change |
|------|--------|
| `layers/useBaseMapLayer.ts` | Manifest fetch; basemap + grid |
| `layers/useTerrainBasemapLayer.ts` | **New** — TileLayer + Y flip |
| `coords/terrainManifest.ts` | **New** — parse manifest |
| `TacticalMap.tsx` | Wire terrain id |

### Files (assets / pipeline — likely new)

| File | Change |
|------|--------|
| `packages/map-assets/everon/tiles/satellite/**` | WebP pyramid z0–5 |
| `packages/map-assets/everon/manifest.json` | `tiles.satellite.urlTemplate`, bounds |
| Node script(s) under `scripts/map-assets/` or `packages/tbd-schema/scripts/` | `.edds` → PNG/WebP + pyramid slice |

### Out of scope

- Map view tiles (T-090.1.1)
- Full world object export (T-090.3)
- N9 synthesized cartographic (not required — real `.topo` exists)
- Docs (Cursor sync after ship)

---

## MCP (prerequisite — shipped @ `e7e7232`)

Shell MCP is reliable — warm daemon ~0.3 s per call. Reference: [`docs/mod/MCP_TOOLING.md`](../docs/mod/MCP_TOOLING.md).

```bash
bash scripts/mod/mcp-call-selftest.sh   # offline 19/19
bash scripts/mod/tbd-dev-bootstrap.sh   # pre-warms daemon + wb_connect
bash scripts/mod/mcp-smoke.sh           # live gate
```

During the T-090.3.0 spike, args-bearing calls were flaky due to a bash `${2:-{}}` brace bug (fixed @ `e7e7232`) — do not use raw JSON-RPC workarounds.

---

## After ship

- Update `.ai/artifacts/map_export_everon.json` → `gates.K3: "pass"`, `tileFindings.satellite.path` set
- Cursor: registry `T-090.1` shipped, `active_slice` → `T-090.1.1`
- Tag `T-090.1` on ship commit
