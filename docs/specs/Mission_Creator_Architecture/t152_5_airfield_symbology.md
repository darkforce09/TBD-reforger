# T-152.5 — Airfield symbology (runway / apron / structures)

**Ticket:** T-152 · **Slice:** T-152.5  
**Status:** `ready` (blocked until **T-152.4** G1–G10 PASS · **T-152.2/.3** icon keys for hangar/tower)  
**Executor:** **grok-cursor**  
**Authority:** T-152 program hub · [`t090_1_2_5_2_water_topo_refine.md`](t090_1_2_5_2_water_topo_refine.md) · [`t144_arma3_map_architecture_study.md`](t144_arma3_map_architecture_study.md)  
**Worktree:** `/home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152` · tag **`T-152.5`**  
**Depends on:** **T-152.4** · **T-152.2/.3** (hangar + control-tower glyph keys) · **T-152.0–.3** PASS

## In one sentence

Polish **NW Everon airfield** cartographic read: **runway** road class styling, **DEM-flat apron** fill, **hangar/control-tower** icons, layer toggle — with taxiway **spike-or-ship** path documented as G-taxiway A or B.

---

## Problem

Runway centerlines already exist in `packages/map-assets/everon/objects/roads.json.gz` as **`roadClass: "runway"`** segments from `.topo` type-0 records (`scripts/map-assets/build-roads-from-topo.mjs:39–45`, `TOPO_TYPES.AIRFIELD → "runway"`). Spike `analyze-water-sources.mjs` documents **5 airfield/runway records** sitting on the **NW engine-flattened pad** (`0.833` flat-overlap fraction). On wgpu they draw as generic road strips (`polyline_strip.rs`) without cartographic **runway polish** (wider pale asphalt, higher contrast casing). **Apron/taxiway pavement** is not a distinct polygon layer — flattened DEM pad is visible in hillshade but not as a labeled cartographic feature. **Hangars** (`buildingClass: "hangar"`) and **towers** export as building OBBs with optional badges at `BUILDING_BADGE_MIN_ZOOM` (`lodGates.ts:25`) but lack **airfield-specific iconography** from T-152.3. **Taxiway** linework: `.topo` type-0 is **runway-only** (5 records); taxiway may require **RoadEntity attribute spike** in-slice — if empty, shipped read = apron + runway + structures (A3-like).

---

## Goal

1. **Runway polish:** Distinct stroke style for `roadClass=runway` — width **20 m** (matches `build-map-cartographic.mjs:58`), fill `#b8bdb8`, casing `#6a706e`, round caps (reuse `expand_polyline_strip`).
2. **Apron polygon:** DEM-guided flat pad at **NW airfield** — rasterize cells where `|elev − pad_mean| < 0.5 m` inside bbox from runway segment union + **30 m** margin; emit single `PolygonFill` layer `world-airfield-apron`.
3. **Structure icons:** Place **`building-hangar`** and **`building-tower`** glyphs (T-152.3 atlas) at centroids of matching `buildingClass` instances inside airfield bbox; scale per `glyph_size_meters` / badge rules.
4. **Layer toggle:** `worldLayerPrefs.airfield` (default on) — controls apron + runway polish + airfield icons (not all roads).
5. **Taxiway spike (in-slice):** Query Workbench MCP / `RoadEntity` / `.topo` attrs for taxiway polylines. **Path A:** attrs found → export + render as `roadClass=road_paved` width 8 m inside apron. **Path B:** empty → verify log records **`G-taxiway-path B`**; acceptance = runway+apron+structures only. **Both PASS** if G3–G8 met.
6. Verify log `.ai/artifacts/t152_5_verify_log.md`.

---

## Out of scope

