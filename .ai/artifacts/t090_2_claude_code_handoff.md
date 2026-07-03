# T-090.2 — Claude Code handoff (map object taxonomy ship)

**Slice:** T-090.2 · **Executor:** claude-code · **Branch:** `ticket/T-090-2`  
**Worktree:** `.ai/artifacts/worktrees/TBD-T-090-2` (parallel with **T-090.1.2.5.1** on `main`)  
**Parent bootstrap:** **T-090.0.2** (cursor-docs — schemas + partial goldens already on `main`)  
**Spec (authority):** [`docs/specs/Mission_Creator_Architecture/t090_2_map_object_taxonomy.md`](../../docs/specs/Mission_Creator_Architecture/t090_2_map_object_taxonomy.md)

---

## What you are building

Complete the **T-090.2 ship slice**: golden coverage for **S1–S10**, a semantic verifier script, expanded `prefab-classify.json` rules, and optional `objects` stub on the Everon manifest — **no** Workbench export, **no** Deck.gl render, **no** satellite ortho edits.

```text
expand golden/map-objects/*  →  verify-map-object-golden.mjs (S2–S10)
  →  prefab-classify.json + census-types stub alignment
  →  make schema-validate + make map-object-enums-verify
  →  tag T-090.2
```

---

## Bootstrap already shipped (T-090.0.2 — do not redo)

| Area | Path | State |
|------|------|-------|
| Schemas | `packages/tbd-schema/schema/map-object-*.schema.json` | **Done** — AJV in `validate.mjs` |
| Enum gate | `scripts/verify-map-object-enums.mjs` | **Done** — GAP-M5 |
| Glyph gate | `scripts/verify-map-glyphs-manifest.mjs` | **Done** |
| Partial goldens | `packages/tbd-schema/golden/map-objects/*` | **Partial S9** — see gap table in spec |
| Type inventory | `packages/map-assets/everon/objects/type-inventory.json` | `censusStatus: pending_export` (expected) |
| Census script | `scripts/map-assets/census-types.mjs` | Validates only until T-090.3 export |

**Baseline @ `0418d952`:** `make schema-validate` exit 0.

---

## S9 gaps (your primary expansion work)

**Prefabs** (`map-object-prefabs-sample.json`) — add stub rows for missing enum examples:

| Kind | Missing `class` values in golden |
|------|----------------------------------|
| `tree` | `dead`, `unknown` |
| `vegetation` | `grass`, `fern`, `dead`, `unknown` |
| `rock` | `cliff`, `pebble`, `scree`, `unknown` |
| `prop` | `barrier`, `sign`, `furniture`, `debris`, `pebble`, `unknown` |
| `utility` | `lamp`, `antenna`, `pipeline`, `unknown` |
| `water` | `dock`, `buoy`, `unknown` |
| `road` (prefab kind) | `highway_paved`, `road_dirt`, `track`, `path`, `runway`, `unknown` (only `road_paved` today) |
| `building` | **Complete** — all 14 `buildingClass` values present |

**Roads** (`map-object-roads-sample.json`) — add segments for `road_paved`, `path`, `runway`, `unknown` (has `highway_paved`, `road_dirt`, `track`).

**Regions** (`map-object-regions-everon-sample.json`) — add ≥1 `waterBody` polygon (has `forest` ×2, `field` ×1).

**Instances** — ensure new prefabIds referenced; keep compact tuple row (S5 dedup demo).

**Resolved** (`map-object-resolved-sample.json`) — add materialized rows for new kinds if S8 coverage requires.

---

## Do not

| Forbidden | Why |
|-----------|-----|
| Edit `docs/**`, `.ai/tickets/registry.json`, CLAUDE status markers | Cursor doc sync after merge |
| Touch `packages/map-assets/everon/satellite/**`, SAP ortho, water composite | **T-090.1.2.5.1** owns that on `main` |
| Workbench export / full census compute | **T-090.3** |
| Deck.gl / MC render layers | **T-090.5** |
| Re-open T-090.0.2 schema structure unless S1 fails | Bootstrap shipped |

**Merge conflict risk (low):** `packages/map-assets/everon/manifest.json` if `.2.5.1` touches manifest — prefer optional `objects` stub only; coordinate via rebase onto `main` before merge.

---

## Execution order (strict)

