# T-090 — Phased object import + mathematical verification

**Status:** Spec ready — **gates T-090.3 export and T-090.5 render**  
**Authority:** [`t090_091_map_terrain_program.md`](t090_091_map_terrain_program.md)  
**Schema:** [`t090_eden_ai_world_object_schema.md`](t090_eden_ai_world_object_schema.md)

---

## In one sentence

Import world objects **one kind at a time** (buildings → trees → … → footpaths), with **automated mathematical proofs** per phase — **no eyeball sign-off**, no advancing until `make map-verify-phase` exits **0**.

---

## Rule (locked)

| ❌ Forbidden | ✅ Required |
|-------------|------------|
| Import full 1M catalog on first try | Enable **one import phase** at a time |
| “Looks aligned in the editor” | **`make map-verify-phase PHASE=Pn` exit 0** |
| Manual spot-check only | **Deterministic scripts + JSON Schema + count identities** |
| Skip phase because next slice is ready | Phase **N+1 blocked** in registry until phase **N** shipped |

**110% mathematically verifiable** = every acceptance criterion is a **computable predicate** (script or schema validator), not human judgment.

---

## Why buildings first (recommended P1)

| Candidate | Pros | Cons |
|-----------|------|------|
| **Buildings (P1)** ✓ | Large footprints → easy alignment check vs basemap; fewer count than trees; `gameplay.cover=hard` unambiguous; Eden AI value immediate | More complex geometry |
| Trees first | Highest volume stress test early | Millions of rows obscure alignment bugs; soft cover defaults noisier |
| Roads first | Network topology | Polylines harder to validate before point props work |

**Locked order:** **P1 buildings** → P2 trees → P3 bushes → P4 rocks → P5 pebbles/props → P6–P9 roads (by class).

---

## Import phases (normative sequence)

Each phase adds **one `kind` or `roadClass` filter** to export + render + verify. Prior phases stay enabled (cumulative).

| Phase | ID | Export filter | Render layer | Golden fixture |
|-------|-----|---------------|--------------|----------------|
| **P1** | `P1_buildings` | `kind=building` | `world-buildings` + footprint polygons; optional `building-*` badge glyphs | `golden/P1-buildings.json` |
| **P2** | `P2_trees` | `kind=tree` | `world-trees` | `golden/P2-trees.json` |
| **P3** | `P3_vegetation` | `kind=vegetation` | `world-vegetation` | `golden/P3-vegetation.json` |
| **P4** | `P4_rocks` | `kind=rock` | `world-rocks` | `golden/P4-rocks.json` |
| **P5** | `P5_props` | `kind=prop` (incl. pebbles) | `world-props` | `golden/P5-props.json` |
| **P6** | `P6_roads_highway` | `roadClass=highway_paved` | `world-roads` | `golden/P6-highway.json` |
| **P7** | `P7_roads_paved` | `roadClass=road_paved` | + same layer | `golden/P7-road-paved.json` |
| **P8** | `P8_roads_dirt` | `roadClass=road_dirt`, `track` | + same layer | `golden/P8-dirt-track.json` |
| **P9** | `P9_roads_path` | `roadClass=path` | + same layer | `golden/P9-path.json` |
| **P10** | `P10_full` | all kinds (full Eden) | all toggles | full sample subset |

**Registry field** (`terrain-registry.json` per terrain):

```json
{
  "terrainId": "everon",
  "importPhaseMax": "P1_buildings",
  "importPhaseShipped": ["P1_buildings"]
}
```

`importPhaseMax` = highest phase **allowed to run**. Advance only after verify PASS + human `./scripts/ticket advance-slice` or registry bump.

---

## Commands (per phase)

```bash
# Export ONLY objects matching current phase (cumulative kinds per table)
make map-export TERRAIN=everon PHASE=P1_buildings

# Mathematical verification — MUST exit 0 before next phase
make map-verify-phase TERRAIN=everon PHASE=P1_buildings

# Render smoke (T-090.5) — optional automated screenshot diff @ fixed camera
make map-render-verify TERRAIN=everon PHASE=P1_buildings
```

Implementation: `export-terrain.sh everon --phase P1_buildings` filters raw entities **before** classify/chunk.

---

## Global mathematical invariants (every phase)

These run on **every** `map-verify-phase` call — phase-specific checks add on top.

