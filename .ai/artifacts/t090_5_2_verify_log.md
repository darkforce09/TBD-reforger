# T-090.5.2 — Map Engine v2: roads + buildings live — verify log

**Date:** 2026-07-05 · **Executor:** claude-code · **Slice spec:** `docs/specs/Mission_Creator_Architecture/t090_5_map_object_render_layer.md` · **Plan:** `.ai/artifacts/t090_10_map_engine_v2_implementation_plan.md` §4.1–4.2, §5, §7 row T-090.5.2 · **LOD authority:** `t090_render_lod_contract.md` v2 · **Glyphs:** `t090_world_object_glyphs.md`

## Scope shipped

First live world-object Deck layers on the T-090.5.1 spine: **`world-roads`** (PathLayer, 766 segments), **`world-buildings`** (PolygonLayer, 5,606 OBB footprints), **`world-building-badges`** (IconLayer, military/tower/bunker) — flag-gated behind `VITE_WORLDMAP_ENABLED=1`, all `pickable: false` (pick = T-090.9). NO trees/forest/contours/sea/chunk-streaming (T-090.5.3/.5.4/.5.5/.8.1). Export pipeline, Workbench, docs, registry untouched.

| # | Deliverable | Path |
|---|---|---|
| 1 | Glyph atlas builder (`make map-glyphs-build`, stub → real) | `scripts/map-assets/build-glyph-atlas.mjs` + root `Makefile` |
| 2 | P1 building glyph set (9 SVGs, every non-empty buildingClass) | `packages/map-assets/glyphs/svg/building-{residential,civic,agricultural,industrial,commercial,hangar,bunker,tower,military}.svg` + `manifest.json` entries |
| 3 | Built atlas artifacts (committed, plain git) | `packages/map-assets/glyphs/atlas/world-glyphs.webp` (1024×512, 11.8 KB, lossless) + `world-glyphs.json` (Deck-ready mapping + meta) |
| 4 | Glyph verify gate extended (G2-everon / G4-atlas / G6-fields) | `packages/tbd-schema/scripts/verify-map-glyphs-manifest.mjs` |
| 5 | Road layer builder + pure style/parse | `worldmap/roadLayer.ts` (+ test) |
| 6 | Building OBB + badge builders + pure geometry/filter | `worldmap/buildingLayer.ts` (+ test) |
| 7 | World-object data loader (roads one-shot + indexed chunk fetch) | `worldmap/worldData.ts` |
| 8 | Glyph atlas load-once | `layers/worldGlyphAtlas.ts` |
| 9 | Layer assembly (toggles + gates, slots 6–7 order) | `worldmap/useWorldMapLayers.ts` (extended, spine intact) |
| 10 | Zoom/terrain wire | `TacticalMap.tsx` (`useWorldMapLayers({ terrain, deckZoom })`) |
| 11 | `@deck.gl/extensions ^9.3.4` (PathStyleExtension dashes) | `apps/website/frontend/package.json` + lock |

## Decisions