1. **P0 — gap audit** — run `make schema-validate`; list missing S9 enum examples (spec §S9 gap audit).
2. **P1 — expand goldens** — prefabs, roads, regions, instances, resolved, catalog bundle as needed.
3. **P2 — `verify-map-object-golden.mjs`** — implement S2–S10 semantic gates; wire `npm run verify-map-object-golden` + Makefile target (or fold into `schema-validate`).
4. **P3 — `prefab-classify.json`** — add rules for new classes where obvious from resourceName patterns; extend fallback `unknown` rows.
5. **P4 — census-types.mjs** — no full compute; ensure stub path still exits 0 for `pending_export`.
6. **Optional:** `everon/manifest.json` `objects` block pointing at golden paths (schema-valid).
7. Log → `.ai/artifacts/t090_2_verify_log.md` (S1–S10 table + command output).
8. Tag **`T-090.2`** · prefix **`T-090.2:`**

---

## Preflight

```bash
cd .ai/artifacts/worktrees/TBD-T-090-2   # or stay on ticket/T-090-2
git fetch origin && git rebase main      # pick up .2.5.1 if merged
make schema-validate                     # baseline must pass before edits
./scripts/ticket brief T-090
```

---

## Key files

| File | Role |
|------|------|
| `packages/tbd-schema/golden/map-objects/map-object-prefabs-sample.json` | S9 prefab coverage |
| `packages/tbd-schema/golden/map-objects/map-object-roads-sample.json` | RoadClass coverage |
| `packages/tbd-schema/golden/map-objects/map-object-regions-everon-sample.json` | Region kinds incl. `waterBody` |
| `packages/tbd-schema/golden/map-objects/map-object-instances-sample.json` | S5/S6 dedup + prefabId resolve |
| `packages/tbd-schema/golden/map-objects/map-object-resolved-sample.json` | S8 resolved schema |
| `packages/tbd-schema/golden/map-objects/map-object-catalog-everon-sample.json` | Full catalog bundle |
| `packages/tbd-schema/scripts/validate.mjs` | S1 AJV (existing) |
| `packages/tbd-schema/scripts/verify-map-object-enums.mjs` | S10 enum drift (existing) |
| `packages/tbd-schema/scripts/verify-map-object-golden.mjs` | **Create** — S2–S9 semantic gates |
| `packages/tbd-schema/rules/prefab-classify.json` | Classify rules + fallback |
| `scripts/map-assets/census-types.mjs` | Stub validate (no full census yet) |
| `packages/tbd-schema/schema/map-object-enums.schema.json` | Single source of truth — extend only if spec requires new enum member |

---

## Verify commands

```bash
make schema-validate
make map-object-enums-verify
make map-census TERRAIN=everon    # must exit 0 with pending_export
```

After you add `verify-map-object-golden.mjs`:

```bash
cd packages/tbd-schema && npm run verify-map-object-golden
```

---

## S2–S10 semantic gates (implement in verify-map-object-golden.mjs)

| ID | Check |
|----|-------|
| S1 | AJV validates all golden rows (`validate.mjs` — already) |
| S2 | Every prefab + instance row has resolvable `kind` + `class` |
| S3 | ≥1 prefab example per top-level `kind` (instance kinds only; regions separate file) |
| S4 | Every road segment / road prefab uses valid `roadClass` |
| S5 | Instances do not duplicate `resourceName` — prefab table dedup only |
| S6 | Every instance `prefabId` resolves in prefabs[] |
| S7 | Every prefab has `ai.summary`, `ai.taxonomyPath`, `gameplay.cover.type`, `spatial.heightM` |
| S8 | Materialized resolved samples validate `map-object-resolved.schema.json` |
| S9 | Golden includes **every** `buildingClass` + **every** class enum member used by each kind (see gap table) |
| S10 | All `class` values ⊆ `map-object-enums.schema.json` (`verify-map-object-enums.mjs` — already) |

---

## Manual acceptance

| ID | Pass |
|----|------|
| **S9-full** | Script prints zero missing enum examples |
| **S7-spot** | Spot-check one building + one tree prefab for full `gameplay` block |

---

## Return contract

- Commit SHA + tag **`T-090.2`**
- `.ai/artifacts/t090_2_verify_log.md` with S1–S10 PASS table
- Automated verify output (all exit 0)
- **Ready for Cursor doc sync.**
