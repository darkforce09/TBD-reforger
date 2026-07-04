# T-144 — Arma 3 map architecture study (reference analysis)

**Ticket:** T-144 · **Program:** map reference · **Route:** `/missions/:id/edit` (informs **T-090**)  
**Status:** **ready** — **active slice:** **T-144.1** (Claude Code research)  
**Executor:** T-144.0 cursor-docs · **T-144.1** claude-code  
**Registry:** [`.ai/tickets/registry.json`](../../../.ai/tickets/registry.json)

---

## Why this ticket exists

T-090 basemap work (Satellite + Map tiles, heuristic land-cover, `.topo` roads) diverges from how **Arma 3 Eden** actually builds map readability. Operator has **Arma 3 engine source** access and wants a **ground-truth architecture study** before more T-090 implementation — not a port, but the **standard** for data flow, layering, zoom/LOD, and object draw.

**Pauses T-090 implementation slices** (`.1.2.9` onward) until this study lands and Cursor doc-syncs recommendations into the T-090 hub.

---

## Source tree (read-only — outside monorepo)

```text
/home/Samuel/Projects/TBD_Arma_3_Remaster/Arma_3_SourceCode_Old/
```

**Do not commit to that repo.** Analysis artifacts live in **this** repo under `.ai/artifacts/`.

---

## Scope

| In scope | Out of scope |
|----------|----------------|
| **2D map viewing** — raster layers, zoom, pan, coord transform | Porting C++ to TypeScript |
| **Mission / Eden editor map** (`DisplayArcadeMap`, `CStaticMapArcadeViewer`) | 3D `ui3DEditor` placement loop (mention only) |
| **Data sources** — what textures/charts/objects feed the map | Event editor game logic unrelated to map |
| **Object draw / LOD / clustering** on the 2D map | Multiplayer, JIP, briefing UI chrome |
| **World query** — how engine resolves terrain height, objects in view | Reforger Enfusion APIs (compare in §Delta only) |
| **Comparison table** — A3 vs current T-090 stack | Implementing fixes (follow-on T-090 slices) |

---

## Discovery first (do not trust this doc’s file names)

**We do not know** that `uiMap.hpp` (or any path in TBD-Reforger docs) is the authoritative entry point until **you prove it from the A3 tree**. Cursor’s prior grep was a **hypothesis**, not ground truth.

**Phase 0 — Entry point discovery** (mandatory; log in report §0):

1. **Orient** — list top-level dirs under `Arma_3_SourceCode_Old/`; note build layout (`lib/`, `lib/UI/`, tools, extern).
2. **Search by behavior**, not filename — examples:
   - mission editor / arcade map / 2D map / map control / draw map
   - satellite, user chart, map chart, DrawField, zoom map
   - `_ENABLE_EDITOR`, Eden, EditorObject, landscape map
3. **Validate candidates** — for each hit: who calls it? is it **mission editor 2D** vs in-game GPS map vs briefing map vs 3D editor?
4. **Reject dead ends** — log paths that looked promising but are wrong surface (briefing chrome, UAV terminal map, VBS-only, `#if 0`, etc.).
5. **Name the spine** — once validated, state the **primary call chain** (display class → map widget → draw → data source) with evidence.

Write **§0 Entry point discovery** in the report *before* architecture sections. Include a table:

| Candidate path | Why searched | Verdict (primary / secondary / reject) | Evidence |

**Hypotheses to verify** (may be wrong — confirm or debunk):

| Hypothesis | If true, you should find… |
|------------|---------------------------|
| `lib/UI/uiMap.*` | 2D map widget, `DrawField`, editor `DisplayArcadeMap` |
| `lib/UI/missionEditor.*` | Editor shell opening the map display |
| `lib/editor.*` | Landscape/world data feeding map (may be bulldozer-only) |
| `lib/world.cpp` | `GetMap()`, terrain singleton wiring |
| Separate `mapViewer` tool | Pipe in `gameStateExt.cpp` — legacy or parallel path? |

Only after §0 is done: trace raster, objects, coords, zoom from **your** validated spine — not from this table alone.

---

## Deliverable

**Primary:** [`.ai/artifacts/t144_arma3_map_architecture_report.md`](../../../.ai/artifacts/t144_arma3_map_architecture_report.md)

Required sections (headings mandatory):

0. **Entry point discovery** — search log + validated spine (see §Discovery first); explicitly answer “is `uiMap.hpp` the right place?”  
1. **Executive summary** — 10 bullets: how A3 map *actually* works vs our T-090 assumptions  
2. **Architecture diagram** — mermaid: data sources → compose → GPU/UI draw → editor overlays  
3. **Raster stack** — satellite vs “map chart” vs grid; config keys; file formats  
4. **Coordinate systems** — world X/Z ↔ map pixels; origin, scale, rotation  
5. **Zoom & LOD** — what changes at each zoom band (icons, roads, forests, labels)  
6. **World objects on map** — taxonomy, indexing, cull, cluster/simplify  
7. **Editor-specific** — `DisplayArcadeMap` vs in-game `DisplayMap`; placement pick hit-test  
8. **Terrain / height** — what the 2D map uses vs 3D `GetSurfaceY`  
9. **Gap analysis** — table: A3 behavior | T-090 today | Recommended T-090 change | Ticket |
10. **Phased recommendation** — reorder or replace T-090.1.2.9 / T-090.3 / T-090.5 based on findings  
11. **File index** — top 30 source files with one-line role  

