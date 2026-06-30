# T-090 Program Audit

**Date:** 2026-06-30 · **Auditor:** Claude Code (read-only) · **Scope:** T-090 slices .0–.7 + living reference
**Verdict:** Data/export/AI half is **strong**; the **render + interaction + forest** half is **the
risk**, and the **active slice (T-090.1) is blocked on schema/data that don't exist yet**.

> Resolution status (2026-06-30): every gap below is closed by the program rewrite — owner constants
> **N1–N12** + new slices **T-090.0.2 / .3.0 / .8 / .9**. See the **Audit closure table** in
> [`t090_091_map_terrain_program.md`](../../docs/specs/Mission_Creator_Architecture/t090_091_map_terrain_program.md)
> (GAP-ID → spec → verify gate → slice).

## Executive summary (top risks if we ship the pre-rewrite specs as-is)
1. **Forests not a first-class concept anywhere.** 400k–900k Everon trees modelled only as point
   instances + supercluster discs. No forest polygon/density/region type in taxonomy, export, or render.
   Owner's #1 non-negotiable has no spec. (GAP-001)
2. **Render LOD ladder in the wrong zoom space.** Specs use tile z0–5; editor viewport is Deck ortho
   [-6,6] default -2. At default view, glyphs (lodMinZoom 3) never render. Unbuildable. (GAP-002)
3. **World-object hover/inspect — an owner requirement — is "out of scope" of T-090.5 with no UI spec.**
   T-090.7 specs only a data API. (GAP-003)
4. **Live render arch forbids the specs' picking model.** Deck onHover/onClick removed in T-057
   (getCursor constant, rbush pick). T-090.5 "Deck pick on world layers" contradicts shipped code.
   (GAP-003)
5. **Active T-090.1 references a dual-pyramid manifest (`tiles.satellite`) neither schema nor live
   manifest has.** Extension mis-assigned to later .1.1. (GAP-004)
6. **The object program rests on an unproven Workbench plugin** (enumerate ~1M entities, read OBB,
   capture ortho + cartographic tiles) — same risk that killed manual height export in T-091.0.
   (GAP-005)
7. **"Reuse slotClusterIndex" impossible** — slots-only module singleton; world trees need a separate,
   worker-hosted index. (GAP-H3/H4)
8. **T-090.3 manifest example fails the live schema** (`tiles.satellite`, `dem.width/height`, missing
   `metersPerPixel`/`precision`). (GAP-004/M1)
9. **No legend, filter/search, inspect panel, or per-phase render/memory budget** — the power-user half
   is unspecified. (GAP-H6/H7/H8)
10. **Toggle persistence inconsistent** — specs use localStorage; live grid/hillshade use per-mission
    `meta.environment`. (GAP-H5)

## Scorecard
| Slice | Completeness | Biggest gap |
|-------|-------------|-------------|
| T-090.1 Satellite basemap (active) | 65% | Depends on dual-pyramid manifest schema+data that don't exist; mis-ordered (GAP-004) |
| T-090.1.1 Dual basemap view | 75% | Schema not extended; persistence conflict (GAP-H5); Map source unproven (GAP-H1) |
| T-090.2 Taxonomy/schema | 75% | No forest/region type (GAP-001); schemas + `objects` block unwritten |
| T-090.3 Export pipeline | 60% | Workbench feasibility hand-waved (GAP-005); no forest export; manifest example invalid (GAP-004) |
| T-090.4 Z pivot audit | 85% | Solid; trust not surfaced (GAP-M3) |
| T-090.5 Render layer | 40% | Wrong zoom space (GAP-002); no forest render (GAP-001); hover/pick out of scope + arch conflict (GAP-003); cluster reuse impossible (GAP-H3) |
| T-090.6 Geometry audit | 85% | Solid; depends on unproven bbox export (GAP-005) |
| T-090.7 Eden AI read path | 65% | API only, no human inspect UI (GAP-003); query impl unspecified; context pack explodes on forests (GAP-M6) |
| Living ref | n/a | "vegetation mass reads as forest" promised, no slice delivers it |

## Critical gaps (P0)
**GAP-001 — Forests as first-class AREAS unspecified** (data+export+render). Add `kind=forest`
(+field,waterBody) region geometry (polygon + treeCount + dominantSpeciesClass + densityPerHa + areaHa +
coverType); export `forest-regions.json.gz` from tree density + hull OR engine mask; render translucent
polygons low/mid zoom, dissolve to glyphs high zoom. Owner slice NEW T-090.8. Verify: region golden
validates; Σ treeCount ≈ byKind.tree.instances; render smoke shows filled polygon @ zoom ≤ −3.
**GAP-002 — LOD wrong zoom space** (render). Re-express in Deck zoom; define REF_ZOOM/WORLD_CLUSTER_*;
reconcile with slot constants. Verify: at zoom −2 forests=polygons, trees=clusters.
**GAP-003 — Hover/inspect required but out of scope + arch conflict** (interaction+AI+UX). Separate world
rbush in worker; hover on container pointermove → tooltip; click → read-only inspect + ask-AI;
slot-wins precedence, Alt forces world. Owner slice NEW T-090.9. Verify: hover building shows
class/summary; world click doesn't change slot selection; ≥55 fps.
**GAP-004 — Active .1 depends on dual-pyramid manifest schema+data that lack; mis-ordered**
(export/data). Land dual-tiles+objects schema before/with .1; fix .3 example; migrate everon manifest.
Verify: dual + live manifests pass schema-validate.
**GAP-005 — Workbench export feasibility unproven** (export). NEW T-090.3.0 spike proves enumerate + OBB
+ 1 ortho + 1 map tile + forest-mask probe before P1–P10. Verify: spike artifacts in ops log.

