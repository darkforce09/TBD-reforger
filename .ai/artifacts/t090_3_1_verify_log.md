# T-090.3.1 — Map Engine v2 export core: verify log

**Slice:** T-090.3.1 (PH-P1 buildings + roads pulled forward per plan Q1) · **Terrain:** everon
**Date:** 2026-07-05 · **Executor:** claude-code (Workbench script reload = operator, one click)
**Plan:** `.ai/artifacts/t090_10_map_engine_v2_implementation_plan.md` §3 + §7 row T-090.3.1

## Result

**ALL GATES PASS.** Everon P1 catalog shipped: **310 building prefabs / 5,606 instances / 219 chunks**
(chunk gz aggregate **84 KB** — far under the 40 MB LFS line) + **roads.json.gz 766 segments**
(5 runway / 367 road_paved / 394 road_dirt, 170,112 points, class mapping provisional). No raster
passes, no `tiles/map/`, no frontend code.

## Workbench full-world export (plugin `TBD_TerrainWorldFullExportPlugin`)

- Compile: operator Script Editor reload — **Errors(0)** (screenshot in session; wb_reload/MCP menu
  compile paths all return ExecuteAction=false — known limitation, documented in export-terrain.sh).
- Run via MCP: `wb_execute_action '{"menuPath":"Plugins,TBD,Export TBD World Objects (full)"}'`
- script.log tail:
  ```
  [TBD][WorldFull] cell (24,24) hits 0 kept 0 (total kept 1409998)
  [TBD][WorldFull] DONE — kept 1409998 (withPrefab 1227337, aabbHits 1446744, oob 0) in 8888 ms
  ```
  25×25 = 625 cell passes over `QueryEntitiesByAABB` in **8.9 s**; `withPrefab` 1,227,337 ≈ the
  1,235,873 wb_state entity count (sanity ✓).
- Staging (completion-sentinel meta enforced):
  ```
  copy-world-export-profile: everon FULL — staged 1409998 rows → packages/map-assets/everon/staging/export/raw-entities.jsonl; meta + stagedAt stamp written
  ```
  Raw JSONL 339.7 MB, gitignored (`**/staging/`).

## E1 — `make map-export TERRAIN=everon PHASE=P1_buildings` → exit 0

```
build-world-objects: everon P1_buildings — {"rawLineCount":1409998,"rawUniqueResourceNames":2030,
 "noPrefab":{"count":182661,...},"outOfBounds":0,
 "catalog":{"prefabCount":310,"instanceCount":5606,"chunkCount":219},"unclassifiedRawTypes":1523}
build-roads-from-topo: everon — {"segments":766,"byClass":{"runway":5,"road_paved":367,"road_dirt":394},"points":170112}
```

Classification loop (host-only, no re-export): first census exposed the Everon prefab path taxonomy
(`Prefabs/Structures/<Category>/…`); appended **12 append-only rules** to `prefab-classify.json`
(path-based: Military/Fortifications→prop-barrier, Military/Bunkers→bunker, Military→military,
Agriculture→agricultural, Ruins→ruin, Cultural→civic, Services→civic, Industrial/Towers→tower,
Industrial→industrial, Commercial→commercial, Airport/Hangar→hangar, Airport/ControlTower→tower;
military + tower rules carry `render.importanceZoom: -4` per plan §3.4/Q9). Deliberately excluded
from P1 (props/line-work, not buildings): Structures/{Walls 36,185, Infrastructure 11,411,
Signs 4,831, BuildingParts 3,144, Forest 430, Civilian 254, Core 53, Recreation 10}.

byBuildingClass (all 9 populated, **zero `unknown`**): residential 3258 · agricultural 1050 ·
civic 1018 · industrial 133 · bunker 57 · tower 44 · commercial 32 · military 13 · hangar 1.

## E1b — `make map-verify-phase TERRAIN=everon PHASE=P1_buildings` → exit 0

```
  PASS  G1 — schema valid (prefabs, chunk rows, roads, inventory)
  PASS  G2 — all instances materialize to valid ResolvedWorldObject
  PASS  G3 — prefabId bijection (0 <= id < prefabs.length)
  PASS  G12 — no orphan prefabs
  PASS  G5 — derived instance ids unique (sidecar <-> files consistent)
  PASS  G6 — chunk partition (clamp(floor(coord/512)))
  PASS  G8 — world bounds 0 <= x,y <= maxX
  PASS  G7 — count identities (sidecar = files = manifest = inventory)
  PASS  G9 — gameplay.cover.type enum
  PASS  G10 — spatial positive (heightM, halfExtentsM)
  PASS  G11 — raw <-> catalog count parity for P1_buildings filter
  PASS  P1-1 — all prefabs kind=building
  PASS  P1-2 — cover=hard >= 99.5% (ruin-open exceptions allowed)
  PASS  P1-3 — footprint or OBB volume > 0 per prefab
  PASS  P1-4 — K=32 anchor sample <= 2 m via committed chunks (38 anchors)
  PASS  P1-6 — byBuildingClass populated; unknown < 0.5%
  PASS  R-P1 — roads present (segments > 0, polylines >= 2 points)
  PASS  SIZE — chunk gz aggregate <= 40 MB (forces LFS decision before P2)
  PASS  E6 — determinism — double scratch build byte-identical AND committed artifacts current (G4 + I6)

map-verify-phase: OK — everon P1_buildings (310 prefabs, 5606 instances, 219 chunks, 766 road segments, chunk gz 84 KB)
```