| ID | Invariant | Formula / method | Pass |
|----|-----------|------------------|------|
| **G1** | Schema valid | `map-object-catalog.schema.json` + `map-object-prefab.schema.json` | exit 0 |
| **G2** | Resolved valid | Materialize all instances → `map-object-resolved.schema.json` | exit 0 |
| **G3** | prefabId bijection | `∀ inst: 0 ≤ inst.prefabId < prefabs.length` | script |
| **G4** | prefabId deterministic | Sort by `resourceName` → ids stable across re-export | byte-identical prefabs gzip |
| **G5** | Instance id unique | `|ids| = |instances|` | set size |
| **G6** | Chunk partition | `∀ inst: chunk(cx,cy) contains (inst.x, inst.y)` | script |
| **G7** | Chunk count sum | `Σ cell.instanceCount = manifest.instanceCount` | integer eq |
| **G8** | World bounds | `∀ inst: 0 ≤ x ≤ maxX`, `0 ≤ y ≤ maxY` (terrain bounds) | script |
| **G9** | gameplay.cover enum | `cover.type ∈ {none, soft, hard}` | script |
| **G10** | spatial positive | `heightM ≥ 0`, `halfExtentsM.* ≥ 0` (where applicable) | script |
| **G11** | Raw ↔ catalog count | `count_raw(filter) = count_catalog(filter)` for phase filter | integer eq |
| **G12** | No orphan prefabs | Every prefabId referenced ≥1 instance **or** flagged `prefabOnly: true` in rules | script |

**No manual steps in G1–G12.**

---

## Phase-specific mathematical gates

### P1 — Buildings

| ID | Check | Pass |
|----|-------|------|
| P1-1 | `∀ prefab: kind=building` | 100% |
| P1-2 | `∀ prefab: gameplay.cover.type=hard` (unless tagged `ruin-open`) | ≥99.5% + explicit exception list |
| P1-3 | `∀ inst: spatial.footprintM2 > 0` or OBB volume > 0 | script |
| P1-4 | Sample **K=32** grid anchors: building centroid within **≤2 m** of nearest exported instance with `kind=building` at anchor (synthetic golden) | test fixture |
| P1-6 | `type-inventory.byBuildingClass.*` populated; `unknown` < 0.5% of building instances | script |

### P2 — Trees

| ID | Check | Pass |
|----|-------|------|
| P2-1 | Cumulative: P1 + P2 instances; P2 filter `kind=tree` purity 100% | script |
| P2-2 | `∀ tree prefab: gameplay.cover.type=soft` (unless `dead`) | script |
| P2-3 | `spatial.heightM ≥ 2` for ≥95% of tree prefabs | percentile script |
| P2-4 | Count conservation G11 for `kind=tree` only | integer eq |
| P2-5 | Cluster index: rbush insert count = instance count | integer eq |

### P3 — Vegetation (bushes)

| ID | Check | Pass |
|----|-------|------|
| P3-1 | `kind=vegetation` purity 100% | script |
| P3-2 | `cover.type ∈ {soft, none}` | script |
| P3-3 | G11 for vegetation filter | integer eq |

### P4 — Rocks

| ID | Check | Pass |
|----|-------|------|
| P4-1 | `kind=rock` purity 100% | script |
| P4-2 | `cover.type=hard` for ≥90% | script |
| P4-3 | G11 for rock filter | integer eq |

### P5 — Props / pebbles

| ID | Check | Pass |
|----|-------|------|
| P5-1 | `kind=prop` purity 100% | script |
| P5-2 | Default `cover.type=none` unless rule override | script |
| P5-3 | G11 for prop filter | integer eq |

### P6–P9 — Roads (by class)

Roads use **`roadSegments[]`** + count conservation on segment ids.

| ID | Check | Applies to |
|----|-------|------------|
| P6-1 | `roadClass=highway_paved` purity 100% | P6 |
| P6-2 | Polyline length > 0: `Σ segment |points| ≥ 2` | P6–P9 |
| P6-3 | `spatial.widthM > 0` per road prefab/segment | P6–P9 |
| P6-4 | G11 segment count for filter | P6–P9 |
| P7-1 | `roadClass=road_paved` purity | P7 |
| P8-1 | `roadClass ∈ {road_dirt, track}` purity | P8 |
| P9-1 | `roadClass=path` purity | P9 |

### P10 — Full catalog