- **Chunk enumeration via export index:** `objects/chunks/manifest.json` (`{chunkSizeM, cells:[{cx,cy,path,instanceCount}]}`, 270 Everon cells) ships with the T-090.3 export — the loader fetches exactly the files that exist. Fallback (index missing): full-grid sweep via `chunkMath` with misses read as empty. Worker + chunkStore LRU streaming still land in T-090.5.3; this loader is the interim main-thread path sanctioned for P1 scale (5.6k buildings).
- **Trees never retained:** mixed P2 chunks are filtered through the building-prefab lookup at parse time; 501,861 tree rows are parsed and dropped (render = T-090.5.5).
- **Badge key derived from `class`**, not prefab `render.iconKey` — everon bunker prefabs (84/99/329) ship without an iconKey; contract badge rule is class-based (military/tower/bunker → `building-badge-*`).
- **Road widths:** contract table is px @ deckZoom 0 where 1 px = 1 m → `widthUnits:'meters'` reproduces it exactly and scales geometrically; `widthMinPixels: 1` = contract clamp. Dash `[8,6]` m for dirt/track/path, `[0,0]` solid.
- **Badge size:** `sizeUnits:'meters'` with size = baseSizePx/2^REF_ZOOM ≡ N2 formula (displayPx = base·2^(zoom−3)) with zero per-frame updates; `sizeMinPixels: 8` readability floor (plan §4.4 glyph min-px clamp).
- **Zoom → layers without churn:** hook memoizes on derived band state (visible road-class key + building/badge booleans), so continuous pan/zoom rebuilds nothing; band crossings swap filtered data (roads) / flip `visible` (buildings, badges — data stays on GPU).
- **gz decode:** gzip magic sniff + `DecompressionStream('gzip')`; Vite serves `.gz` with `Content-Encoding: gzip` (browser auto-decompresses → sniff falls through to plain parse) — both paths verified. SPA HTML fallback (missing file → 200 text/html) detected and treated as absent.
- **Atlas:** 128 px cells (crisp to ~8× base @ z+6), row-major sorted keys, power-of-two 1024×512 ≤ 4096² (GL-G4), magick RSVG rasterize @ density 512 + libwebp lossless. Committed plain (not LFS — `.gitattributes` map-assets rules cover png/r16/tbd-sat only; webp is 11.8 KB).
- **R11 empty state:** no `manifestUrl` / no manifest `objects` block (Arland, custom) → loader resolves empty, zero layers, no error.

## Automated verification — ALL PASS

```
make map-glyphs-build    → build-glyph-atlas: OK — 19 glyphs → 1024×512 atlas (11.8 KB)
make map-glyphs-verify   → verify-map-glyphs: OK (19 glyphs, golden + everon iconKeys covered, atlas rects verified)
make schema-validate     → all gates incl. verify-t090-specs: OK (36 spec files, 12 gates)
npm run test -- --run    → Test Files 13 passed · Tests 102 passed (85 pre-existing + 17 new; zero regressions)
npm run build            → tsc -b + vite clean (chunk-size warning pre-existing)
npm run lint             → clean
prettier --check         → slice files clean
```

**New vitest coverage** (`worldmap/roadLayer.test.ts` 8 tests, `worldmap/buildingLayer.test.ts` 9 tests):
- Road class table verbatim (colors/widths/dash) + N3 bands: −6…−4 highway/paved/runway only · dirt+track join @ −2 · path @ +4 only (LOD vitest gate).
- `parseRoadsPayload` narrows real payload shape; drops unknown-class/degenerate/malformed rows and non-JSON.
- `obbCorners` @ 0° vs 90° distinct (**R8** rotation-distinct), 360°≡0°, shoelace area rotation-invariant (L2 clockwise-from-north handedness).
- Buildings hidden @ −3, visible @ −2.5/−2 (**LOD3** building half); badges hidden @ 0, visible @ +1.
- `badgeIconKey` incl. bunker-without-prefab-iconKey; prefab lookup keeps buildings only; chunk filter drops trees/unknown/non-finite.
- Construction smoke: `world-roads`/`world-buildings`/`world-building-badges` ids, `pickable:false` everywhere, badge layer null without atlas (R5 degrade), null on empty data.

**One-off integration proof against the real committed export** (temp vitest, run then removed — census parity is the export pipeline's regression surface, not FE's):

```
roads: 766/766 segments parse (394 road_dirt / 367 road_paved / 5 runway)
buildings: 310-prefab lookup → 5,606 instances decoded from 270 indexed chunks
badges: count == type-inventory military+tower+bunker sum
chunk index cells == chunk files on disk (exact set)
```

**Dev-server asset smoke** (`make map-assets-link` + Vite): chunk index / glyph mapping / roads.gz / chunk gz / terrain manifest all 200; missing chunk path returns SPA HTML (200 text/html) — loader guard covers; `.gz` served `Content-Encoding: gzip`.

