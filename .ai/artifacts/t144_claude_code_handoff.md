# T-144.1 — Claude Code handoff (Arma 3 map architecture study)

**Slice:** T-144.1 · **Executor:** claude-code · **Branch:** `main` (TBD-Reforger)  
**Spec:** [`docs/specs/Mission_Creator_Architecture/t144_arma3_map_architecture_study.md`](../../docs/specs/Mission_Creator_Architecture/t144_arma3_map_architecture_study.md)

---

## What you are doing

**Read-only archaeology** of Bohemia’s Arma 3 **2D mission-editor map** — discover where it really lives in source, then document data flow, layering, zoom/LOD, and coords. **Do not assume** `uiMap.hpp` is correct until you prove it.

```text
Parent: orient → (optional) parallel read-only subagents → synthesize spine
  → deep trace + T-090 compare → report §0–§11 → tag T-144.1
```

---

## Parallel agents (operator-approved)

This slice is **long**. Parallel subagents are **encouraged** — not forbidden.

| Role | Responsibility |
|------|----------------|
| **Parent** | Orient, spawn agents, lock §0 spine, merge partials, write final report + verify log, commit + tag |
| **Subagents A–E** | Read-only deep dive on one slice of `$ARMA`; write **partial** markdown only |

**Rules:**
- Subagents: **read-only** on A3 source; write only to `.ai/artifacts/t144_parallel/*.md`
- Parent: **only** agent that commits `t144_arma3_map_architecture_report.md` and tags **T-144.1**
- No subagent edits docs/, apps/, registry
- Parent must **cross-validate** spine from ≥2 partials before §0 is final

### Subagent briefs (copy to each parallel task)

**Shared header for all subagents:**
```text
T-144.1 partial — read-only. ARMA=/home/Samuel/Projects/TBD_Arma_3_Remaster/Arma_3_SourceCode_Old
Write ONLY your assigned partial file. Cite path:line + symbols. Log rejects. No TBD-Reforger code/docs edits.
Arcade 2012 source — not Eden 3D. Retail vs VBS guards matter.
```

| ID | Prompt focus | Output file |
|----|--------------|-------------|
| **A** | UI/editor: DisplayArcadeMap, DisplayMapEditor, missionEditor*, uiMap*, uiArcade, missionEditorCursor, dispMissionEditor — who opens the 2D map? | `t144_parallel_A_ui_editor.md` |
| **B** | Landscape/world: Landscape, LandObject, world objects on map, roads, forests, cull — how does data reach the map widget? | `t144_parallel_B_landscape_world.md` |
| **C** | Raster/draw: DrawField, CStaticMap*, texture bind, LandGrid, pictureMap, satellite vs chart switch | `t144_parallel_C_raster_draw.md` |
| **D** | Config: cfg/, CfgWorlds, RscDisplay*Map*, map-related ParamEntry — what config drives the map? | `t144_parallel_D_config.md` |
| **E** | Tools/legacy: VisitorExchange/, buldozer/, CfgEdit/, mapViewer pipe, TerSynth/, binarize/ — parallel or dead paths? | `t144_parallel_E_tools_legacy.md` |

Parent merges partials → validates uiMap.* hypothesis → writes §0–§11.

---

## Two repos

| Repo | Path | Your access |
|------|------|-------------|
| **TBD-Reforger** (this) | `/home/Samuel/Projects/TBD-Reforger` | Write **`.ai/artifacts/t144_*` only** |
| **Arma 3 source** (external) | `/home/Samuel/Projects/TBD_Arma_3_Remaster/Arma_3_SourceCode_Old` | **Read only — entire tree** |

**Scope:** You are **not** confined to `lib/UI/`. Follow the map pipeline wherever it goes: `landscape.*`, `world.cpp`, `cfg/`, texture loaders, visitor/tools, `extern/`. Operator expects **exhaustive** archaeology.

---

## Phase 0 — Discovery (do this first)

**Goal:** Answer “where is the Eden / mission editor **2D map** implemented?” with evidence — not filename guessing.

### Step 1 — Orient

```bash
ARMA=/home/Samuel/Projects/TBD_Arma_3_Remaster/Arma_3_SourceCode_Old
ls "$ARMA"
find "$ARMA/lib" -maxdepth 2 -type d | head -40
```

### Step 2 — Broad search (entire tree — behavior, not paths)

