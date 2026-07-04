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

## Seed files (start here — not exhaustive)

| Path (under `Arma_3_SourceCode_Old/`) | Role |
|---------------------------------------|------|
| `lib/UI/uiMap.hpp` | `CStaticMap`, `CStaticMapMain`, `DisplayArcadeMap`, `DisplayMapEditor`, draw API |
| `lib/UI/uiMap.cpp` | `DrawField`, satellite vs user chart, map texture bind |
| `lib/UI/uiMapExt.cpp` | Editor map UI, modes, object move/rotate |
| `lib/UI/uiMapExport.cpp` | Map-related export hooks |
| `lib/UI/missionEditor.*`, `baseEditor.*` | Editor shell around map |
| `lib/editor.hpp` / `editor.cpp` | Landscape / visitor / bulldozer hooks |
| `lib/world.cpp` | `DisplayMap`, terrain, map singleton |
| `lib/gameStateExt.cpp` | External `mapViewer.exe` pipe (BIS internal tool — note if dead) |

Trace **call chains** from `CStaticMap::DrawField` / `DisplayArcadeMap::OnDraw` outward.

---

## Deliverable

**Primary:** [`.ai/artifacts/t144_arma3_map_architecture_report.md`](../../../.ai/artifacts/t144_arma3_map_architecture_report.md)

Required sections (headings mandatory):

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
| **R1** | Report exists with all 11 sections | Yes |
| **R2** | ≥15 primary source files cited with path + symbol | Yes |
| **R3** | Gap table references live T-090 slices by id | Yes |
| **R4** | No edits under `Arma_3_SourceCode_Old/` | Yes |
| **R5** | No edits to `apps/`, `packages/`, `docs/` in TBD-Reforger (Cursor sync after) | Yes |

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
    ls /home/Samuel/Projects/TBD_Arma_3_Remaster/Arma_3_SourceCode_Old/lib/UI/uiMap.hpp

═══ READ (in order) ═══
  1. .ai/artifacts/t144_claude_code_handoff.md
  2. docs/specs/Mission_Creator_Architecture/t144_arma3_map_architecture_study.md
  3. docs/specs/Mission_Creator_Architecture/t090_091_map_terrain_program.md (what we built — compare only)
  4. docs/specs/Mission_Creator_Architecture/t090_eden_map_reference.md

═══ PROBLEM ═══
  T-090 map work (SAP ortho heuristics, offline compose, Deck.gl tiles) may not match
  how Arma 3 Eden actually feeds and draws the 2D mission editor map. Operator has A3
  engine source — produce a ground-truth architecture report before more T-090 code.

═══ LOCKED ═══
  - **Read-only** on `/home/Samuel/Projects/TBD_Arma_3_Remaster/Arma_3_SourceCode_Old/**`
  - **No port** — analysis + recommendations only
  - Focus **2D map** (DisplayArcadeMap / CStaticMap / DrawField) — not full 3D editor
  - Deliverable in **TBD-Reforger** `.ai/artifacts/` only
  - Do **not** edit docs/**, registry, CLAUDE.md, or application code in TBD-Reforger
  - Cite file paths + class/function names; quote short snippets where decisive

═══ DO ═══
  1. Recon — ripgrep + read seed files in handoff; expand to top ~30 files
  2. Trace raster path: satellite vs map chart vs grid — config + load + draw
  3. Trace editor map: DisplayArcadeMap → CStaticMapArcadeViewer → draw + input
  4. Trace world objects on map: icons, layers, LOD, clustering (if any)
  5. Document coord transform + zoom behavior with concrete formulas/constants
  6. Write `.ai/artifacts/t144_arma3_map_architecture_report.md` (11 sections per spec)
  7. Write `.ai/artifacts/t144_verify_log.md` (R1–R5)
  8. Commit **only** artifact files in TBD-Reforger; prefix **T-144.1:** · tag **T-144.1**

═══ DO NOT ═══
  - Modify Arma 3 source tree
  - Implement T-090 changes or “quick fixes” in the website
  - Edit generated docs or ticket registry (Cursor doc sync follows)
  - Spend time on briefing/GPS/radio UI chrome unrelated to map canvas

═══ VERIFY ═══
  Self-check R1–R5 in t144_verify_log.md (no make targets for this slice)

═══ RETURN ═══
  - Commit SHA + tag T-144.1
  - Report path + top 5 surprises vs T-090
  - Recommended T-090 program reorder (bullets)
  - **Ready for Cursor doc sync.**
```
