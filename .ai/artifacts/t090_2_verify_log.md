# T-090.2 verify log — map object taxonomy ship

**Slice:** T-090.2 · **Branch:** `ticket/T-090-2` · **Date:** 2026-07-03 · **Executor:** claude-code  
**Spec:** [`docs/specs/Mission_Creator_Architecture/t090_2_map_object_taxonomy.md`](../docs/specs/Mission_Creator_Architecture/t090_2_map_object_taxonomy.md)

---

## Preflight / baseline

- `git fetch origin && git rebase main` — **clean**, no conflicts. Main advanced with T-090.1.2.5.1 (water slice) @ `82488c6f`; `packages/map-assets/everon/manifest.json` merged cleanly → optional `objects` stub **shipped** (not deferred).
- Baseline `make schema-validate` @ post-rebase: all gates PASS **except** `verify-t090-specs` — 3 fails, all `t090_2_map_object_taxonomy.md: referenced npm run verify-map-object-golden not in packages/tbd-schema/package.json scripts`. Pre-existing spec-forward reference, **self-healed by this slice** (script + npm wiring shipped). No `docs/**` edits made or needed.

---

## Automated gates

| ID | Command / check | Result | Notes |
|----|-----------------|--------|-------|
| S1 | `make schema-validate` (AJV goldens) | **PASS** (exit 0) | "All contracts valid." — now also runs S2–S9 verifier |
| S2–S9 | `npm run verify-map-object-golden` | **PASS** (exit 0) | 8/8 gates; zero missing enum examples |
| S10 | `make map-object-enums-verify` | **PASS** (exit 0) | 52 prefabs, 10 glyphs, enums single-source |
| Census | `make map-census TERRAIN=everon` | **PASS** (exit 0) | `pending_export` OK — validate-only path, unchanged |

**Commit:** see `git log -1` on `ticket/T-090-2` (T-090.2 ship commit) · **Tag:** `T-090.2`

### `make schema-validate` output (primary ship gate)

```
All contracts valid.
verify-map-object-enums: OK (52 prefabs, 10 glyphs, enums single-source)
  PASS  S2 — every prefab + instance row has resolvable kind + class
  PASS  S3 — ≥1 prefab example per instance kind
  PASS  S4 — road segments + road prefabs use valid roadClass
  PASS  S5 — prefab dedup — unique prefabId/resourceName; instances carry no type fields
  PASS  S6 — every instance prefabId resolves in its own prefab table
  PASS  S7 — every prefab has ai.summary + ai.taxonomyPath + gameplay.cover.type + spatial.heightM
  PASS  S8 — resolved samples validate map-object-resolved.schema.json
  PASS  S9 — full closed-enum coverage (prefab classes + road segments + region kinds)
verify-map-object-golden: OK (S2–S9; 52 prefabs, 7 instances, 7 segments, 4 regions, 4 resolved; zero missing enum examples)
verify-map-glyphs: OK (10 glyphs, all golden iconKeys covered)
verify-type-inventory: OK
verify-t090-specs: OK (30 spec files + authority docs, all 11 gates pass)
verify-n6-sentence: OK (N6 sentence identical across 5 locations)
verify-n10-tile-budget: OK (N10 tile-budget single-source across basemap + pipeline)
```

### `make map-census TERRAIN=everon` output

```
verify-type-inventory: OK
map-census: everon censusStatus=pending_export — exact counts unknown until Workbench export + classify (see t090_world_object_type_inventory.md)
```

---

## S9 coverage checklist

Prefabs — one row per missing enum example (prefabId 23–51; all new rows omit `render` so the glyph gate stays green pending T-090.5 SVGs):

- [x] tree: `dead`, `unknown`
- [x] vegetation: `grass`, `fern`, `dead`, `unknown`
- [x] rock: `cliff`, `pebble`, `scree`, `unknown`
- [x] prop: `barrier`, `sign`, `furniture`, `debris`, `pebble`, `unknown`
- [x] utility: `lamp`, `antenna`, `pipeline`, `unknown`
- [x] water: `dock`, `buoy`, `unknown`
- [x] road prefab: `highway_paved`, `road_dirt`, `track`, `path`, `runway`, `unknown`
- [x] building: all 14 `buildingClass` (already complete @ bootstrap)

Roads sample:

- [x] `road_paved`, `path`, `runway` (3-coord points), `unknown`

Regions sample:

- [x] `waterBody` polygon (`waterbody-everon-001`, `source: engine-mask`)

Also expanded:

- [x] instances: +3 rows referencing new prefabIds 29/39/33 (one compact tuple kept — S5 dedup demo)
- [x] resolved: +2 rows (rock/cliff with `placement` overlay, utility/lamp with `z: null`)
- [x] `map-object-catalog-everon-sample.json`: **unchanged** (self-contained bundle; S6 bundle isolation)

---

## Other deliverables

- **Verifier:** `packages/tbd-schema/scripts/verify-map-object-golden.mjs` — S2–S9; catalog bundles resolve instances only against their **own** `prefabs[]` (bundle isolation).
- **Wiring:** npm `verify-map-object-golden`; folded into `make schema-validate`; standalone `make map-object-golden-verify`.
- **Classify rules:** +12 append-only rules (palm, dead tree, bush, grass, cliff, boulder, lamp, antenna, pier, barrier, sign, runway). Render blocks only reuse existing glyph keys (`tree-palm`, `vegetation-bush`, `rock-boulder`); the rest omit `render`. Bootstrap rules + fallback untouched.
- **Manifest:** minimal `objects` stub on `packages/map-assets/everon/manifest.json` (`schemaVersion`, `format: catalog-v1`, `typeInventoryPath`) — schema-valid, no counts (census `pending_export`).
- **census-types.mjs:** unchanged.

---

## Manual spot-check

| ID | Result | Notes |
|----|--------|-------|
| S7-spot | **PASS** | prefabId 14 (building/military Barracks) + prefabId 0 (tree/conifer Pine medium) — full `gameplay` + `ai` blocks eyeballed |
| S9-full | **PASS** | verify-map-object-golden prints "zero missing enum examples" |

**Ready for Cursor doc sync.**
