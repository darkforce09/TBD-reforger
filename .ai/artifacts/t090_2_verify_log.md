# T-090.2 verify log — map object taxonomy ship

**Slice:** T-090.2 · **Branch:** `ticket/T-090-2`  
**Spec:** [`docs/specs/Mission_Creator_Architecture/t090_2_map_object_taxonomy.md`](../docs/specs/Mission_Creator_Architecture/t090_2_map_object_taxonomy.md)

---

## Automated gates (Claude Code fills on ship)

| ID | Command / check | Result | Notes |
|----|-----------------|--------|-------|
| S1 | `make schema-validate` (AJV goldens) | | |
| S2–S9 | `npm run verify-map-object-golden` | | |
| S10 | `make map-object-enums-verify` | | |
| Census | `make map-census TERRAIN=everon` | | `pending_export` OK |

**Commit:** `{sha}` · **Tag:** `T-090.2`

---

## S9 coverage checklist (Claude Code)

Prefabs — one row per missing enum example:

- [ ] tree: `dead`, `unknown`
- [ ] vegetation: `grass`, `fern`, `dead`, `unknown`
- [ ] rock: `cliff`, `pebble`, `scree`, `unknown`
- [ ] prop: `barrier`, `sign`, `furniture`, `debris`, `pebble`, `unknown`
- [ ] utility: `lamp`, `antenna`, `pipeline`, `unknown`
- [ ] water: `dock`, `buoy`, `unknown`
- [ ] road prefab: `highway_paved`, `road_dirt`, `track`, `path`, `runway`, `unknown`
- [ ] building: all 14 `buildingClass` (already complete @ bootstrap)

Roads sample:

- [ ] `road_paved`, `path`, `runway`, `unknown`

Regions sample:

- [ ] `waterBody` polygon

---

## Manual spot-check

| ID | Result | Notes |
|----|--------|-------|
| S7-spot | | One building + one tree — full `gameplay` + `ai` blocks |
| S9-full | | verify-map-object-golden prints zero gaps |
