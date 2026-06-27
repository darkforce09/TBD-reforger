# T-090 / T-091 — Map & terrain program (hub)

**Status:** **ACTIVE** — **T-090.0** (spec + manifest schema + verify pipeline).  
**Tickets:** T-090 · T-091 · **Route:** `/missions/:id/edit`  
**Registry:** [`.ai/tickets/registry.json`](../../../.ai/tickets/registry.json)  
**Spawn parity (separate hub):** [`t092_spawn_transform_program.md`](t092_spawn_transform_program.md)

**Hard gate:** No T-071 ORBAT, T-068 Phase 2 loadout, or mod LOBBY slot picker until **T-091.0 anchor verify PASS** (`make verify-terrain-strict`) and **T-092.2** mod compile ship.

---

## Program order

```text
T-090.0  hub + schema + verify scripts (cursor-docs)     ← ACTIVE NOW
  → T-091.0  human: DEM + tiles + anchors + strict verify
  → T-090.1  Cartesian basemap (claude-code)
  → T-091.1  DEM loader (claude-code)
  → T-091.2  Z UX + hillshade (claude-code)
  → T-092    mod compile + spawn — t092_spawn_transform_program.md
  → T-071 → T-068.13 → T-068.7+
```

---

## Slice specs (read these — not optional)

Each slice has its **own spec file** with locked decisions, file touch list, and **mandatory verification gate** (automated commands + acceptance table).

| Slice | Spec | Executor | Exit gate |
|-------|------|----------|-----------|
| **T-090.0** | [`t090_0_map_program_hub.md`](t090_0_map_program_hub.md) | cursor-docs | `make ticket-check-strict` + `make verify-terrain` + `make schema-validate` |
| **T-091.0** | [`t091_0_dem_tile_export.md`](t091_0_dem_tile_export.md) | human | `make verify-terrain-strict` + A1–A10 checklist |
| **T-090.1** | [`t090_1_aligned_basemap.md`](t090_1_aligned_basemap.md) | claude-code | Build/lint + horizontal H1/H2 manual log |
| **T-091.1** | [`t091_1_dem_loader.md`](t091_1_dem_loader.md) | claude-code | Unit tests 3 pixels ±0.01 m |
| **T-091.2** | [`t091_2_z_axis_editor.md`](t091_2_z_axis_editor.md) | claude-code | Manual M1–M7 + version payload Z |

---

## Verify commands (run on every doc/code pass)

```bash
make ticket-sync ticket-check-strict
make schema-validate          # golden missions + terrain manifest + anchors example
make verify-terrain           # stub OK — manifest ↔ terrains.ts + anchor schema
make verify-terrain-strict    # T-091.0 human gate — DEM + ≥10 anchors ±1 m
cd apps/website/frontend && npm run build && npm run lint
```

Scripts live in `packages/tbd-schema/scripts/verify-terrain-*.mjs`.

---

## Verified repo state (2026-06)

| Item | Today | Target |
|------|-------|--------|
| Basemap | Procedural grid [`useBaseMapLayer.ts`](../../../apps/website/frontend/src/features/tactical-map/layers/useBaseMapLayer.ts) | T-090.1 aligned tiles |
| Slot Z | `z: 0` in [`ydoc.ts`](../../../apps/website/frontend/src/features/tactical-map/state/ydoc.ts) | T-091.2 `sampleElevation` |
| DEM assets | Stub manifest only | T-091.0 export |
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
    dem/everon-dem-16bit.png   # Git LFS — T-091.0
    tiles/{z}/{x}/{y}.webp     # Git LFS — T-091.0
    anchors/verification.json  # terrain-anchors.schema.json
    anchors/verification.example.json
```

Dev serve: `apps/website/frontend/public/map-assets/` → symlink or copy (DEV_RUNBOOK §Map assets).

Schemas: [`terrain-manifest.schema.json`](../../../packages/tbd-schema/schema/terrain-manifest.schema.json) · [`terrain-anchors.schema.json`](../../../packages/tbd-schema/schema/terrain-anchors.schema.json)

---

## T-091.0 ops log template

```text
Date:
Workbench version:
Everon world path (exact):
Info & Diags — planar resolution (m):
Info & Diags — height min/max (m):
Info & Diags — heightmap widthPx × heightPx:
Export — Base PNG max anchor error (m):
Export — Modified PNG max anchor error (m):
Chosen dem.source:
H1 origin check:
H2 landmark @ 3 zoom levels:
make verify-terrain-strict: pass/fail
Git LFS push: yes/no
```

Full runbook: [`t091_0_dem_tile_export.md`](t091_0_dem_tile_export.md).

---

## Acceptance checklist (program-level)

Human sign-off @ T-091.0 completes items **A1–A10** in [`t091_0_dem_tile_export.md`](t091_0_dem_tile_export.md). Code slices add **S/M** gates in their own specs.

---

## Related

- [`t092_spawn_transform_program.md`](t092_spawn_transform_program.md)
- [`t071_orbat_manager_program.md`](t071_orbat_manager_program.md)
- [`engineering_plan.md`](engineering_plan.md) §4.2–§4.3
- [`DEV_RUNBOOK.md`](../../website/DEV_RUNBOOK.md) §Map assets
