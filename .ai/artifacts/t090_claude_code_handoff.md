# T-090 — Claude Code handoff (active slice T-090.3.0)

**Generated:** 2026-06-26 · **Executor:** claude-code  
**Program hub:** [`docs/specs/Mission_Creator_Architecture/t090_091_map_terrain_program.md`](../docs/specs/Mission_Creator_Architecture/t090_091_map_terrain_program.md)

---

## Active slice: T-090.3.0 — Workbench world-export feasibility spike

**Spec (read first):** [`t090_3_0_workbench_export_spike.md`](../docs/specs/Mission_Creator_Architecture/t090_3_0_workbench_export_spike.md)
· constants **N1–N12** + Audit closure: [`t090_091_map_terrain_program.md`](../docs/specs/Mission_Creator_Architecture/t090_091_map_terrain_program.md)

### Goal

Prove on a ~512 m Everon subregion (gates K1–K7): enumerate world entities → `raw-entities.jsonl`, read
per-prefab **OBB**, capture **1 Satellite + 1 Map** tile (or document the **synthesized-cartographic**
fallback, N9), probe a vegetation/**forest mask** (N5 path A vs the mandatory derived-hull fallback), and
record rotation handedness + `localUp → world Z` remap (L2). Write the ops log
`.ai/artifacts/map_export_everon.json`. **No full export and no T-090.1 basemap ship until K1–K7 pass.**

### Blocker chain (registry `status`, not `blocked_by`)

**T-090.0.2** schemas + goldens (shipped, this pass) → **T-090.3.0** spike (**active**) → **T-090.1**
Satellite basemap (queued until 0.2 + 3.0) → **T-090.1.1** Map → **T-090.2** taxonomy (+ forest regions)
→ **T-090.3** export (+ forest-regions, dual tiles) → {**T-090.4**, **T-090.6**, **T-090.8**} →
**T-090.5** render (Deck-zoom LOD, forests first) → **T-090.9** interaction → **T-090.7** AI.

### Key files

| Area | Path |
|------|------|
| Basemap today | `apps/website/frontend/src/features/tactical-map/layers/useBaseMapLayer.ts` |
| Terrains manifest URL | `apps/website/frontend/src/features/tactical-map/coords/terrains.ts` |
| Map host | `apps/website/frontend/src/features/tactical-map/TacticalMap.tsx` |
| Schema | `packages/tbd-schema/schema/terrain-manifest.schema.json` |
| Dev static | `make map-assets-link` → `public/map-assets/` |

### Verification (must pass)

```bash
make verify-terrain
make schema-validate
cd apps/website/frontend && npm run build && npm run lint
```

Manual: H1/H2 corner alignment gates in T-090.1 spec; toast on 404; grid stays on top.

---

## Queue after T-090.3.0 (do not start until slice advances)

| Slice | Spec | Notes |
|-------|------|-------|
| T-090.2 | `t090_2_map_object_taxonomy.md` | Schema + **ai** prefab blocks + golden sample |
| T-090.3 | `t090_terrain_export_pipeline.md` | **`make map-export TERRAIN=*`** — all maps, automated |
| T-090.4 | `t090_4_z_placement_audit.md` | Phase A (runs inside export) |
| T-090.6 | `t090_6_geometry_placement_audit.md` | Phase B OBB (runs inside export) |
| T-090.5 | `t090_5_map_object_render_layer.md` | Deck layers |

**AI after export:** `.ai/artifacts/map_export_{terrainId}.json`  
**Rules (all maps):** `packages/tbd-schema/rules/prefab-classify.json`

**UX reference:** `t090_eden_map_reference.md`  
**Out of scope:** T-126 building floor selector (`idea`)

---

## AI cost reminders

- Full object catalog: **50–500 MB** gzip — never `JSON.parse` whole file on main thread (T-090.5 uses chunks).
- Tile pyramid z0–5: **200–800 MB** LFS — lazy TileLayer fetch only.

---

## Spawn gate (unchanged)

T-092 still blocks T-071 / T-068 Phase 2. T-090.5 is **visual only** — spawn authority remains mod `GetSurfaceY`.
