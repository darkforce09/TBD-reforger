# T-090 / T-091 — Map & terrain program (hub)

**Status:** **active — T-090.1.2** (SAP supertexture high-detail satellite). **T-090.1 shipped** @ `564419e` (interim rasterization + LOD — aligned). **T-090.3.0 shipped** @ `b342c35`. **T-091 complete** @ `dde589e`.  
**Tickets:** T-090 · T-091 · **Route:** `/missions/:id/edit`  
**Registry:** [`.ai/tickets/registry.json`](../../../.ai/tickets/registry.json)  
**Spawn parity (separate hub):** [`t092_spawn_transform_program.md`](t092_spawn_transform_program.md)  
**UX reference:** [`t090_eden_map_reference.md`](t090_eden_map_reference.md)  
**One-command export (all maps):** [`t090_terrain_export_pipeline.md`](t090_terrain_export_pipeline.md) — `make map-export TERRAIN=<id>`

**Hard gate:** **T-091.0** anchor verify **PASS** (`make verify-terrain-strict` @ `6d96339`). T-071 ORBAT / T-068 Phase 2 loadout still blocked on **T-092.2** mod compile + spawn verify. **Building floor selector** explicitly **out of scope** → **T-126** (`idea`).

**Workbench MCP:** shell tooling hardened @ `e7e7232` — [`docs/mod/MCP_TOOLING.md`](../../mod/MCP_TOOLING.md). Offline: `make mcp-selftest`. Live: `bash scripts/mod/tbd-dev-bootstrap.sh` then `make mcp-smoke`.

---

## Program order

**Normative order (identical in ROADMAP + handoff). Gates encoded by registry `status`, not `blocked_by`.**

```text
T-090.0    hub + schema + verify scripts (shipped)
T-090.0.1  program expansion — slices + taxonomy + cost docs (shipped)
T-090.0.2  map-object schemas + goldens + verify wiring (shipped @ this pass)  ✓
  → T-091.0/.1/.2  DEM + Z + hillshade (shipped)  ✓
  → T-090.3.0  Workbench export feasibility spike (shipped @ b342c35)  ✓
  → T-090.1    Satellite basemap (interim rasterization + LOD)  ✓ @ 564419e
  → T-090.1.2  SAP supertexture ortho — true satellite detail  ← ACTIVE NOW
  → T-090.1.1  Map basemap (Export Map Data .topo — roads/cartographic)
  → T-090.2    taxonomy + JSON schema (+ forest/field/waterBody region kinds)
  → T-090.3    phased export (+ forest-regions, dual tiles) — P1 → P10
  → T-090.4 + T-090.6 + T-090.8  Z/geometry audits + forest regions (parallel where deps met)
  → T-090.5    Deck.gl layers (forests first, Deck-orthographic-zoom LOD per render contract)
  → T-090.9    world-object interaction — hover, inspect, filter, legend (read-only)
  → T-090.7    Eden AI read API — resolveWorldObject, queryByCover, context pack
  → T-092      mod compile + spawn → T-071 → T-068.13 → T-068.7+
  → T-110      binary base + sparse deltas (consumer of catalog v1 — outside T-090)
  → T-126      building floor selector (idea — outside T-090)
```

**Blocker chain (implementers):** **T-090.1** (shipped @ `564419e`) → **T-090.1.2** (SAP supertexture detail, active) → **T-090.1.1** (Map) → **T-090.2** → **T-090.3** → {**T-090.4**, **T-090.6**, **T-090.8**} → **T-090.5** → **T-090.9** → **T-090.7**.

---

## Slice specs (read these — not optional)

Each slice has its **own spec file** with locked decisions, file touch list, and **mandatory verification gate** (automated commands + acceptance table).

