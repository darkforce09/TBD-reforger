# T-152.9 — Road names (polyline-following labels)

**Ticket:** T-152 · **Slice:** T-152.9  
**Status:** `ready` (blocked until **T-152.1** text lane **and** **T-152.8** PASS)  
**Executor:** **grok-cursor**  
**Worktree:** `/home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152` · tag **`T-152.9`**  
**Depends on:** **T-152.1** · **T-152.8** (sequential per program) · `roads.json.gz` shipped (T-090.3.3)

## In one sentence

Spike **RoadEntity / `.topo` name attrs**; if empty ship **`road-names.json`** curated for Everon major routes and draw **polyline-following** road labels on the text lane with declutter.

---

## Problem

`packages/map-assets/everon/objects/roads.json.gz` has **888** segments (`CLAUDE.md` T-090.3.3) with `roadClass` but **no `name` field** in the committed schema sample. `decode-topo.mjs` documents `.topo` as **geometry-only** (roads/airfield line work — `analyze-water-sources.mjs:287–288`). Eden **RoadEntity** may carry display names — **unknown until in-slice spike**. Cartographic maps need **major route names** (A3 draws road names at sufficient zoom). Text must use **T-152.1** lane with **tangent-aligned** placement along polylines.

---

## Goal

1. **Spike (mandatory):** `.ai/artifacts/t152_9_road_name_spike.json` — query:
   - `RoadEntity` / `SCR_Road` attributes via Workbench MCP;
   - `.topo` record attrs beyond type/class;
   - Eden wiki / game data cross-check.
2. **Data path A:** attrs found → extend `roads.json.gz` export with optional **`name?: string`** (schema bump + `make schema-validate`).
3. **Data path B (IN SCOPE if A empty):** `packages/map-assets/everon/road-names.json` — curated list:
   ```json
   { "roads": [{ "id": "...", "name": "...", "segmentIds": ["r042", ...] }] }
   ```
   Cover **`MAJOR_EVERON_ROADS`** list in Locked decisions (≥6 named highways).
4. **Placement:** For each named polyline, place **1–3** labels at arc-length fractions **0.5** (and 0.25/0.75 if length > 3 km); rotate text **tangent ± 180°** so readable (upright); offset **6 m** normal from centerline.
5. **Declutter:** Reuse label-label distance gate **`≥ 60·2^(-zoom)`** m; road-road priority by `roadClass` (highway > paved > dirt).
6. **Zoom:** Names visible **`deckZoom ≥ ROAD_NAME_MIN_ZOOM (0)`** for highway; **`≥ 1`** for secondary.
7. Toggle `worldLayerPrefs.roadNames`.
8. Verify log `.ai/artifacts/t152_9_verify_log.md`.

---

## Out of scope

- Generating road geometry (shipped).
- OSM import.
- Arland road names (Everon gates).
- Street-level naming for every `track`/`path`.

---

## Locked decisions

| # | Decision | Rationale |
|---|----------|-----------|
| L1 | **`MAJOR_EVERON_ROADS`** must be covered: `["Main Highway", "North-South Highway", "Coastal Road", "Airfield Access", "Gorey Road", "Morton Road"]` — fuzzy match OK | Operator + map literacy |
| L2 | Path **A or B** documented; **curated JSON is IN SCOPE** not deferral | User locked |
| L3 | Schema: if `name` on segment, **`typeof name === 'string' && name.length ≥ 2`** | Gate |
| L4 | Polyline following: tangent from segment at placement **s**; angle flip if `|θ| > 90°` | Readability |
| L5 | Max **24** road name labels on screen after declutter | Perf |
| L6 | Draw below town labels, above roads stroke | Z-order |
| L7 | Tag **`T-152.9`** | Convention |

---

## Tasks

1. Spike JSON + README in artifact.
2. Schema update if path A; else author `road-names.json` + loader.
3. Rust `road_labels.rs` — placement + declutter + text instances.
4. TS toggle.
5. `verify-road-names.mjs` for G3/G4.
6. Verify log.

---

## Mathematical acceptance matrix

| Gate | Predicate | Class |
|------|-----------|-------|
| **G1** | T-152.1 + T-152.8 verify **PASS** | Dependency |
| **G2** | Spike JSON exists with **`path: "A"|"B"`** | Provenance |
| **G3** | **`MAJOR_EVERON_ROADS`** each **`∃ label`** with `name` fuzzy-match at `deckZoom=0` | Coverage |
| **G4** | **`∀ label: name.length ≥ 2`** | Schema |
| **G5** | Placement: label center within **`≤ 12 m`** perpendicular distance of assigned polyline | Geometry |
| **G6** | Declutter: **`∀ pair: dist ≥ 60·2^(-z)`** at test zoom 0 | Declutter |
| **G7** | **`|on_screen| ≤ 24`** | Cap |
| **G8** | Regression green | CI |

---

## Verify

```bash
cd /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152
git lfs pull && make map-assets-link

test -f .ai/artifacts/t152_9_road_name_spike.json || (echo 'G2 FAIL' && exit 1)

make schema-validate
cargo test -p map-engine-core road_labels --all-features
make wasm
cd apps/website/frontend && npm test && npm run build && npm run lint

node scripts/map-assets/verify-road-names.mjs --terrain everon --zoom 0
```

---

## Manual acceptance

- **M1:** Zoom Everon central valley — at least **one** highway name readable along curve.
- **M2:** Rotate/pan — labels stay glued to road (no sliding).
- **M3:** Toggle road names off.

---

## Documentation sync (Cursor, after merge)

Registry; note curated vs exported path in hub; `./scripts/ticket sync`.

---

## Grok Code prompt — T-152.9 (copy-paste)

```
Read CLAUDE.md first. CWD: /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152

Implement **T-152.9** — road names.

═══ PREFLIGHT ═══
  Confirm t152_8_verify_log.md + t152_1_verify_log.md PASS

═══ READ ═══
  1. docs/specs/Mission_Creator_Architecture/t152_9_road_names.md
  2. packages/map-assets/everon/objects/roads.json.gz
  3. scripts/map-assets/decode-topo.mjs
  4. packages/tbd-schema/schema/map-object-roads*.json (if present)
  5. docs/mod/MCP_TOOLING.md

═══ PROBLEM ═══
  Roads have no names in export; spike attrs; if empty ship curated road-names.json + polyline labels.

═══ LANGUAGE GATE ═══
  Rust: polyline tangent placement, declutter, text GPU.
  TS: toggle. Curated JSON editing is data — not TS policy.

═══ LOCKED ═══
  - Spike mandatory; path B curated JSON IN SCOPE
  - MAJOR_EVERON_ROADS coverage G3
  - Polyline follow + declutter G5/G6

═══ DO ═══
  1. t152_9_road_name_spike.json
  2. Path A or B data + loader
  3. road_labels.rs + text lane
  4. verify-road-names.mjs; t152_9_verify_log.md · tag T-152.9

═══ DO NOT ═══
  - Defer curated file as follow-up; DOM labels; docs/registry

═══ VERIFY / RETURN ═══
  Per spec.
```