## High gaps
GAP-H1 Map cartographic source unproven yet vital → specify synthesized fallback. GAP-H2 footprint vs OBB
→ pick one (OBB rectangles default). GAP-H3 slotClusterIndex is slots-only singleton → separate
worker-hosted world index; quantify load() @ 900k. GAP-H4 worker offload named but unspecified → add
worldObjects.worker.ts (fetch+gunzip+parse+rbush, Comlink) modeled on T-066. GAP-H5 persistence split →
pick one + document. GAP-H6 no legend → generated Legend panel. GAP-H7 no filter/search → panel wired to
the same WorldObjectFilter the AI uses. GAP-H8 no per-phase render/load/memory budget → targets +
residency model.

## Medium gaps
GAP-M1 manifest `additionalProperties:false` rejects objects/satellite/map until extended. GAP-M2 tile
storage/fetch budget loose+conflicting; add eviction/concurrency/cold-load. GAP-M3 Z-trust not surfaced;
add badge + optional fail-highlight. GAP-M4 road dashing — PathLayer no native dash; name PathStyleExtension.
GAP-M5 enum coverage drift; single-source in map-object-enums. GAP-M6 AI context pack explodes on forest
bbox; return region summaries + cap. GAP-M7 no empty/"export-not-run" state for object layers (Arland).

## Low gaps
L1 atlas size budget unstated. L2 rotationDeg zero/handedness + localUp→Z remap unreconciled. L3
type-inventory.json UI-driving but unconsumed (AI-only). L4 accessibility (color-only roadClass). L5
Arland registry ↔ terrains.ts consistent (noted).

## Forest & vegetation deep-dive
By Deck zoom (default −2): −6…−3 forest polygons only; −3…−1 polygons + density; 0…+2 outline fades,
cluster discs + prominent glyphs; +3…+6 individual rotated glyphs + low-α fill context. Export
`objects/forest-regions.json.gz` (polygons + treeCount + dominantSpecies + densityPerHa + areaHa +
coverType) derived from tree density + concave/alpha hull OR a Workbench vegetation/forest mask; keep
per-tree instances for high zoom. Hover region → "Mixed conifer forest · ~12,400 trees · 38 ha · soft
cover"; click → species breakdown + ask-AI. Comparison: SAP-only (pretty, not queryable); per-tree
overlay (truthful, 900k = perf death + mud); **hybrid (recommended)** — imagery basemap + typed forest
polygons low/mid zoom + per-tree glyphs high zoom: simpler than Eden for beginners, deeper for power
users. Reforger-native: confirm whether Enfusion forest/vegetation generator layers export as
polygons/raster and ingest directly (the GetSurfaceY lesson).

## World-object interaction deep-dive
Ships in T-090: passive hover tooltip, click→inspect (read-only resolved + ask-AI), per-kind toggles,
class sub-filter, prefab search, legend, Z-trust badge. Defer correctly: moving/deleting world props
(Workbench), floor picker (T-126). Picking: new worldSpatialIndex (rbush) per loaded chunk-set in
worldObjects.worker.ts; layer ids world-forest|roads|buildings|trees|props; no Deck picking re-enabled;
world hover on the existing container pointermove that feeds emitCursor. Conflict: slot tools unchanged;
click precedence slot-first then world; Alt forces world; marquee slots-only. Preserves T-057/T-063 perf
contracts.

## New slice proposals
T-090.3.0 Workbench export spike (de-risk before P1–P10); T-090.8 Forest & vegetation regions
(type+export+render+inspect); T-090.9 World-object hover + inspect + filter/search + legend.

## Spec contradictions found
1. z0–5 vs Deck −6…6 (GAP-002). 2. T-090.3 manifest example vs live schema (GAP-004/M1). 3. T-090.5 Deck
pick/onHover vs live no-pick arch (GAP-003). 4. "reuse slotClusterIndex" vs slots-only singleton
(GAP-H3). 5. localStorage vs meta.environment (GAP-H5). 6. .5 "pick/select world objects (future)" vs
owner requirement + .7 id-keyed "move/delete this object" (GAP-003).

## Questions for product owner (resolved by owner decisions D1–D10 / constants N1–N12)
1. Forest source (derive vs engine mask). 2. Toggle persistence per-mission vs per-user. 3. World-object
editing in MC scope? 4. Synthesized Map acceptable?

## Recommended implementation order change
0 schema prereq (T-090.0.2); 1 .1 Satellite; 2 NEW .3.0 spike; 3 .2 taxonomy + forest type; 4 .3 export
incl. forest-regions + dual tiles; 5 .4/.6 audits; 6 .5 render forests-first then buildings then glyphs
(Deck-zoom LOD); 7 NEW .9 hover/inspect/filter/legend; 8 .7 AI; 9 .1.1 Map view after schema prereq.

## Appendix: files read + code paths inspected
Specs (15): all `t090_*.md`. Registry T-090. Handoff. Live code: TacticalMap.tsx, useBaseMapLayer.ts,
useIconLayer.ts, slotSpatialIndex.ts, slotClusterIndex.ts, constants.ts, useOrthographicView.ts,
terrains.ts, MissionSettingsDialog.tsx. Schemas/rules/data: terrain-manifest.schema.json,
prefab-classify.json, live everon/arland manifests, map-assets tree (only dem/+anchors/), empty golden/,
absent terrain-registry.json + map-object-*.schema.json. engineering_plan.md §4. Validate wiring:
validate.mjs (ajv strict; explicit per-file checks; golden auto-discovered; manifest validated against
live everon manifest).
