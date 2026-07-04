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

**Exploration scope:** the **entire** `Arma_3_SourceCode_Old/` tree is in bounds — `lib/`, `cfg/`, tools, `extern/`, scripts, resources. Do **not** artificially limit yourself to `lib/UI/uiMap.*`. Follow every validated call chain across modules (landscape, world, config, texture load, visitor, tools). Complexity is expected; thoroughness beats speed.

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

Optional deliverables (use when complexity warrants — encouraged for this slice):

- [`.ai/artifacts/t144_arma3_map_callgraph.md`](../../../.ai/artifacts/t144_arma3_map_callgraph.md) — deep call graph  
- [`.ai/artifacts/t144_arma3_map_data_flow.md`](../../../.ai/artifacts/t144_arma3_map_data_flow.md) — raster + object + config pipeline detail  
- [`.ai/artifacts/t144_arma3_search_log.md`](../../../.ai/artifacts/t144_arma3_search_log.md) — full discovery journal (queries, hit counts, rejects)

---

## Verification

| ID | Check | Pass |
|----|-------|------|
| **R1** | Report exists with all **12** sections (§0–§11) | Yes |
| **R2** | §0 documents ≥3 search strategies + verdict on `uiMap.*` hypothesis | Yes |
| **R3** | Gap table references live T-090 slices by id | Yes |
| **R4** | ≥15 primary source files cited with path + symbol (from **your** discovery, not this spec); **≥40** if full-tree exploration was needed | Yes |
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

Execute **T-144.1** — Arma 3 map architecture study (read-only, **exhaustive**).

═══ PREFLIGHT ═══
  cd /home/Samuel/Projects/TBD-Reforger
  git pull
  ./scripts/ticket brief T-144
  ARMA=/home/Samuel/Projects/TBD_Arma_3_Remaster/Arma_3_SourceCode_Old
  test -d "$ARMA/lib" && test -d "$ARMA/cfg" || exit 1
  # Orient the whole tree — not just lib/UI
  find "$ARMA" -maxdepth 2 -type d | head -60

═══ READ (in order) ═══
  1. .ai/artifacts/t144_claude_code_handoff.md
  2. docs/specs/Mission_Creator_Architecture/t144_arma3_map_architecture_study.md
  3. docs/specs/Mission_Creator_Architecture/t090_091_map_terrain_program.md (compare only)
  4. docs/specs/Mission_Creator_Architecture/t090_eden_map_reference.md

═══ PROBLEM ═══
  T-090 (SAP ortho, offline compose, Deck tiles, heuristic land-cover) may be architecturally
  wrong because we never studied how Arma 3 actually builds the 2D mission-editor map.
  This system is **deep and cross-cutting** — UI, landscape DB, textures, config, world sim.
  Operator authorizes **full-tree read-only exploration** of the A3 source. Take the time
  to be thorough. uiMap.hpp is one hypothesis among many.

