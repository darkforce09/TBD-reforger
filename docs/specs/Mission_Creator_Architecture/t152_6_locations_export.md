# T-152.6 — Locations export (`locations.json`)

**Ticket:** T-152 · **Slice:** T-152.6  
**Status:** `ready` (blocked until **T-152.5** PASS)  
**Executor:** **grok-cursor**  
**Authority:** T-152 program hub · [`.ai/artifacts/t144_arma3_map_architecture_report.md`](../../../.ai/artifacts/t144_arma3_map_architecture_report.md) §5 (town names / importance) · [`t090_10_map_engine_v2_implementation_plan.md`](../../../.ai/artifacts/t090_10_map_engine_v2_implementation_plan.md) row 12 (`locations` export)  
**Worktree:** `/home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152` · tag **`T-152.6`**  
**Depends on:** **T-152.5** · blocks **T-152.8** (town labels)

## In one sentence

Export **`locations.json`** (+ JSON Schema in `packages/tbd-schema`) from Eden **`World/Locations/*`** with human display names — mathematical gates on schema, count, and required Everon towns.

---

## Problem

Town labels (**T-152.8**) need a stable **name + position + importance** feed. Map Engine v2 implementation plan defers `locations` export (row 12: "future"). No `locations.json` exists under `packages/map-assets/everon/`. Workbench viewport evidence (`.ai/artifacts/map_export_everon.json:22`) confirms real Everon towns (**Gorey, Highstone, RaccoonRock**). Engine location entities default to composition names like **"Location composition"** — unacceptable for cartographic labels. A3 uses a `Locations` store with importance-distance declutter (`t144` §5, `uiMap.cpp:1684–1697`).

---

## Goal

1. **Schema:** `packages/tbd-schema/schema/locations.schema.json` — array of `{ id, name, x, y, importance?, kind? }`; `name` min length 2; `x`/`y` world meters; `importance` default 0.5.
2. **Export artifact:** `packages/map-assets/{terrainId}/locations.json` (uncompressed JSON; gzip optional for large terrains).
3. **Export path (spike-or-ship in-slice):**
   - **Primary:** Workbench MCP / plugin enumerate `World/Locations/*` (or `Location` entities) → resolve display name from entity **displayName** / **name** property, not prefab class string.
   - **Fallback B:** Parse Eden `.ent` / pak `Location` resources if MCP blocked — document in `.ai/artifacts/t152_6_locations_spike.json`.
   - **Fallback C (Everon-only):** Curated seed from operator wiki list **only if** A+B fail after one operator ask — must still satisfy G3/G4.
4. **Validation:** Wire into `make schema-validate` / `validate.mjs`.
5. **Manifest pointer:** `packages/map-assets/everon/manifest.json` gains `"locations": { "path": "locations.json" }`.
6. Verify log `.ai/artifacts/t152_6_verify_log.md`.

---

## Out of scope

- Rendering labels (**T-152.8**).
- Arland locations (Everon gates only this slice).
- Backend API exposure.
- Editing locations in Mission Creator.

---

## Locked decisions

| # | Decision | Rationale |
|---|----------|-----------|
| L1 | **`REQUIRED_EVERON_TOWNS`** = `["Morton", "Gorey", "Highstone", "Raccoon Rock", "Saint Philippe", "Levie", "Montignac", "Kermovan"]` | Map export + operator evidence |
| L2 | Interim minimum count **`N_MIN = 10`** if census unknown pre-export; after export set **`N_MIN = count`** in verify log as new floor | User gate |
| L3 | Reject rows where `name` matches `/location composition/i` or `name.length < 2` | Display name quality |
| L4 | Coordinates: **x east, y north**, same as object chunks (`manifest.json` `worldBounds`) | Contract |
| L5 | `importance` ∈ **[0, 1]**; capital/large town ≥ **0.7** (operator table in verify log) | A3 declutter prep |
| L6 | Spike JSON mandatory; **no ship without G2 schema PASS** | Math gate |
| L7 | Tag **`T-152.6`** | Convention |

---

## Tasks

