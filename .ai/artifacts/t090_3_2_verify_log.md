# T-090.3.2 — Density grids (TBDD) + PH-P2 trees + forest regions: verify log

**Slice:** T-090.3.2 (PH-P2 trees + P2b forest regions + §3.3 density binary) · **Terrain:** everon
**Date:** 2026-07-05 · **Executor:** claude-code (no Workbench session — staged raw from T-090.3.1 reused)
**Plan:** `.ai/artifacts/t090_10_map_engine_v2_implementation_plan.md` §3.3 + §7 row T-090.3.2 ·
forest spec `t090_8_forest_vegetation_regions.md` (Path B) · phased spec `t090_phased_object_import.md`

## Result

**ALL GATES PASS.** Everon P2 cumulative catalog shipped: **361 prefabs / 507,467 instances / 270
chunks** (chunk gz aggregate **6,248 KB** — under the 10 MB LFS line → committed plain) —
**501,861 tree instances in 51 tree prefab types** on top of the unchanged 5,606 P1 buildings.
New artifacts: **625 `objects/density/{cx}_{cy}.bin`** TBDD grids (732,500 B total, plain) +
**`objects/forest-regions.json.gz`** (36 derived-hull regions, 43.6 KB). `roads.json.gz` untouched.
No raster passes, no `tiles/map/`, no frontend code.

## Classification (4 append-only rules; host-only, no re-export)

Everon trees use Latin species dirs (`Prefabs/Vegetation/Tree/t_picea_abies/…`) — the legacy
`Tree_Pinus`-style needles matched zero rows. Appended to `prefab-classify.json` (internal order:
debris **before** species, so stump/fallen variants never reach the species rules):

1. **prop/debris** — `Vegetation/Tree/Debris/`, `_stump_`, `_fallen`, `_branch_`, `_stem_`
   (38,306 instances). Deliberate: stumps/fallen trunks are ground debris, not standing trees —
   classing them `tree` would fail PH-P2-3 (heightM ≥ 2 for ≥95 % of tree prefabs) at ~34 % of types.
2. **tree/conifer** — `/t_picea_abies/`, `/t_pinus_sylvestris/` (19 types, 295,126).
3. **tree/deciduous** — 9 species dirs (betula/carpinus/malus/sorbus/alnus/prunus/salix/populus/tilia)
   (32 types, 206,735). Standing-dead `d`-suffix variants stay in their species class (soft cover — PH-P2-2 holds).
4. **rock/boulder** — `Prefabs/Rocks/` (36 granite types, 276,349 raw; total classified rock
   307,157 incl. prior `Rock_`/`Boulder`/`Cliff_` needle matches) — feeds the TBDD rock channel only;
   rocks are **not** imported instances before P4.

`unclassifiedRawTypes` 1,523 → **1,411** (−112 = 76 tree + 36 rock — exact).

## E1 — `make map-export TERRAIN=everon PHASE=P2_trees` → exit 0 (2.6 s)

```
build-world-objects: everon P2_trees — {"catalog":{"prefabCount":361,"instanceCount":507467,"chunkCount":270},
 "rawLineCount":1409998,"outOfBounds":0,"unclassifiedRawTypes":1411}
```

- **Density** (`fullExport.phases.P2_trees.density`): 625 files · 732,500 B ·
  `treeCornerSum` **501,861** == tree instances (builder-fatal identity) · `rockCornerSum` 307,157 ·
  `rockOutOfBounds` 0.
- **Forest regions** (`…forestRegions`): cellM 32 · densityThreshold 2 · minComponentCells 8 ·
  dominantShare 0.66 → 30,715 dense cells · 251 components → **36 kept regions** ·
  **496,693 assigned + 5,168 unassigned = 501,861 (F2 exact)** · dominant split: 8 conifer / 23 deciduous / 5 mixed.
- Census: `byKind.tree {51, 501861}` · `bySpeciesClass {conifer, deciduous}` ·
  `byRegionKind.forest {count 36, treeCount 496693}` · `unassignedTrees 5168` · `importPhaseMax P2_trees`.
- Manifest objects: + `regionsPath`, `densityPath: "objects/density"`, `densityCellM: 32`,
  `lod {schemaVersion, refZoom 3, gates{…§5 constants}}`, `importPhaseShipped [P1_buildings, P2_trees]`.

## E1b — `make map-verify-phase TERRAIN=everon PHASE=P2_trees` → exit 0 (24 gates, 6.8 s)