| Slice | Spec | Executor | Exit gate |
|-------|------|----------|-----------|
| **T-090.0** | [`t090_0_map_program_hub.md`](t090_0_map_program_hub.md) | cursor-docs | **shipped** |
| **T-090.0.1** | this hub + slice specs below | cursor-docs | **shipped** — AI cost/taxonomy docs land |
| **T-090.0.2** | `map-object-*.schema.json` + `golden/map-objects/*` + `verify-map-*` + `verify-t090-spec-consistency` | cursor-docs | **shipped** (this pass) — `make schema-validate` |
| **T-091.0** | [`t091_0_dem_tile_export.md`](t091_0_dem_tile_export.md) | claude-code | **shipped** @ `6d96339` |
| **T-091.1** | [`t091_1_dem_loader.md`](t091_1_dem_loader.md) | claude-code | **shipped** @ `2c56c2e` |
| **T-091.2** | [`t091_2_z_axis_editor.md`](t091_2_z_axis_editor.md) | claude-code | **shipped** @ `dde589e` |
| **T-090.3.0** | [`t090_3_0_workbench_export_spike.md`](t090_3_0_workbench_export_spike.md) | claude-code | **shipped** @ `b342c35` |
| **T-090.1** | [`t090_1_aligned_basemap.md`](t090_1_aligned_basemap.md) | claude-code | **shipped** @ `564419e` — interim rasterization + LOD |
| **T-090.1.2** | [`t090_1_2_sap_supertexture_satellite.md`](t090_1_2_sap_supertexture_satellite.md) | claude-code | **SAP supertexture** high-detail satellite — **active** |
| **T-090.1.1** | [`t090_basemap_dual_view.md`](t090_basemap_dual_view.md) | claude-code | **Map** pyramid + view switch |
| **T-090.2** | [`t090_2_map_object_taxonomy.md`](t090_2_map_object_taxonomy.md) + [`t090_world_object_type_inventory.md`](t090_world_object_type_inventory.md) + [`t090_eden_ai_world_object_schema.md`](t090_eden_ai_world_object_schema.md) | claude-code | Enums + census + golden per `buildingClass` |
| **T-090.3** | [`t090_3_map_asset_export.md`](t090_3_map_asset_export.md) + [`t090_phased_object_import.md`](t090_phased_object_import.md) | claude-code | `map-export` + **`map-verify-phase` per P1–P10** |
| **T-090.4** | [`t090_4_z_placement_audit.md`](t090_4_z_placement_audit.md) | claude-code | Phase A pivot audit @ 1M |
| **T-090.6** | [`t090_6_geometry_placement_audit.md`](t090_6_geometry_placement_audit.md) | claude-code | Phase B OBB / visibility audit |
| **T-090.5** | [`t090_5_map_object_render_layer.md`](t090_5_map_object_render_layer.md) + [`t090_world_object_glyphs.md`](t090_world_object_glyphs.md) | claude-code | Layers + SVG atlas per class |
| **T-090.7** | [`t090_eden_ai_world_object_schema.md`](t090_eden_ai_world_object_schema.md) | claude-code | `resolveWorldObject` + AI context pack |
| **T-090.8** | [`t090_8_forest_vegetation_regions.md`](t090_8_forest_vegetation_regions.md) | claude-code | forest-regions export + render + inspect (F1–F6) |
| **T-090.9** | [`t090_9_world_object_interaction.md`](t090_9_world_object_interaction.md) | claude-code | hover + inspect + filter + legend (I1–I8) |

**Cross-cutting contracts (not slices):** render LOD authority [`t090_render_lod_contract.md`](t090_render_lod_contract.md) (N1–N3) · picking/worker [`t090_world_objects_worker.md`](t090_world_objects_worker.md).

---

## Audit closure (T-090 program audit 2026-06-30)

Every gap from [`.ai/artifacts/t090_program_audit_2026-06-30.md`](../../../.ai/artifacts/t090_program_audit_2026-06-30.md)
is closed by a spec + verify gate + slice. Owner constants **N1–N12** are the single source.

