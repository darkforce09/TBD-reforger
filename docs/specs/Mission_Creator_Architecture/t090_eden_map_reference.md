# T-090 — Eden / Arma 3 editor map UX reference

**Status:** living reference (not a slice)  
**Audience:** AI agents + mission makers — **what “good” looks like** for map detail  
**Authority:** [`t090_091_map_terrain_program.md`](t090_091_map_terrain_program.md)

---

## Why this doc exists

Product goal: when you open the Mission Creator map, you should see **enough world context** to place units and objectives confidently — similar to **Bohemia Eden Editor (Arma 3 / Reforger Workbench)**. This doc captures what Eden does well, what it does poorly, and how T-090 slices map to parity.

**Not in scope for T-090:** building **floor selector** (which floor am I editing?) — deferred to **T-129** (`idea`).

**Architecture authority:** **T-144.1** shipped @ `b1949182` — [`.ai/artifacts/t144_arma3_map_architecture_report.md`](../../../.ai/artifacts/t144_arma3_map_architecture_report.md) · [`t144_arma3_map_architecture_study.md`](t144_arma3_map_architecture_study.md). Key A3 lesson: **no basemap tiles** — map drawn live from world data; Sat↔Map = zoom crossfade + toggle; vectors (roads, forests, objects) always on top. T-090 keeps our raster basemaps for web perf but **prioritizes export + vector layers** per report §10.

---

## Eden strengths (copy these)

| Pattern | Eden behavior | T-090 target |
|---------|---------------|--------------|
| **Satellite / color basemap** | Aerial ortho under the grid | **T-090.1** `tiles/satellite` |
| **Map / cartographic basemap** | Styled map (Google Maps “Map”) | **T-090.1.1** `tiles/map` — **vital** |
| **Basemap view switch** | User picks Satellite **or** Map | Mission Settings radio — [`t090_basemap_dual_view.md`](t090_basemap_dual_view.md) |
| **Road readability** | Major roads visible at medium zoom; minor tracks fade | **T-090.5** typed road layer + width/color by `roadClass` |
| **Structure silhouettes** | Building footprints / roofs visible @ zoom | **T-090.5** `building` instances (simple fill or icon) |
| **Vegetation mass** | Tree clusters read as “forest” without 1:1 icons @ low zoom | **T-090.8** marching squares on density grid; **T-090.5** density-gate LOD (A3: no clustering) |
| **Layer toggles** | User can hide categories (units, objects, triggers…) | **T-090.5** MC settings: basemap / roads / structures / vegetation |
| **Horizontal alignment** | Map matches in-game coords | **T-090.1** H1/H2 gates + manifest `alignmentOrigin` |

---

## Eden weaknesses (do better or accept tradeoffs)

| Issue | Eden pain | T-090 response |
|-------|-----------|----------------|
| **Clutter @ high zoom** | Too many similar icons; hard to pick one prop | Typed labels + filter by category; spatial index pick (T-063) |
| **Z ambiguity** | Objects float or bury; hard to see without 3D view | **T-090.4** pivot screen + **T-090.6** OBB “visible above ground %” @ 1M objects (no manual verify) |
| **Performance** | Large maps stutter with all objects visible | Viewport cull + LOD (**T-112** idea); mission slots separate from world base (**T-110**) |
| **No floor slice** | Multi-story buildings edit as one blob | **T-129** future — explicit out of T-090 |

---

## Visual stack (target z-order, bottom → top)

```text
1. DEM hillshade (optional, T-091.2 — shipped)
2. Active basemap — **Satellite OR Map** (T-090.1 / T-090.1.1) — mutually exclusive
3. Procedural grid (optional)
4. Static world objects — roads, buildings, trees (T-090.5)
5. Authored mission entities — slots, markers, vehicles (T-069+)
6. Selection / marquee overlay
```

---

## Beyond Eden — shipped in T-090 (not aspirations)

| Capability | Eden | T-090 |
|------------|------|-------|
| Forests as queryable **areas** | mass of icons | **T-090.8** forest-region polygons (typed, counted, AI-queryable) |
| Hover a prop → identity / cover / AI summary | weak | **T-090.9** hover tooltip + read-only inspect + "Ask AI" |
| Legend / symbology | none | **T-090.9** generated legend (glyph + roadClass + forest + Z-trust) |
| Filter / search by taxonomy | category toggles | **T-090.9** kind→class filter + prefab search + within-selection |
| Z trust (buried/floating) | invisible | **T-090.4/.6** audit + **T-090.9** Z-trust badge |

Each is a program deliverable with a verify gate (see the hub Audit closure table) — not a future idea.

## AI agent notes

- **Do not conflate** raster tiles (T-090.1) with **vector/instance world objects** (T-090.2–.5). Tiles are imagery; objects are structured data with taxonomy.
- **Cost:** Everon full object extract may be **100k–1M+** instances — see [`t090_2_map_object_taxonomy.md`](t090_2_map_object_taxonomy.md) §Cost model. Render uses LOD, not 1M Deck icons @ once.
- **Spawn authority** remains mod `GetSurfaceY` (T-092) — map object Z is **visual**, not spawn truth.