**Editorconfig:** all slice files pass. Repo-wide `make verify-editorconfig` fails on 4 **pre-existing, untouched** files (`missions_compiled_integration_test.go` spaces-vs-tabs, `everon-sat.tbd-sat` locally-materialized LFS binary, `sap-seam-metrics.mjs`, vendored `bcdec.h`) — predates this slice, not introduced here.

## Manual gates — PENDING-OPERATOR (in-browser)

`VITE_WORLDMAP_ENABLED=1 make web` → dev-login → open any mission in the editor (Everon):

| ID | Check | Expected |
|----|-------|----------|
| R1 | Roads @ default zoom −2, basemap on | paved (grey) + dirt (brown dash) + runway strokes over sat |
| R2 | `tbd-mc-world-layers` roads toggle off | `world-roads` gone (buildings stay) |
| R3 | Buildings @ ≥ −2.5 | grey OBB rects, #888 stroke; military tint #a08060 |
| R5 | Pan sweep, FpsCounter | ≥55 fps with P1 data loaded |
| Z1–Z6 | Screenshots @ −6/−4/−2/0/+3/+6 | per N3 rows: −6 highway/paved/runway only, no buildings · −4 same · −2 +dirt/track +building rects · 0 same · +3 badges visible (≥+1) · +6 all but nothing extra (path class absent from Everon export) |
| M-reg | Flag OFF (`make web`) | zero world layers, sat/hillshade/grid byte-identical to T-090.5.1 behavior |

Code-level equivalents of R1–R3 + LOD bands are covered by the vitest gates above (precedent: T-090.5.1 M-gates, T-090.1.1 M6/M9). Note Z-ladder: Everon export has no `highway_paved`/`track`/`path` segments (394 dirt / 367 paved / 5 runway), so band steps visible in practice are paved+runway (all zooms) → +dirt @ −2.

## Follow-ups (not this slice)

- T-090.5.3: worker chunk streaming + chunkStore LRU + `visibleInstances` (this loader shrinks to the manifest gate); INSTANCE_BUDGET census vitest.
- T-090.5.5: tree/veg/prop glyph layers (atlas already carries tree/veg/rock/prop/utility sprites).
- T-090.9: pick wiring (`PICK_RADIUS_PX`, worker `pickNearest/pickRect`).
- `WorldLayerToggles.tsx` UI (prefs are live via localStorage; surfaced control ships with the toggles panel deliverable).

---

# T-090.5.2.1 — Road centerline extraction + visual quality pass

**Date:** 2026-07-05 · **Trigger:** operator in-browser review of T-090.5.2 ("roads and buildings look shit, but right direction") with screenshots.

## Diagnosis