`$ARMA` = full source root. Run queries under **`$ARMA`** (not only `$ARMA/lib`):

```bash
ARMA=/home/Samuel/Projects/TBD_Arma_3_Remaster/Arma_3_SourceCode_Old

# 1. Editor + 2D map displays
rg -l "DisplayArcadeMap|DisplayMapEditor|MissionEditor|ArcadeMap" "$ARMA" --glob '*.{cpp,hpp,h,sqf}' 2>/dev/null | head -40

# 2. Map widget / draw
rg -l "CStaticMap|DrawField|DrawExt|StaticMap" "$ARMA" --glob '*.{cpp,hpp,h}' | head -40

# 3. Raster / chart / satellite
rg -l "satellite|UserChart|MapChart|LandGrid|pictureMap|LandTexture" "$ARMA" --glob '*.{cpp,hpp,h}' | head -40

# 4. Landscape + world object feed (often NOT in UI/)
rg -l "Landscape|LandObject|DrawMapObjects|mapObject" "$ARMA/lib" --glob '*.{cpp,hpp,h}' | head -40

# 5. Config-driven map (cfg/ + stringtable)
rg -l "RscDisplayArcadeMap|RscMap|class Map" "$ARMA" 2>/dev/null | head -30

# 6. Tools / legacy
rg -l "mapViewer|VisitorExchange|ExportMap" "$ARMA" --glob '*.{cpp,hpp,h}' | head -20
```

**When a symbol crosses modules**, read the callee — do not stop at UI boundary.

### Step 3 — Validate each candidate

For every file that looks like “the map”:

| Question | Pass = keep tracing |
|----------|---------------------|
| Is this **mission editor** 2D, not in-game GPS/briefing/UAV? | `#if _ENABLE_EDITOR`, `DisplayArcadeMap`, editor modes |
| Does it **draw the terrain backdrop** or only icons/overlays? | Find `DrawField`, texture bind, landscape sample |
| Is it compiled in retail path or `#if _VBS3` / dead code? | Check guards |

**Reject** and log: briefing maps, diary maps, artillery/UAV terminals, pure 3D `ui3DEditor` (mention separately).

### Step 4 — Hypothesis check: `uiMap.hpp`

Cursor guessed `lib/UI/uiMap.*`. Your job:

- **Confirm** — show call chain from editor open → this file → pixel draw  
- **Or debunk** — “uiMap is in-game map only; editor uses X instead”  
- **Or split** — shared base in uiMap, editor specialization elsewhere  

This verdict is **§0 row 1** in the report.

### Step 5 — Lock the spine

One paragraph: “The authoritative chain for **mission editor 2D map** is …” with 5–10 symbols. **All later sections trace from this only.**

---

## Investigation checklist (after spine locked)

Answer each with **file:line + mechanism**:

1. **What pixels fill the map background?**  
2. **Separate “map view” vs “satellite view”?** How switched?  
3. **Roads/buildings/trees** — baked raster vs vector vs icons?  
4. **Zoom / LOD** — continuous or stepped?  
5. **Pick / hit-test** on 2D editor map  
6. **Height / Z** — DEM on 2D map or pivot only?  
7. **What loads when editor opens** — mission file vs world database  
8. **Legacy tools** — e.g. external mapViewer pipe — still used?

---

## T-090 context (compare, don’t re-implement)

| T-090 today | Question for report |
|-------------|---------------------|
| SAP ortho + offline compose | A3: bake roads into raster or draw vectors live? |
| SAP land-cover heuristic | A3: engine land-cover or photo? |
| T-090.3 export (planned) | A3: what loads at editor boot? |
| Deck.gl cluster (T-065) | A3: icon LOD / cull? |
| rbush pick (T-063) | A3: 2D editor pick path? |

---

## Deliverables

| File | Purpose |
|------|---------|
| `.ai/artifacts/t144_arma3_map_architecture_report.md` | §0 discovery + §1–§11 architecture |
| `.ai/artifacts/t144_verify_log.md` | R1–R6 |
| Optional `.ai/artifacts/t144_arma3_map_callgraph.md` | Deep call graph |

**Commit in TBD-Reforger only** — artifacts + verify log. Prefix `T-144.1:` · tag `T-144.1`.

---

## After ship

Cursor doc sync merges recommendations into T-090 hub and unblocks map implementation slices.