| Gap | Owning spec | Verify gate | Slice |
|-----|-------------|-------------|-------|
| GAP-001 forests first-class | [`t090_8_forest_vegetation_regions.md`](t090_8_forest_vegetation_regions.md) | F1–F6 + `make schema-validate` (region golden) | T-090.8 |
| GAP-002 LOD zoom space | [`t090_render_lod_contract.md`](t090_render_lod_contract.md) | `make t090-spec-verify` gate 3 | T-090.5 |
| GAP-003 hover/inspect UI | [`t090_9_world_object_interaction.md`](t090_9_world_object_interaction.md) | I1–I8 | T-090.9 |
| GAP-004 dual-pyramid manifest | `terrain-manifest.schema.json` + [`t090_1_aligned_basemap.md`](t090_1_aligned_basemap.md) | `make schema-validate` (dual + legacy goldens) | T-090.0.2 / .1.1 |
| GAP-005 Workbench feasibility | [`t090_3_0_workbench_export_spike.md`](t090_3_0_workbench_export_spike.md) | K1–K7 | T-090.3.0 |
| GAP-H1 Map source / synth | [`t090_basemap_dual_view.md`](t090_basemap_dual_view.md) (N9) | spike S4 | T-090.1.1 |
| GAP-H2 footprint vs OBB | N6 sentence (t090_2/.5/.6/glyphs + prefab schema) | N6 identical-sentence check | T-090.5 |
| GAP-H3 cluster reuse | [`t090_world_objects_worker.md`](t090_world_objects_worker.md) (separate world index) | gate 2 + W3 | T-090.5 |
| GAP-H4 worker offload | [`t090_world_objects_worker.md`](t090_world_objects_worker.md) | W1–W5 | T-090.5 |
| GAP-H5 persistence split | [`t090_basemap_dual_view.md`](t090_basemap_dual_view.md) (N8) | I (persistence) | T-090.1.1 |
| GAP-H6 legend | [`t090_9_world_object_interaction.md`](t090_9_world_object_interaction.md) | I6 | T-090.9 |
| GAP-H7 filter/search | [`t090_9_world_object_interaction.md`](t090_9_world_object_interaction.md) | I4 | T-090.9 |
| GAP-H8 per-phase budget | [`t090_phased_object_import.md`](t090_phased_object_import.md) (N11) | budget tables | T-090.3 / .8 |
| GAP-M1 manifest closed props | `terrain-manifest.schema.json` | `make schema-validate` | T-090.0.2 |
| GAP-M2 tile cache/storage | N10 table (basemap + pipeline) | identical-table check | T-090.1.1 |
| GAP-M3 Z-trust surfaced | [`t090_9_world_object_interaction.md`](t090_9_world_object_interaction.md) badge + [`t090_4_z_placement_audit.md`](t090_4_z_placement_audit.md) | I5 | T-090.9 |
| GAP-M4 road dashing | [`t090_render_lod_contract.md`](t090_render_lod_contract.md) (PathStyleExtension) | LOD table | T-090.5 |
| GAP-M5 enum drift | `map-object-enums.schema.json` | `make map-object-enums-verify` | T-090.0.2 |
| GAP-M6 AI context pack | [`t090_eden_ai_world_object_schema.md`](t090_eden_ai_world_object_schema.md) + region summaries | A5 | T-090.7 |
| GAP-M7 empty/export-not-run | [`t090_9_world_object_interaction.md`](t090_9_world_object_interaction.md) | I7 | T-090.9 |
| L1 atlas budget | [`t090_world_object_glyphs.md`](t090_world_object_glyphs.md) | G4 (atlas bounds) | T-090.5 |
| L2 rotation handedness + localUp→Z | [`t090_world_object_glyphs.md`](t090_world_object_glyphs.md) + [`t090_6_geometry_placement_audit.md`](t090_6_geometry_placement_audit.md) | spike K6 | T-090.3.0 |
| L3 type-inventory drives UI | [`t090_world_object_type_inventory.md`](t090_world_object_type_inventory.md) + [`t090_9_world_object_interaction.md`](t090_9_world_object_interaction.md) | I4 | T-090.9 |
| L4 accessibility color+shape | [`t090_9_world_object_interaction.md`](t090_9_world_object_interaction.md) + [`t090_render_lod_contract.md`](t090_render_lod_contract.md) | I6 | T-090.9 |
| L5 Arland empty state | [`t090_9_world_object_interaction.md`](t090_9_world_object_interaction.md) + [`t090_terrain_export_pipeline.md`](t090_terrain_export_pipeline.md) | I7 | T-090.9 |

---

## Verify commands (run on every doc/code pass)

```bash
make ticket-sync ticket-check-strict
make schema-validate          # golden missions + terrain manifest + anchors example
make verify-terrain           # stub OK — manifest ↔ terrains.ts + anchor schema
make verify-terrain-strict    # T-091.0 gate — GetSurfaceY plugin DEM + ≥10 anchors ±1 m
make map-census TERRAIN=everon   # pending_export until full T-090.3 export census
make ci-local-frontend        # frontend lint + build + unit tests (apps/website/frontend)
```

Scripts live in `packages/tbd-schema/scripts/verify-terrain-*.mjs`.

---

## Verified repo state (2026-06)

| Item | Today | Target |
|------|-------|--------|
| **Satellite / Map basemap views** | Grid + hillshade only | **T-090.1** Satellite + **T-090.1.1** Map — [`t090_basemap_dual_view.md`](t090_basemap_dual_view.md) |
| World objects | None on map | T-090.2–.5 taxonomy → export → render |
| Road/building/tree types | N/A | T-090.2 closed enums; **exact counts** in `type-inventory.json` (`censusStatus`; null until export) |

### Exact object counts (Everon)

**Authoritative when export runs:** `packages/map-assets/everon/objects/type-inventory.json` — validated by `map-object-type-inventory.schema.json` + `verify-type-inventory.mjs` (integer equality gates I1–I8).

**Today:** `censusStatus: "pending_export"` — all counts **null**. Do not verify against guesses.

| Doc | Purpose |
|-----|---------|
| [`t090_world_object_type_inventory.md`](t090_world_object_type_inventory.md) | Census contract + Everon baseline table |
| [`.ai/artifacts/everon_object_count_baseline.md`](../../../.ai/artifacts/everon_object_count_baseline.md) | Human-readable mirror (updated @ first census) |

