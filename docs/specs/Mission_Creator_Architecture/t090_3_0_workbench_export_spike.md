# T-090.3.0 — Workbench world-export feasibility spike

**Ticket:** T-090 · **Slice:** T-090.3.0
**Status:** **shipped** @ `b342c35` — enumeration proven (K1/K1b/K2/K5/K6/K7 pass). **K3 red** (satellite tile = T-090.1 work). **K4 red** on gate; real `.topo` cartographic source found → T-090.1.1 de-risked. Ops log: [`.ai/artifacts/map_export_everon.json`](../../../.ai/artifacts/map_export_everon.json).
**Executor:** **claude-code** (+ one-time Workbench focus by human if the GUI must be driven)
**Authority:** [`t090_091_map_terrain_program.md`](t090_091_map_terrain_program.md)

---

## In one sentence

Before committing to the full P1–P10 object export, **prove on a small Everon subregion** that the
Reforger Workbench can actually (a) enumerate world entities, (b) read per-prefab bounds, (c) capture a
Satellite **and** a Map tile, and (d) expose (or not) a vegetation/forest mask — so the taxonomy, OBB
audit, dual basemap and forest specs rest on verified engine capability, not assumption.

---

## Why a spike (GAP-005)

T-091.0 already learned the hard way that the obvious path (manual World Editor **Export Height Map**)
was **dead on packed Eden** and needed a `GetSurfaceY` resample workaround. The object program leans on
the same class of unproven capability: enumerate ~1M entities, read OBB (`GetBounds()` "or equivalent"),
capture ortho + cartographic rasters, and find a forest mask. If any is absent, `spatial`/`gameplay`
(T-090.2), the geometry audit (T-090.6), the Map basemap (T-090.1.1) and forest path A (T-090.8) change
shape. **Prove first, on a subregion, cheap.**

---

## Spike tasks (each produces a committed artifact)

| # | Probe | Proves | Fallback if absent |
|---|-------|--------|--------------------|
| S1 | Enumerate world entities in a ~512 m subregion → `raw-entities.jsonl` | entity iteration API exists; ≥1 real building row with `resourceName` + transform | escalate — blocks P1 |
| S2 | Read per-prefab **OBB** (half-extents + yaw) for the S1 rows | `spatial.halfExtentsM` is real, not kind-default (N6/T-090.6) | OBB-only via kind defaults; footprint rings dropped (N6 conditional) |
| S3 | Capture **1 Satellite** tile (ortho/SAP) for a known cell | T-090.1 source path | escalate |
| S4 | Capture **1 Map** tile (cartographic) for the same cell **or** document absence | T-090.1.1 source path | **synthesized-cartographic** per N9 (DEM hillshade + land-cover + baked roads) |
| S5 | Probe a vegetation/**forest mask** / generator layer | T-090.8 path A (`source: engine-mask`) | T-090.8 path B derived-hull (mandatory fallback) |
| S6 | Read rotation + bounds and record **handedness + localUp→world Z** remap (L2) | glyph rotation + OBB axis remap correctness | document measured remap; consumed by glyphs + T-090.6 |

The plugin is `TBD_TerrainWorldExportPlugin.c` (also used by T-090.3). The spike exercises a **subregion**
only — full-map export stays blocked until P1.

---

## Exit gate (before P1 / T-090.1)

```bash
# Spike artifacts present + the catalog rows they produce validate
make schema-validate
```

| ID | Check | Pass |
|----|-------|------|
| K1 | `raw-entities.jsonl` has ≥1 building with resourceName + transform (S1) | file + script |
| K1b | Subregion census written to staging `type-inventory-spike.json` with **exact** integer row counts (not estimates) | file + `verify-type-inventory.mjs` |
| K2 | At least one S1 building has a real OBB (S2) **or** the ops log states OBB unavailable + the kind-default decision | ops log |
| K3 | One Satellite tile saved for a known cell (S3) | file |
| K4 | One Map tile saved **or** ops log records "synthesized-cartographic required" (S4/N9) | file or ops log |
| K5 | Forest-mask probe result recorded: `engine-mask` available, or `derived-hull` mandated (S5/N5) | ops log |
| K6 | Handedness + localUp→Z remap measured and written for glyphs + audit (S6/L2) | ops log |
| K7 | Ops log `.ai/artifacts/map_export_everon.json` written; an agent can read it + resolve 3 sample rows | manual |

**Spike slice done** when K1 + K1b + K7 pass (enumeration proven). **T-090.1 basemap ship** requires **K3 pass** (real Satellite tile on disk). K3/K4 may be red at spike completion — see ops log `gatesNote`.

---

## Artifacts

- `packages/map-assets/everon/staging/spike/raw-entities.jsonl` (gitignored staging)
- One `tiles/satellite/...` + one `tiles/map/...` sample tile (or synth note)
- `.ai/artifacts/map_export_everon.json` — ops log (probe results, API names actually used, remap)

---

## Out of scope
- Full P1–P10 export ([`t090_phased_object_import.md`](t090_phased_object_import.md)).
- Mod spawn (**T-092**). The spike is **visual/data feasibility only**.

## Related
- [`t090_3_map_asset_export.md`](t090_3_map_asset_export.md) · [`t090_terrain_export_pipeline.md`](t090_terrain_export_pipeline.md)
- [`t090_6_geometry_placement_audit.md`](t090_6_geometry_placement_audit.md) · [`t090_8_forest_vegetation_regions.md`](t090_8_forest_vegetation_regions.md)
- [`t091_0_dem_tile_export.md`](t091_0_dem_tile_export.md) — the prior "prove the engine path" precedent
