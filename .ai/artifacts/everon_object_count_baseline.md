# Everon world-object count baseline

**Status:** `pending_export` — **no authoritative integers yet.**

Exact prefab and instance counts for Everon are **unknown until** the T-090.3.0 Workbench spike (K1) and the first `make map-census TERRAIN=everon` run after classify. Until then:

- Do **not** cite order-of-magnitude ranges in verification gates, UI copy, or phase budgets.
- Do **not** accept "close enough" or rounded totals for world-object acceptance.

## Source of truth (once export runs)

| Artifact | Path |
|----------|------|
| Machine-readable census | `packages/map-assets/everon/objects/type-inventory.json` |
| Agent copy | `.ai/artifacts/type_inventory_everon.json` (written by `make map-census`) |
| Manifest mirror | `packages/map-assets/everon/manifest.json` → `objects.instanceCount`, `objects.prefabCount` |

## Required fields after first census (exact integers)

| Field | Meaning |
|-------|---------|
| `levels.totalInstances` | Total world-object placements on Everon (e.g. `1_100_112`) |
| `levels.uniquePrefabs` | Distinct `resourceName` count |
| `byKind.building.instances` | Building placements (exact) |
| `byKind.tree.instances` | Tree placements (exact) |
| `byKind.vegetation.instances` | Bush/grass/etc. (exact) |
| `byKind.rock.instances` | Rocks (exact) |
| `byKind.prop.instances` | Props (exact) |
| `byKind.utility.instances` | Utilities (exact) |
| `byKind.water.instances` | Water structures (exact) |
| `byKind.road.segments` | Road polylines (exact) |
| `byBuildingClass.*.instances` | Per building subtype (exact; must sum to `byKind.building.instances`) |
| `bySpeciesClass.*.instances` | Per tree/vegetation subtype (exact where populated) |

## Verification

```bash
make map-census TERRAIN=everon          # after export — must exit 0
make schema-validate                    # runs verify-type-inventory.mjs (I1–I6)
make map-verify-phase TERRAIN=everon PHASE=Pn   # phase gates use same integers
```

**Mathematical rule:** `Σ byKind.*.instances = levels.totalInstances` — **integer equality**, not ±2% (the only ±2% rule in the program is legacy forest-hull *provisional* assignment during P2 development; shipped gates use exact `forest.treeCount + unassignedTrees = byKind.tree.instances`).

Spec: [`docs/specs/Mission_Creator_Architecture/t090_world_object_type_inventory.md`](../../docs/specs/Mission_Creator_Architecture/t090_world_object_type_inventory.md)