1. Author `locations.schema.json` + golden `packages/tbd-schema/golden/locations-everon-sample.json` (≥3 rows).
2. Spike script `scripts/map-assets/export-locations.mjs` (or Workbench plugin extension).
3. Run export for `TERRAIN=everon`; commit `locations.json`.
4. Hook validator; manifest update.
5. Census script for G3/G4; verify log.

---

## Mathematical acceptance matrix

| Gate | Predicate | Class |
|------|-----------|-------|
| **G1** | `make schema-validate` exit 0 including locations schema + sample | Schema |
| **G2** | `locations.json` validates against schema (Ajv) | Schema |
| **G3** | `count(locations) ≥ N_MIN` (**10** interim, bump after first export) | Census |
| **G4** | **`REQUIRED_EVERON_TOWNS ⊆ {loc.name}`** (case-insensitive match; allow "Raccoon Rock" vs "RaccoonRock") | Coverage |
| **G5** | **`∀ loc: name.length ≥ 2 ∧ finite(x,y)`** | Row quality |
| **G6** | **`∀ loc: ¬/location composition/i.test(name)`** | No placeholder names |
| **G7** | Spike JSON documents export path **A, B, or C** with evidence | Provenance |
| **G8** | T-152.5 verify PASS; `make map-export-validate` still PASS | Regression |

---

## Verify

```bash
cd /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152
git lfs pull && make map-assets-link

# Export (after spike script exists)
node scripts/map-assets/export-locations.mjs --terrain everon
# or documented Workbench step → locations.json

make schema-validate
make map-export-validate

node -e "
const fs=require('node:fs');
const locs=JSON.parse(fs.readFileSync('packages/map-assets/everon/locations.json'));
const req=['Morton','Gorey','Highstone','Raccoon Rock','Saint Philippe','Levie','Montignac','Kermovan'];
const names=new Set(locs.map(l=>l.name.toLowerCase().replace(/\\s+/g,'')));
for(const t of req){
  const k=t.toLowerCase().replace(/\\s+/g,'');
  if(!names.has(k) && ![...names].some(n=>n.includes(k.slice(0,6)))) {
    console.error('G4 FAIL missing',t); process.exit(1);
  }
}
if(locs.length<10){console.error('G3 FAIL',locs.length);process.exit(1);}
console.log('G3/G4 OK',locs.length);
"
```

---

## Manual acceptance

- **M1:** Spot-check **Gorey** + **Morton** coordinates against cartographic Map cursor (±100 m).
- **M2:** No row displays **"Location composition"** in exported JSON.

---

## Documentation sync (Cursor, after merge)

Registry; hub data row; schema index; `./scripts/ticket sync`.

---

## Grok Code prompt — T-152.6 (copy-paste)

```
Read CLAUDE.md first. CWD: /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152

Implement **T-152.6** — locations export.

═══ PREFLIGHT ═══
  Confirm t152_5_verify_log.md PASS
  git lfs pull && make map-assets-link

═══ READ ═══
  1. docs/specs/Mission_Creator_Architecture/t152_6_locations_export.md
  2. .ai/artifacts/t144_arma3_map_architecture_report.md (§5 labels)
  3. .ai/artifacts/map_export_everon.json
  4. packages/map-assets/everon/manifest.json
  5. packages/tbd-schema/schema/*.schema.json (patterns)
  6. scripts/mod/mcp-call.sh + docs/mod/MCP_TOOLING.md (Workbench spike)

═══ PROBLEM ═══
  No locations.json; town labels blocked. Need schema + Everon export with real names.

═══ LANGUAGE GATE ═══
  Export/validation scripts may be Node (.mjs). No wgpu label rendering in this slice.
  If Workbench plugin needed: Enfusion C in apps/mod per existing export plugins.

═══ LOCKED ═══
  - REQUIRED_EVERON_TOWNS list; N_MIN≥10; no "Location composition" names
  - Spike JSON mandatory (path A/B/C)
  - Schema + golden sample + manifest pointer

═══ DO ═══
  1. locations.schema.json + golden sample
  2. Spike then export everon/locations.json
  3. validate.mjs hook; G3–G7 scripts
  4. t152_6_verify_log.md · tag T-152.6

═══ DO NOT ═══
  - Town label rendering (T-152.8); docs/registry; silent deferral without operator ask

═══ VERIFY / RETURN ═══
  Per spec.
```