G4/E6/I6 collapse into the E6 double-scratch-build + committed byte-compare (plan decision 7:
canonical sorts, gzip level 9 mtime-0, `generatedAt`/`exportedAt` from the `stagedAt` stamp — never
wall clock). P1-4 runs the SAME `checkAnchors()` as the synthetic golden gate S12
(`scripts/map-assets/lib/anchor-check.mjs` — remap/partition re-implemented independently of the
builder, non-circular).

## Remaining acceptance gates

- `make schema-validate` → **exit 0** — `All contracts valid.` + S2–S9 + **S11** (chunk golden:
  5-tuple rows, half-open 512.0 boundary, sort order, importanceZoom coverage) + **S12** (anchor
  fixture) + 12 spec gates (verify-t090-specs, incl. new **gate 12 = INV-I8** budget-row grep).
- `make map-census TERRAIN=everon` → exit 0 (`censusStatus=partial`, verify-type-inventory OK with
  new **I3** enum-membership + **I5/I7** manifest cross-checks).
- `make map-export-validate` → **exit 0** — committed artifacts valid per registry terrain; **E2a**
  (2 terrains) / **E2b** (`export-terrain.sh arland` → exit 2 operator-instructions branch, same
  code path) / **E2c** (zero literal terrain ids in pipeline scripts).
- `bash scripts/map-assets/verify-spike-all.sh TERRAIN=everon` → **ALL PASS** — T-090.3.0 spike
  verifiers unaffected (ops log `fullExport` block APPENDED, spike keys intact).

## Host pipeline smoke (pre-Workbench, spike JSONL 7,401 rows)

Builder produced 13 prefabs / 41 instances / chunk `2_12` — byte-identical across runs and equal to
the spike census; all rows Ajv-valid; **41/41** building anchors resolved ≤ 2 m through the chunk
files. Roads builder byte-identical across runs.

## Deviations / notes

- **Chunk tuple:** `[prefabId, x, y, z, rotationDeg]` all-number 5-tuple per plan §3.2 (additive
  `oneOf` branch in `map-object-instance.schema.json`; legacy id-first tuple tightened to
  string-first `prefixItems`; Ajv consumers gain `strictTuples: false` — lint heuristic only).
- **Rotation:** plugin emits `headingDeg = GetAngles()[1]` (S6 fix); builder accepts legacy spike
  `pitchDeg` fallback for smoke only.
- **noPrefab rows** (`resourceName === ""`): 182,661 (12.95%) — excluded from catalog/census/G11 by
  rule; counted in ops log (top classNames: GenericEntity 163,694; Building 9,047 — engine-side
  entities with no `EntityToSource`, unreachable by prefab classification).
- **roadClass mapping provisional** (`classMappingProvisional: true` in ops log fullExport.roads):
  topo type 3→road_paved, 5→road_dirt until P6–P9 purity gates own the correction.
- **Pre-existing spec-gate failures fixed in passing** (code-side, no doc edits): gate 8 regex
  matched `T-090.1` inside `T-090.10.1` (`(?!\d)` added); gate 7 `make verify-t090-spec-consistency`
  alias target added to the Makefile.
- **manifest.json** objects block filled (prefabsPath/prefabCount/instanceCount/chunksPath/
  chunkSizeM/roadsPath/importPhaseMax/importPhaseShipped/exportedAt); `terrain-registry.json` everon
  `importPhaseShipped: ["P1_buildings"]` after gates passed. `densityPath`/`densityCellM`/`lod`
  schema fields documented + reserved — values land in **T-090.3.2**.
- **wb_reload cannot recompile Workbench plugin scripts** (all ScriptEditor/WorldEditor menu compile
  paths return ExecuteAction=false) — new plugin classes need one operator Script Editor reload or
  Workbench restart; documented in the `make map-export` operator instructions.

## Next

**T-090.3.2** — density grids (`objects/density/*.bin` TBDD) + PH-P2 trees + forest regions.
Cursor doc sync: phase-gate renumbering (roads pulled forward), LOD contract v2 references,
registry/spec status. **T-090.5.1** render spine only after this doc sync per single-lane rule.