```
G1 G2 G3 G12 G5 G6 G8 G7 G9 G10 G11  — catalog-scope invariants   ALL PASS
PH-P2-1  cumulative P1+P2; kinds ⊆ {building,tree}; trees present  PASS
PH-P2-2  tree prefabs cover=soft (dead exception)                  PASS
PH-P2-3  heightM >= 2 for >= 95% of tree prefabs                   PASS (100%)
PH-P2-4  G11 count conservation for kind=tree only                 PASS
PH-P2-5  density insert identity (Σ global tree corners = trees)   PASS   ← export-side analog of
                                                                      the spec's "rbush insert count
                                                                      = instance count" (render-side
                                                                      rbush lands in T-090.5.3)
D1  625 density files, TBDD header + 1,172 B size exact            PASS
D2  density byte-identical to recompute (committed chunks + raw)   PASS
F1  36 region rows validate map-object-region.schema.json          PASS
F2  forest.treeCount + unassignedTrees = byKind.tree.instances     PASS (exact)
F6  Path B re-derivation from committed chunks byte-identical      PASS
R-P1 roads · SIZE ≤ 40 MB · E6 determinism                         PASS
```

## No-regression — `make map-verify-phase TERRAIN=everon PHASE=P1_buildings` → exit 0 (19 gates)

Phase-scope split (new): phases are cumulative, so G11 + P-gates now filter committed rows/prefabs
to the REQUESTED phase's kinds; catalog-scope gates (G1–G10, G12, SIZE) run on the whole artifact
set; **E6 rebuilds at the committed `manifest.objects.importPhaseMax`** (a P1 re-verify on the P2
catalog byte-compares against a P2 rebuild). P1-1 additionally asserts catalog kinds ⊆ the
committed importPhaseMax kind set; P1-6's denominator fixed to `byKind.building.instances`
(cumulative totalInstances would dilute it). P1-4 anchors (38) resolve ≤ 2 m through the mixed catalog.

## Remaining acceptance gates

- `make schema-validate` → **exit 0** — incl. new **S13** (TBDD fixture: encode byte-identity +
  header contract + expected corners, `golden/map-objects/density/density-fixture.{json,bin}`) and
  **S14** (derivation fixture with hole ring + mixed-dominant + below-min blob,
  `golden/map-objects/regions-derivation-fixture.json`); new catalog bundle
  `golden/map-objects/phased/P2-trees.json` rides S2–S7 (5-tuple instances).
- `make map-export-validate` → **exit 0** — new committed-only checks: 625 density bins header-valid
  + **tree channel == recompute from committed chunks** (rock byte-compare needs staging → D2 only);
  regions rows Ajv + F2 vs inventory; `lib/density-grid.mjs` + `lib/forest-regions.mjs` added to the
  E2c literal-terrain-id scan (clean).
- `make map-census TERRAIN=everon` → exit 0 (censusStatus=partial; verify-type-inventory F-count/I3
  green on real bySpeciesClass/byRegionKind ints).
- `bash scripts/map-assets/verify-spike-all.sh` → **ALL PASS** (ops log `fullExport.phases.P2_trees`
  APPENDED; spike `probes.*` keys untouched).

## Deviations / notes

- **prefabId renumbering:** cumulative rebuild re-sorts the combined resourceName set, so P1
  building prefabIds shifted (310 rows unchanged in content, new ids). Legal: no consumer exists
  yet (T-090.5.x unbuilt); G4 = byte-stable across re-export of the same phase set — E6 PASS.
- **Rock channel semantics:** TBDD channel 1 counts **classified raw** rock rows (307,157) — rocks
  are density aggregates, not imported instances (P4 owns rock import). Verify: D2 recomputes the
  rock grid from the staged raw; the CI-safe validator checks header + tree channel only.
- **Mega-region observation (operator/T-090.8):** threshold 2 trees/cell + 8-connectivity merges
  Everon's central forest into one region — `forest-everon-001` = 478,749 trees / 2,944.31 ha /
  911 rings (outer + 910 clearing holes). Gates don't mandate granularity; render-slice tuning
  (4-connectivity or higher threshold in `lib/forest-regions.mjs` constants) can split it later
  without schema change — re-run export + re-freeze S14 fixture if constants change.
- **LFS decision (3.1 policy):** chunk gz aggregate 6.25 MB < 10 MB line → **plain commit** (no
  `.gitattributes` change). Density 732.5 KB + regions 43.6 KB plain per plan.
- **importPhaseShipped:** registry + manifest both `["P1_buildings","P2_trees"]` after all gates
  passed; `type-inventory.json` carries `importPhaseMax` only (schema has no shipped list).
- Builder perf: full 1.41M-row stream + classify + chunks + density + regions in **2.6 s**;
  P2 verify 6.8 s (incl. E6 double scratch build).

## Next

**T-090.5.1** render spine scaffold (after Cursor doc sync per single-lane rule). Rock import = P4;
`build-landcover-mask.mjs` now formally superseded by density grids (freeze → retire per plan §8).
