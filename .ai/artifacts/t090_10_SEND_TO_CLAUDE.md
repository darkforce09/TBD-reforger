# Send-off — T-090.10.1 (Map Engine v2 implementation plan)

**CWD:** `/home/Samuel/Projects/TBD-Reforger` (`main`)

```bash
./scripts/ticket prompt T-090 --slice T-090.10.1
```

**Spec:** [`docs/specs/Mission_Creator_Architecture/t090_10_map_engine_v2.md`](../../docs/specs/Mission_Creator_Architecture/t090_10_map_engine_v2.md)  
**A3 authority:** [`.ai/artifacts/t144_arma3_map_architecture_report.md`](t144_arma3_map_architecture_report.md)  
**Legacy (do not extend):** [`docs/specs/Mission_Creator_Architecture/t090_legacy_raster_pipeline.md`](../../docs/specs/Mission_Creator_Architecture/t090_legacy_raster_pipeline.md)

**Product:** Operator pivoted the entire T-090 map program to **A3-structural parity** (data + vectors, not raster compose). Your job is **plan only** — no application code.

**Deliverable (mandatory path):**  
`.ai/artifacts/t090_10_map_engine_v2_implementation_plan.md`

---

## Copy-paste prompt for Claude Code

```
Read CLAUDE.md §Status first. Active slice: T-090.10.1 — Map Engine v2 implementation PLAN (no code).

Context:
- T-144.1 shipped @ b1949182 — A3 2D map draws LIVE from world data (GLandscape), no readability tile pyramid. Sat = terrain texture; Map look = sea band + contours + vectors. Sat↔Map = zoom crossfade; vectors always on top. LOD = density gates per feature class + per-type importance — NO clustering for world objects.
- Operator decision: KILL the legacy raster-compose pipeline (dual map pyramid, bake roads/land-cover into pixels, T-090.1.2.9, T-090.1.2.3). KEEP: tbd-sat unified texture as frozen photo field (A3 DrawField analogue), DEM/Z (T-091), T-063 pick, T-090.3 export direction, MC slot perf (T-057–067).
- Scaffold spec: docs/specs/Mission_Creator_Architecture/t090_10_map_engine_v2.md
- Legacy disposition: docs/specs/Mission_Creator_Architecture/t090_legacy_raster_pipeline.md

Your task — write ONLY:
  .ai/artifacts/t090_10_map_engine_v2_implementation_plan.md

The plan must be exhaustive enough that the next Claude Code slices (T-090.3 → T-090.5 → T-090.8 → T-090.9) can execute without ambiguity. Include:

1. EXECUTIVE SUMMARY — one paragraph on the pivot and what "done" looks like for Map Engine v2.

2. A3 → DECK MAPPING TABLE — every DrawBackground layer (uiMap.cpp) mapped to: Deck layer type, data source, file path in our repo, new vs reuse.

3. DATA CONTRACT — exact export artifacts (extend T-090.3): chunk format, density grid for forests, road schema, mapType/importance from prefabs. What changes in TBD_TerrainWorldExportPlugin.c vs current t090_3_map_asset_export.md. Drop dual pyramid pass A2 explicitly.

4. RENDER SPINE — module layout under apps/website/frontend/src/features/tactical-map/ (new files, refactors). Layer order, z-index, how sat opacity crossfade replaces Satellite|Map radio. Migration path for useTerrainBasemapLayer / tiles/map/.

5. LOD TABLE — numeric: A3 ptsPerSquare equivalents → Deck orthographic zoom thresholds per class (roads, buildings, trees, props, labels). Per-type importance gates from t090_world_object_type_inventory.md. Confirm: world layer NO supercluster; slot cluster unchanged.

6. CHUNK STREAMING — A3 landSave analogue: per-frame hydrate budget, 5% border preload, neighbor radius for oversized objects. Worker vs main thread split (t090_world_objects_worker.md).

7. PHASED SLICES — break work into implementable slices with:
   - slice ID (T-090.3.x, T-090.5.x, etc. if needed)
   - files touched
   - acceptance gates (make commands, vitest, manual zoom checklist)
   - dependencies

8. LEGACY MIGRATION — what to do with: build-map-cartographic.mjs, tiles/map/, Mission Settings basemap radio, t090_basemap_dual_view.md, shipped T-090.1.1.1 land-cover compose.

9. RISK REGISTER — perf @ 1M objects, memory, first-paint, fallback if a layer fails, CI/LFS implications.

10. OPEN QUESTIONS — list anything needing operator decision; default recommendation per item.

Read before writing:
- .ai/artifacts/t144_arma3_map_architecture_report.md (§2–§3, §7, §9–§10)
- docs/specs/Mission_Creator_Architecture/t090_3_map_asset_export.md
- docs/specs/Mission_Creator_Architecture/t090_5_map_object_render_layer.md
- docs/specs/Mission_Creator_Architecture/t090_render_lod_contract.md
- apps/website/frontend/src/features/tactical-map/TacticalMap.tsx
- apps/website/frontend/src/features/tactical-map/layers/useTerrainBasemapLayer.ts

Do NOT write application code. Do NOT extend raster compose scripts. Do NOT reopen T-090.1.2.9 or T-090.1.2.3.

When done: summarize in verify log .ai/artifacts/t090_10_1_verify_log.md with checklist PASS/FAIL per section 1–10.
```

---

**After plan lands:** Operator reviews → Cursor doc sync → active slice advances to **T-090.3** (data export, rescoped).