Optional: [`.ai/artifacts/t144_arma3_map_callgraph.md`](../../../.ai/artifacts/t144_arma3_map_callgraph.md) if call depth warrants split.

---

## Verification

| ID | Check | Pass |
|----|-------|------|
| **R1** | Report exists with all **12** sections (§0–§11) | Yes |
| **R2** | §0 documents ≥3 search strategies + verdict on `uiMap.*` hypothesis | Yes |
| **R3** | Gap table references live T-090 slices by id | Yes |
| **R4** | ≥15 primary source files cited with path + symbol (from **your** discovery, not this spec) | Yes |
| **R5** | No edits under `Arma_3_SourceCode_Old/` | Yes |
| **R6** | No edits to `apps/`, `packages/`, `docs/` in TBD-Reforger (Cursor sync after) | Yes |

Log: **`.ai/artifacts/t144_verify_log.md`**

---

## Ship

| Item | Value |
|------|-------|
| T-144.0 | cursor-docs — this spec + handoff (no tag) |
| T-144.1 | claude-code — report @ commit prefix **`T-144.1:`** · tag **`T-144.1`** |
| Post-ship | Cursor doc sync — fold recommendations into [`t090_091_map_terrain_program.md`](t090_091_map_terrain_program.md), [`t090_eden_map_reference.md`](t090_eden_map_reference.md), resume/unblock T-090 slices |

---

## Related

- T-090 hub: [`t090_091_map_terrain_program.md`](t090_091_map_terrain_program.md)  
- Eden UX target: [`t090_eden_map_reference.md`](t090_eden_map_reference.md)  
- External repo: `TBD_Arma_3_Remaster/Arma_3_SourceCode_Old` (sibling project)

---

## Claude Code prompt — T-144.1 (copy-paste)

Authority: this spec + handoff. **Do not edit docs/registry in TBD-Reforger.**

```
Read CLAUDE.md first (TBD-Reforger repo).

Execute **T-144.1** — Arma 3 map architecture study (read-only source analysis).

═══ PREFLIGHT ═══
  cd /home/Samuel/Projects/TBD-Reforger
  git pull
  ./scripts/ticket brief T-144 --slice T-144.1
  Confirm external tree exists:
    test -d /home/Samuel/Projects/TBD_Arma_3_Remaster/Arma_3_SourceCode_Old/lib

═══ READ (in order) ═══
  1. .ai/artifacts/t144_claude_code_handoff.md
  2. docs/specs/Mission_Creator_Architecture/t144_arma3_map_architecture_study.md
  3. docs/specs/Mission_Creator_Architecture/t090_091_map_terrain_program.md (compare only)
  4. docs/specs/Mission_Creator_Architecture/t090_eden_map_reference.md

═══ PROBLEM ═══
  We may be building T-090 wrong — SAP heuristics, offline compose, Deck tiles — because
  we never traced how A3 Eden actually implements the 2D mission editor map. Discover the
  real entry points in source (uiMap.hpp is a **hypothesis**, not gospel).

═══ LOCKED ═══
  - **Discovery first** — Phase 0: find + validate spine before deep tracing
  - Report **§0** must verdict: is `lib/UI/uiMap.*` primary, secondary, or wrong?
  - Read-only on Arma_3_SourceCode_Old/** · no port · artifacts in .ai/artifacts/ only
  - Focus mission editor **2D map** — not briefing/GPS/UAV/3D-only paths
  - No edits to docs/registry/app code in TBD-Reforger

═══ DO ═══
  1. Phase 0 — broad rg searches (handoff); validate/reject candidates; write §0 discovery log
  2. From **your** spine only: raster stack, coords, zoom, objects on map
  3. Editor pick + terrain height usage on 2D canvas
  4. Gap table vs T-090 slices + phased recommendation
  5. `.ai/artifacts/t144_arma3_map_architecture_report.md` (§0–§11)
  6. `.ai/artifacts/t144_verify_log.md` (R1–R6)
  7. Commit artifacts only · prefix **T-144.1:** · tag **T-144.1**

═══ DO NOT ═══
  - Skip §0 or assume uiMap.hpp without evidence
  - Modify A3 source or implement T-090 fixes
  - Edit docs/registry (Cursor sync follows)
  - Deep-dive briefing/GPS/radio chrome

═══ VERIFY ═══
  Self-check R1–R6 in t144_verify_log.md

═══ RETURN ═══
  - SHA + tag T-144.1
  - §0 one-liner: confirmed spine vs uiMap hypothesis
  - Top 5 surprises vs T-090 · recommended program reorder
  - **Ready for Cursor doc sync.**
```