- New DEM export (T-091.0 shipped).
- Second airfield (Arland) — Everon-only gates; Arland follows after operator request.
- PAPI/VASI lights, runway numbers text (**T-152.7**/**T-152.9**).
- Full P6–P10 road phases (runway already in roads.json.gz).
- Registry/docs (Cursor).

---

## Locked decisions

| # | Decision | Rationale |
|---|----------|-----------|
| L1 | **NW airfield bbox** = union of runway segment AABBs + **30 m** margin; pinned test centroid **(2100, 11100)** ±500 m (operator calibrate in verify log) | `.topo` type-0 alignment |
| L2 | Runway style: width **20 m**, colors per Goal; `class_visible('runway')` uses `lodGates.ts:75` (**−6**) | Existing road LOD |
| L3 | Apron fill: `#9aa3a2` @ α0.55; only where local DEM flatness **σ < 0.3 m** inside bbox | DEM-flat predicate |
| L4 | Hangar/tower icons only inside airfield bbox; min zoom **`BUILDING_BADGE_MIN_ZOOM (1)`** | Readability |
| L5 | Taxiway spike **mandatory attempt**; document MCP/pak query in verify log §Spike | No silent deferral |
| L6 | **`G-taxiway-path A`** = taxiway polyline count ≥ 1 rendered; **B** = count 0 + explicit operator note | User locked |
| L7 | Draw order: apron → runway casing → runway center → building footprints → airfield icons | Z-order |
| L8 | Everon runway segment count gate **≥ 5** (matches `.topo` type-0 census) | Data evidence |
| L9 | Tag **`T-152.5`** | Convention |

---

## Pinned numbers

| Quantity | Value | Source |
|----------|-------|--------|
| Runway `.topo` records | **5** | `analyze-water-sources.mjs:668` |
| Runway `roadClass` | **`runway`** | `build-roads-from-topo.mjs:40` |
| Cartographic runway width | **20 m** | `build-map-cartographic.mjs:58` |
| `BUILDING_BADGE_MIN_ZOOM` | **1** | `lodGates.ts:25` |
| Apron flatness σ threshold | **0.3 m** | This slice |

---

## Tasks

1. Compute airfield bbox from `roads.json.gz` runway segments; vitest fixture.
2. Rust: runway style table branch in road compose (`residency` or `roads_compose.rs`).
3. Rust: DEM apron mask → polygon (`dem/sample.rs` + marching or grid flood).
4. Rust: hangar/tower glyph instances filtered by bbox + class.
5. Taxiway spike script `.ai/artifacts/t152_5_taxiway_spike.json` + optional render path.
6. TS: `worldLayerPrefs.airfield` + Mission Settings checkbox.
7. Verify log + tag.

---

## Mathematical acceptance matrix

| Gate | Predicate | Class |
|------|-----------|-------|
| **G1** | `count(roadSegments where roadClass=runway) ≥ 5` on Everon | Census |
| **G2** | Apron polygon area **`> 0`** m² and **`≥ 15_000`** m² (sanity floor for NW pad) | Geometry |
| **G3** | **`∃ hangar`** instance in bbox with glyph drawn at `deckZoom=2` | Icon |
| **G4** | **`∃ tower`** (`buildingClass=tower`) in bbox with glyph at `deckZoom=2` | Icon |
| **G5** | Runway segment screen width at `deckZoom=0` = **`20·2^0 ± 0.05`** m world (L9 strip gate) | Class R |
| **G6** | Toggle `airfield=false` → apron+runway polish+icons hidden; other roads unchanged | UI |
| **G7** | Verify log contains **`G-taxiway-path A` or `B`** with spike evidence JSON | Spike |
| **G8** | T-152.4 verify **PASS**; FE build/lint/test; `make wasm` | Regression |

---

## Verify

```bash
cd /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152
git lfs pull && make map-assets-link

# G1 runway census
node -e "
const z=require('node:zlib'),f=require('node:fs');
const r=JSON.parse(z.gunzipSync(f.readFileSync('packages/map-assets/everon/objects/roads.json.gz')));
const n=(r.roadSegments||[]).filter(s=>s.roadClass==='runway').length;
if(n<5){console.error('G1 FAIL',n);process.exit(1);}
console.log('G1 OK runways',n);
"

cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test -p map-engine-core --all-features
cargo test -p map-engine-render
make wasm
cd apps/website/frontend && npm test && npm run build && npm run lint
```

---

## Manual acceptance

- **M1:** Island zoom cartographic Map — **NW airfield** reads as flattened pale pad + runway cross.
- **M2:** Zoom to airfield — **hangar row** icons visible; control tower distinct.
- **M3:** Toggle airfield layer — pad/runway polish/icons disappear together.
- **M4:** If path B — operator confirms taxiway-less read is **acceptable A3-like** (note in verify log).

---

## Documentation sync (Cursor, after merge)

Registry `T-152.5 → shipped`; hub airfield row; spike JSON link; `./scripts/ticket sync`.

---

## Grok Code prompt — T-152.5 (copy-paste)

```
Read CLAUDE.md first. CWD: /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152

Implement **T-152.5** — airfield symbology.

═══ PREFLIGHT ═══
  cd /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152
  git lfs pull && make map-assets-link && make wasm
  Confirm t152_4_verify_log.md ALL PASS

═══ READ ═══
  1. docs/specs/Mission_Creator_Architecture/t152_5_airfield_symbology.md
  2. scripts/map-assets/{build-roads-from-topo,build-map-cartographic,analyze-water-sources}.mjs
  3. crates/map-engine-core/src/{geometry/polyline_strip,dem/sample,world/residency}.rs
  4. apps/website/frontend/src/features/tactical-map/worldmap/lodGates.ts
  5. packages/map-assets/everon/objects/roads.json.gz

═══ PROBLEM ═══
  Runways exist as generic road strips; NW airfield lacks apron fill and airfield-specific icons.
  Taxiway data unknown — spike in-slice.

═══ LANGUAGE GATE ═══
  Rust: runway/apron geometry, DEM flat mask, icon placement, GPU compose.
  TS: layer toggle + thin wasm hooks only.

═══ LOCKED ═══
  - G1: ≥5 runway segments; apron area >0; hangar+tower glyphs in bbox
  - Taxiway path A or B — both PASS if G7 recorded
  - Runway 20 m styling; airfield bbox + apron σ gate
  - worldLayerPrefs.airfield

═══ DO ═══
  1. Runway polish in road compose
  2. DEM apron polygon (NW bbox)
  3. Hangar/tower glyphs (T-152.3 keys)
  4. Taxiway spike JSON + optional render
  5. Layer toggle + tests G1–G8
  6. t152_5_verify_log.md · tag T-152.5

═══ DO NOT ═══
  - docs/registry edits; skip taxiway spike attempt; defer apron as "follow-up"

═══ VERIFY / MANUAL / RETURN ═══
  Per spec §Verify, M1–M4, standard return block.
```