═══ COMPLEXITY (explicit permission) ═══
  - **Explore the entire** Arma_3_SourceCode_Old tree — lib/, cfg/, tools/, extern/, etc.
  - **Follow call chains wherever they lead** — landscape, world, texture, visitor, config
  - **Do not stop** at the first plausible file; cross-validate from 2+ independent paths
  - Large files (uiMap.cpp ~32k lines): read by **offset/section**, trace symbols, never whole-file
  - Source is **Arma3_2012** era — classic **Arcade** 2D editor, not 2016 Eden 3D; state caveat
  - Retail vs **VBS-only** (#if _VBS3): validate what ships in retail path
  - It's OK if this takes many exploration passes — completeness > speed
  - **Parallel subagents encouraged** — see handoff §Parallel agents; parent synthesizes one report

═══ PARALLEL AGENTS (operator-approved — use when runtime is long) ═══
  You MAY spawn **read-only** parallel explorers after a 15-min parent orient. Parent owns
  §0 spine verdict, final report, gap table, commit, and tag. Subagents write ONLY partial
  drafts under `.ai/artifacts/t144_parallel/` — never commit partials as final.

  Suggested split (launch in parallel once $ARMA path confirmed):
  | Agent | Scope | Partial output |
  |-------|-------|----------------|
  | **A — UI/Editor** | lib/UI/*, DisplayArcadeMap, missionEditor*, uiArcade, baseEditor, cursor | `t144_parallel_A_ui_editor.md` |
  | **B — Landscape/World** | landscape.*, world.cpp, object DB, roads/forests on map | `t144_parallel_B_landscape_world.md` |
  | **C — Raster/Draw** | DrawField, CStaticMap*, textures, LandGrid, satellite/chart switch | `t144_parallel_C_raster_draw.md` |
  | **D — Config** | cfg/, CfgWorlds, RscDisplay*Map*, map params | `t144_parallel_D_config.md` |
  | **E — Tools/Legacy** | VisitorExchange/, buldozer/, CfgEdit/, mapViewer, TerSynth/, extern/ | `t144_parallel_E_tools_legacy.md` |

  Each partial MUST include: files touched (path:line), symbols, rejects, open questions.
  Parent: cross-validate ≥2 agents before locking spine; merge into one coherent report.
  Delete or archive `t144_parallel/` in verify log (optional keep for audit).

═══ LOCKED ═══
  - Phase 0 discovery **before** deep architecture (§0 mandatory)
  - §0 verdict: `lib/UI/uiMap.*` primary / secondary / wrong — with evidence
  - Read-only on A3 source · artifacts only in TBD-Reforger `.ai/artifacts/`
  - **Center of gravity** = mission editor **2D map** canvas (not briefing/GPS/UAV chrome)
  - No port, no T-090 fixes, no docs/registry edits in TBD-Reforger

═══ DO ═══
  **Phase 0 — Discovery (≥6 independent search strategies; log all in §0 + optional search_log)**
  1. Display/editor classes across **entire** tree (not only lib/UI)
  2. Draw/render: DrawField, StaticMap, texture bind, LandGrid, pictureMap
  3. Data feed: Landscape, world objects, roads, forests, satellite/chart
  4. Config: CfgWorlds, RscDisplay*, map-related param classes in cfg/
  5. Tools: mapViewer, visitor, export — parallel or legacy paths?
  6. Editor guards: _ENABLE_EDITOR, retail vs VBS, dead #if 0 branches
  Validate each candidate; reject dead ends with reason; lock **spine** paragraph

  **Phase 1 — Deep trace (from spine; chase into landscape/cfg/tools as needed)**
  7. §3 Raster stack — every layer that hits pixels; switch mechanisms
  8. §4 Coordinates — world ↔ pixel math with constants
  9. §5 Zoom/LOD — thresholds, per-band visibility tables
  10. §6 World objects — DB, cull, forest polygon vs icons, road vectors
  11. §7 Editor — pick/hit-test, load sequence at editor open
  12. §8 Terrain/height on 2D vs 3D GetSurfaceY
  13. §9–§10 Gap table + T-090 program reorder (cite slice IDs)
  14. §1–§2 Executive summary + mermaid (write last)
  15. §11 File index — **top 40** files if tree-wide exploration; min 30

  **Deliverables**
  16. `.ai/artifacts/t144_arma3_map_architecture_report.md` (§0–§11)
  17. `.ai/artifacts/t144_verify_log.md` (R1–R6)
  18. Optional but encouraged if complex: `t144_arma3_map_callgraph.md`, `t144_arma3_map_data_flow.md`, `t144_arma3_search_log.md`
  19. Commit artifacts only · **T-144.1:** · tag **T-144.1**

═══ DO NOT ═══
  - Artificially confine search to lib/UI/uiMap.* only
  - Skip §0 or stop tracing when a call crosses module boundaries
  - Modify A3 source; implement T-090; edit docs/registry
  - Treat briefing/GPS/diary maps as the mission editor (log as rejects)
  - Claim Eden 3D parity without provenance caveat

═══ VERIFY ═══
  R1–R6 in verify log · optional FE build/lint sanity (note pre-existing failures)

═══ RETURN ═══
  - SHA + tag T-144.1
  - §0 spine one-liner + uiMap verdict
  - **Top 10** surprises vs T-090 (not just 5 — system is complex)
  - Modules explored beyond lib/UI (bullet list)
  - Recommended T-090 reorder with rationale
  - **Ready for Cursor doc sync.**
```