| ID | Check | Pass |
|----|-------|------|
| P10-1 | All phases P1–P9 shipped in registry | registry |
| P10-2 | G11 for **unfiltered** full raw export | integer eq |
| P10-3 | T-090.4 + T-090.6 audits complete | audit JSON present |
| P10-4 | Performance: stream full catalog verify < 45 min | ops log |

---

## Per-phase budgets (N11)

Each phase carries a render + load + memory budget at deckZoom −2, not just export math:

| Phase | instances (from inventory) | max load ms | max resident MB | min fps @ −2 | eviction |
|-------|----------------------------|-------------|-----------------|--------------|----------|
| P1 buildings | `byKind.building.instances` | 2000 | 40 | 55 | chunk LRU 4 |
| P2 trees | `byKind.tree.instances` | 8000 | 180 | 55 | **forest regions required**; chunk LRU 8 |
| P2b forest regions | derived | 3000 | +20 | 55 | region index pinned |
| P5 + roads | `byKind.road.segments` | 10000 | 200 | 55 | roads whole-file OK |
| P10 full | `levels.totalInstances` | 15000 | 256 | 50 | chunk LRU + region index + worker-only parse |

**P2b** ([`t090_8_forest_vegetation_regions.md`](t090_8_forest_vegetation_regions.md)) runs immediately
after P2: `forest.treeCount + unassignedTrees = byKind.tree.instances` (exact). Until `censusStatus` ≠ `pending_export`, phase budgets reference **inventory integers**, not hard-coded ~900k.
The **P10 residency model** (chunk LRU + region index + worker-only parse) is specified **in T-090**, not
deferred to T-110.

## Z / geometry audits (phase-gated)

| Phase | T-090.4 pivot audit | T-090.6 OBB audit |
|-------|---------------------|-------------------|
| P1–P5 | Run on **phase filter only** | Run on **phase filter only** |
| P6–P9 | Roads: sample points every 4 m | Skip OBB (line model) |
| P10 | Full catalog | Full catalog |

Audit formulas unchanged — see [`t090_4_z_placement_audit.md`](t090_4_z_placement_audit.md), [`t090_6_geometry_placement_audit.md`](t090_6_geometry_placement_audit.md). Verify scripts **recompute** `zDeltaM` on sample and compare to stored audit (detect stale reports).

---

## Render verification (T-090.5, per phase)

Optional but automated where possible:

| ID | Check | Method |
|----|-------|--------|
| R1 | Layer visible | Deck layer `world-*` instance count > 0 |
| R2 | Layer purity | Pick **N=100** random instances → all match phase `kind`/`roadClass` |
| R3 | No leak from next phase | Instances with kinds **not yet imported** = 0 in store |

No screenshot judgment in CI v1 — instance count + purity only. Screenshot diff = stretch.

---

## Deliverables (T-090.3 + T-090.5)

| # | Path |
|---|------|
| 1 | `scripts/map-assets/export-terrain.sh --phase Pn` |
| 2 | `scripts/map-assets/verify-phase.mjs` → `make map-verify-phase` |
| 3 | `packages/tbd-schema/golden/phased/P1-buildings.json` … `P9-path.json` |
| 4 | `packages/tbd-schema/scripts/verify-map-phase.mjs` |
| 5 | `terrain-registry.json` → `importPhaseMax`, `importPhaseShipped[]` |
| 6 | Vitest: golden + count identities |

---

## Program integration

```text
T-090.2   schema + phased golden fixtures
T-090.3   export --phase Pn (cumulative)
          map-verify-phase MUST pass before registry importPhaseMax bump
T-090.5   enable one Deck layer per shipped phase
T-090.7   AI queries respect importPhaseMax (no hallucinating unimported kinds)
P10       full 1M — only after P1–P9 all shipped
```

**Do not run `make map-export --all` for production Everon until P10.**

Development shortcut: `--phase P1_buildings` on a **Workbench subregion** or filtered export is allowed for faster iteration; full-map counts required to **ship** a phase.

---

## Related

- [`t090_terrain_export_pipeline.md`](t090_terrain_export_pipeline.md)
- [`t090_2_map_object_taxonomy.md`](t090_2_map_object_taxonomy.md)
- [`t090_5_map_object_render_layer.md`](t090_5_map_object_render_layer.md)
- [`t090_eden_ai_world_object_schema.md`](t090_eden_ai_world_object_schema.md)