```bash
make map-census TERRAIN=everon    # after export — writes/validates exact integers
make schema-validate              # includes verify-type-inventory
```
| Z burial audit | N/A | T-090.4 pivot + **T-090.6** OBB visibility @ 1M |
| DEM loader | **`dem/*` + `sampleElevation()`** @ `2c56c2e` — Everon loads in editor; API not wired to toolbelt/slots yet | T-091.1 **shipped** |
| Slot Z | `sampleElevation` in [`ydoc.ts`](../../../apps/website/frontend/src/features/tactical-map/state/ydoc.ts) | **Done (T-091.2)** @ `dde589e` |
| Toolbelt CUR/SEL Z | Sampled elevation @ 3 dp; X/Y @ 3 dp | **Done (T-091.2)** |
| DEM assets | **6400² PNG** @ `packages/map-assets/everon/dem/` | T-091.0 **shipped** |
| Everon bounds | 12800×12800 m | Biki confirmed |
| Everon altitude | [`terrains.ts`](../../../apps/website/frontend/src/features/tactical-map/coords/terrains.ts): −204.78…375.53 m | Manifest must match |
| Arland bounds | **4096×4096** m (fixed from wrong 10240) | Defer assets until Everon gate |

**Do not hard-code DEM pixel size** — record `widthPx`/`heightPx` from World Editor **Info & Diags** at export.

---

## Coordinate contract

| System | Horizontal | Vertical (T-091+) | Facing |
|--------|--------------|---------------------|--------|
| Editor Deck.gl | `position.x`, `position.y` (m; origin bottom-left; +y north) | `position.z` (m ASL) | `position.rotation` ° |
| Mod `slots[]` | `x`, `z` (**editor y → export z**) | optional `y` @ T-092 | `headingDeg` @ T-092 |

**Storage precision:** 0.001 m in UI/export floats. **Spawn authority:** mod `GetSurfaceY` + capsule offset (T-092) — not DEM alone.

---

## Asset layout

```text
packages/map-assets/
  terrain-registry.json        # all maps — add row, run make map-export
  {terrainId}/                 # everon, arland, … — identical layout
    manifest.json
    dem/
    tiles/satellite/{z}/{x}/{y}.webp   # aerial / SAP view
    tiles/map/{z}/{x}/{y}.webp         # cartographic map view
    objects/prefabs.json.gz    # taxonomy + ai metadata (deduped types)
    objects/chunks/{cx}_{cy}.json.gz
    objects/roads.json.gz
    objects/type-inventory.json
    glyphs/manifest.json           # iconKey → SVG + atlas (rotatable/scalable symbols)
    glyphs/svg/*.svg
    glyphs/atlas/world-glyphs.webp
    objects/z-audit.json
    anchors/verification.json
  .ai/artifacts/map_export_{terrainId}.json   # AI ops log (repo root)
```

Dev serve: `apps/website/frontend/public/map-assets/` → symlink or copy (DEV_RUNBOOK §Map assets).

Schemas: [`terrain-manifest.schema.json`](../../../packages/tbd-schema/schema/terrain-manifest.schema.json) · [`terrain-anchors.schema.json`](../../../packages/tbd-schema/schema/terrain-anchors.schema.json)

---

## T-091.0 ops log (shipped reference)

See [`.ai/artifacts/t091_0_ops_log.txt`](../../../.ai/artifacts/t091_0_ops_log.txt) @ `6d96339`. Re-export template:

```text
Date:
Workbench version:
Plugin: TBD_TerrainExportPlugin.c (GetSurfaceY resample)
Grid: 6400×6400 @ 2 m
DEM sha256:
make verify-terrain-strict: PASS (maxDeltaM, anchor count)
Tiles: deferred (T-090.1)
```

Full runbook: [`t091_0_dem_tile_export.md`](t091_0_dem_tile_export.md).

---

## Acceptance checklist (program-level)

Automated sign-off @ T-091.0: Claude Code completes **A1–A11** in [`t091_0_dem_tile_export.md`](t091_0_dem_tile_export.md) (`make verify-terrain-strict` exit 0). Code slices add **S/M** gates in their own specs.

---

## Related

- [`t092_spawn_transform_program.md`](t092_spawn_transform_program.md)
- [`t071_orbat_manager_program.md`](t071_orbat_manager_program.md)
- [`engineering_plan.md`](engineering_plan.md) §4.2–§4.3
- [`DEV_RUNBOOK.md`](../../website/DEV_RUNBOOK.md) §Map assets