1. **Roads drew as "centipedes"** — `roads.json.gz` polylines are NOT centerlines but **road-surface quad soup**: alternating cross-edge point pairs, every second cross-edge duplicated. Measured: even-indexed steps near-constant per class (runway **20.0 m**, paved **4.0 m**, dirt **1.75 m**; p10/p90 tight), 41,758 / 169,346 steps are sub-cm duplicates (≈ every 4th), only 8 true direction reversals. PathLayer connecting the raw sequence rendered perpendicular ticks along every road.
2. **Buildings read as ghost wireframes** — spec colors (fill rgba(120,120,130,0.35), stroke #888 @ 1 px) wash out over the dimmed satellite.
3. **Rotation checked — NOT a bug:** export `rotationDeg = GetAngles()[1]` (ops-log `handednessRemap`), left-handed Y-up ⇒ clockwise-from-north, matches glyphs-spec L2 + `obbCorners` + T-092 spawn parity.

## Fix (render-side; export untouched)

- `extractRoadCenterline` (pure, `roadLayer.ts`): midpoint each cross pair → centerline vertex, collapse <0.05 m dups, median pair length = **measured true width** per segment; `parseRoadsPayload` applies it (`RoadSegment.widthM`, style-table width as sanity fallback).
- Roads render at **measured geometric width** + **near-black casing** (`world-roads-casing`, +40% width, under `world-roads`) — operator-approved style pass. **Deviations from contract/spec for Cursor doc sync:** road width source = measured data (not the px@z0 table; table keeps colors/dash + fallback), new layer id `world-roads-casing` (plan §4.2 addendum).
- Buildings: solid dark blocks — fill `rgba(38,38,44,0.72)`, stroke `rgba(150,150,158,0.8)` 1 px, military `#7a5c3d` (supersedes t090_5 ghost values — doc sync).

## Verification

```
vitest --run             → 13 files / 106 tests (6 new: quad-soup centerline, dedupe, odd-point guard,
                           <2-midpoint null, junction-flare median width, casing+road construction smoke)
one-off real-data sweep  → 766/766 segments centerline; residual dup steps 0;
                           median widths runway 20 / paved 4 / dirt 1.75; >50% vertex reduction (temp test, removed)
npm run build / lint / prettier → clean
```

Operator re-check in browser (Vite HMR, flag on): roads as clean cased strokes at true width, dirt dashed; buildings as solid dark footprints.

## Follow-up for export lane (T-090.3.x, not FE)

`decode-topo` road extraction emits quad soup — a future export slice can emit true centerlines + `widthM` directly and drop `extractRoadCenterline`; FE contract already matches that shape.

---

# T-090.3.3 + T-090.5.2.2 — Data-driven structure taxonomy + missing highway network

**Date:** 2026-07-05 · **Trigger:** operator review of T-090.5.2.1 — main asphalt highway absent; lighthouse/castle/harbor sites garbled box heaps; bridges/railings/benches/signs/fences classified as buildings. Directive: enumerate the REAL structure types from the data and categorize everything.

## Findings

- **Missing highway = mislabeled .topo types.** decode-topo's legacy names call type 1 "RIVER" (12 recs, **12 m constant width, 19.7 km**, central-valley chain) and type 2 "STREAM" (110 recs, **8 m, 57.9 km**) — but G1-B (T-090.1.2.5.2) already proved the .topo carries roads only. Engineered constant widths confirm: types 1+2 are the asphalt highway/main-road network. Old export mapped only {0,3,5} → highway never reached `roads.json.gz`.
- **Classify fallback dumped everything**: greedy needles (`House`, `Village`) put HouseRuin brick piles, SignVillage, GuardHouse into `residential`; `Structures/Cultural/` → 694 cemetery instances as `civic`. Confirmed on-map: the Morton_Peninsula "building" heap was 33 HouseRuin rubble rows.
- **Raw structure taxonomy** (1.41M-row staged export, offline — no Workbench): Walls 37.5k · Infrastructure 15.2k (Pavements 7.9k, Power 2.7k, Naval/piers 2.3k, Lamps, **Bridges 113**, Railways 112) · Signs 6.4k · **BuildingParts 3.2k** (composite-building part soup) · Ruins 1.5k · Cemeteries 694 · plus Lighthouse ×10, Castle ×~135, ConcreteBridge ×~160, Piers ~1.5k, Sheds ~700, Containers ~600, Tents ~40, GuardTower/DeerStand/GuardHouse, Benches ~670.
- **`World/Locations/Eden/*.et`** = ~180 whole-POI composition entities (towns, bays, hills) with halfExtents [0,0,0] — children export as their own rows; compositions must never render.
- **Prefab OBBs were rule-template constants** (every residential 5×5 m) — the raw rows carry real per-entity `halfExtentsM` that the export discarded.

## Shipped — T-090.3.3 (export lane, offline from staged raw)

1. **Roads**: `build-roads-from-topo.mjs` full type mapping `{0: runway, 1: highway_paved, 2: road_paved, 3: road_dirt, 5: track}` (semantic; provisional flag dropped). `roads.json.gz` → **888 segments** (5 runway / 12 highway / 110 paved / 367 dirt / 394 track). decode-topo untouched (frozen consumers).
2. **Classification**: `prefab-classify.json` rebuilt — 34 new path-prefix rules ahead of the repaired residential rule (`Structures/Houses/` only), Structures catch-all → `building/generic`. Walls→prop/fence, Signs→prop/sign, BuildingParts→prop/buildingpart, Pavements/Railways→prop, Power/Lamps/Piping→utility, Naval→**water/pier**, Bridges→**building/bridge**, Cemeteries/Calvaries→prop/monument, Castles→**building/castle**, Lighthouse→**building/lighthouse**, ruin piles→prop/debris, ruin shells→building/ruin, Shed*→**shed**, Garage→garage, containers→**container**, tents→**tent**, Guard/Deer/Control/Transmitter/Water towers→tower, GuardHouse/Box→military, benches/boards/beds→prop/furniture, Anthill→rock/boulder, fuel/silo/factory/cranes/transformers→industrial, FuelStation→commercial, `World/Locations/`→prop/**composition** (never rendered). 32-case classification matrix vitest-style smoke: 32/32.
3. **Schema/goldens/glyphs**: enums += buildingClass{bridge,castle,lighthouse,shed,container,tent} + propClass{buildingpart,pavement,rail,monument,composition}; 11 golden prefab rows (S9 coverage); 9 new `building-*` SVGs (bridge/castle/lighthouse/shed/container/tent/ruin/garage/generic); atlas rebuilt **28 glyphs @ 1024×512 (21.3 KB)**.
4. **Export**: `build-world-objects.mjs` — P2 kinds += `water`; **measured prefab OBBs** (per-axis median of sampled raw halfExtents, engine→map remap, rule fallback for degenerate bounds). Full rebuild: **391 prefabs / 508,291 instances / 275 chunks** (buildings 4,131 across 18 populated classes + piers 2,299 + trees 501,861 unchanged). Building footprints now real: 0–4,066 m², median 84 m².
5. **Gate repairs** (taxonomy-driven): verify-phase PHASE_KINDS += water; P1-2 tent exemption (canvas = soft cover by design); anchor-check co-located tie fix (stacked containers share x/y — nearest-match now accepts the anchor's own prefab within tolerance).

**Gates — ALL PASS:** `map-export-validate` · `map-verify-phase` P1 (incl. E6 determinism double-build) + P2 · `map-census` · `make schema-validate` (enums / golden S2–S14 / glyphs incl. everon coverage + atlas / type-inventory / t090-specs 12/12).

## Shipped — T-090.5.2.2 (render lane)

- `buildingPrefabLookup` includes `water` pier/dock (walkable structures) alongside buildings; buoys/trees/props stay filtered.
- Per-class fills over the solid-dark default: bridge grey-blue, pier/dock timber, ruin faded, castle stone-brown, **lighthouse white + red stroke**, container steel-blue, tent olive, shed/garage dark, military sand-brown.
- Vitest **107/107** (pier-inclusion, buoy exclusion, class-fill mapping); FE build + lint + prettier clean.
- Sweep vs new export: 888 road segments parse; 6,430 footprint instances (4,131 buildings + 2,299 piers).

## Doc-sync flags (Cursor)

- Enum additions (6 buildingClass + 5 propClass) → `t090_2_map_object_taxonomy.md` / enums docs.
- Road class mapping now semantic + `classMappingProvisional: false` (plan decision 5 closed).
- P2 phase kinds include `water`; prefab `spatial` now measured (t090_phased_object_import / prefab schema docs).
- P1-2 tent exemption + anchor tie semantics in the phase-gate doc.

## Operator re-check (browser, flag on)

Expect: main asphalt highway + secondary asphalt visible from whole-island zoom; gravel net brown-dashed from −2; castle/lighthouse/harbor no longer box heaps (parts are props now — excluded until T-090.5.5); bridges/piers visible as distinct tinted structures; building rects at true footprint sizes.
