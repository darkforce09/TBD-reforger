# T-090 / T-091 — Map & terrain program (hub)

**Status:** **active** — **T-090.1** (aligned basemap). **T-091 complete** @ `dde589e` (`.0` @ `6d96339`, `.1` @ `2c56c2e`, `.2` @ `dde589e`).  
**Tickets:** T-090 · T-091 · **Route:** `/missions/:id/edit`  
**Registry:** [`.ai/tickets/registry.json`](../../../.ai/tickets/registry.json)  
**Spawn parity (separate hub):** [`t092_spawn_transform_program.md`](t092_spawn_transform_program.md)

**Hard gate:** **T-091.0** anchor verify **PASS** (`make verify-terrain-strict` @ `6d96339`). T-071 ORBAT / T-068 Phase 2 loadout still blocked on **T-092.2** mod compile + spawn verify.

---

## Program order

```text
T-090.0  hub + schema + verify scripts (shipped)
  → T-091.0  Everon DEM + anchors (shipped @ 6d96339)  ✓
  → T-091.1  DEM loader (shipped @ 2c56c2e)  ✓
  → T-091.2  Z UX + hillshade (claude-code)  ✅ shipped @ dde589e
  → T-090.1  aligned basemap tiles (claude-code)  ← ACTIVE NOW
  → T-092    mod compile + spawn
  → T-071 → T-068.13 → T-068.7+
  → T-121    tiles / Arland re-export / MCP polish (deferred)
```

---

## Slice specs (read these — not optional)

Each slice has its **own spec file** with locked decisions, file touch list, and **mandatory verification gate** (automated commands + acceptance table).

| Slice | Spec | Executor | Exit gate |
|-------|------|----------|-----------|
| **T-090.0** | [`t090_0_map_program_hub.md`](t090_0_map_program_hub.md) | cursor-docs | `make ticket-check-strict` + `make verify-terrain` + `make schema-validate` |
| **T-091.0** | [`t091_0_dem_tile_export.md`](t091_0_dem_tile_export.md) | claude-code | **shipped** @ `6d96339` — `make verify-terrain-strict` PASS |
| **T-090.1** | [`t090_1_aligned_basemap.md`](t090_1_aligned_basemap.md) | claude-code | Build/lint + horizontal H1/H2 manual log |
| **T-091.1** | [`t091_1_dem_loader.md`](t091_1_dem_loader.md) | claude-code | **shipped** @ `2c56c2e` — S1–S10 PASS (15 vitest, verify-terrain-strict) |
| **T-091.2** | [`t091_2_z_axis_editor.md`](t091_2_z_axis_editor.md) | claude-code | **shipped** @ `dde589e` |
| **T-090.1** | [`t090_1_aligned_basemap.md`](t090_1_aligned_basemap.md) | claude-code | Aligned WebP tiles — **active** |

---

## Verify commands (run on every doc/code pass)

```bash
make ticket-sync ticket-check-strict
make schema-validate          # golden missions + terrain manifest + anchors example
make verify-terrain           # stub OK — manifest ↔ terrains.ts + anchor schema
make verify-terrain-strict    # T-091.0 gate — GetSurfaceY plugin DEM + ≥10 anchors ±1 m
cd apps/website/frontend && npm run build && npm run lint
```

Scripts live in `packages/tbd-schema/scripts/verify-terrain-*.mjs`.

---

## Verified repo state (2026-06)

| Item | Today | Target |
|------|-------|--------|
| Basemap | Procedural grid [`useBaseMapLayer.ts`](../../../apps/website/frontend/src/features/tactical-map/layers/useBaseMapLayer.ts) | T-090.1 aligned tiles |
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
  everon/
    manifest.json              # terrain-manifest.schema.json
    dem/everon-dem-16bit.png   # Git LFS — T-091.0 shipped
    tiles/{z}/{x}/{y}.webp     # Git LFS — T-090.1 / T-121 (deferred)
    anchors/verification.json  # terrain-anchors.schema.json
    anchors/verification.example.json
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
