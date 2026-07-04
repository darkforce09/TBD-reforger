# T-144.1 — Claude Code handoff (Arma 3 map architecture study)

**Slice:** T-144.1 · **Executor:** claude-code · **Branch:** `main` (TBD-Reforger)  
**Spec:** [`docs/specs/Mission_Creator_Architecture/t144_arma3_map_architecture_study.md`](../../docs/specs/Mission_Creator_Architecture/t144_arma3_map_architecture_study.md)

---

## What you are doing

**Read-only archaeology** of Bohemia’s Arma 3 **2D map canvas** — how Eden/mission editor loads terrain imagery, draws world objects, handles zoom/LOD, and maps world coordinates to pixels. Output is a **report + gap table** that tells T-090 what to build next (not a code port).

```text
Recon seed files → trace DrawField / DisplayArcadeMap → document data flow
  → compare to T-090 (SAP compose, Deck.gl, phased export)
  → write t144_arma3_map_architecture_report.md → tag T-144.1
```

---

## Two repos

| Repo | Path | Your access |
|------|------|-------------|
| **TBD-Reforger** (this) | `/home/Samuel/Projects/TBD-Reforger` | Write **`.ai/artifacts/t144_*` only** |
| **Arma 3 source** (external) | `/home/Samuel/Projects/TBD_Arma_3_Remaster/Arma_3_SourceCode_Old` | **Read only** |

---

## Start symbols (grep from these)

```bash
ARMA=/home/Samuel/Projects/TBD_Arma_3_Remaster/Arma_3_SourceCode_Old

# Core map UI
rg -n "class CStaticMap|DrawField|DisplayArcadeMap|CStaticMapArcadeViewer" \
  "$ARMA/lib/UI/uiMap.hpp" "$ARMA/lib/UI/uiMap.cpp" "$ARMA/lib/UI/uiMapExt.cpp"

# Satellite vs chart
rg -n "satellite|user chart|UserChart|MapChart" "$ARMA/lib/UI/uiMap.cpp"

# Editor shell
rg -n "DisplayMapEditor|EditorObject|EditorMode" "$ARMA/lib/UI/uiMapExt.cpp" "$ARMA/lib/UI/missionEditor.cpp"

# World / terrain hooks
rg -n "DisplayMap|GetMap\(\)|Landscape" "$ARMA/lib/world.cpp" | head -40
```

---

## Investigation checklist

Answer each with **file:line + mechanism**:

1. **What pixels fill the map background?** (satellite texture? procedural? multi-layer?)  
2. **Is there a separate “map view” vs “satellite view”?** How switched?  
3. **Where do roads/buildings/trees come from on the 2D map?** (pre-baked raster vs vector draw vs icons)  
4. **Zoom:** discrete levels or continuous? What simplifies at low zoom?  
5. **Pick / hit-test:** how does editor know what object was clicked on the map?  
6. **Height / Z:** does 2D map sample terrain DEM or only use object pivots?  
7. **Mission file vs world database:** what is loaded when editor opens a map?  
8. **External tools:** `mapViewer.exe` pipe in `gameStateExt.cpp` — still relevant?

---

## T-090 context (compare, don’t re-implement)

| T-090 today | Question for report |
|-------------|---------------------|
| SAP ortho + offline water/road compose | Does A3 bake roads into raster or draw vectors live? |
| `build-landcover-mask.mjs` SAP heuristic | Does A3 use engine land-cover or photo classification? |
| `make map-export` (T-090.3 planned) | What does A3 load from disk/world at editor boot? |
| Deck.gl IconLayer + supercluster (T-065) | How does A3 cluster/limit icon draw count? |
| rbush pick (T-063) | A3 editor pick path on 2D map |

---

## Deliverables

| File | Purpose |
|------|---------|
| `.ai/artifacts/t144_arma3_map_architecture_report.md` | Main 11-section report |
| `.ai/artifacts/t144_verify_log.md` | R1–R5 self-check |
| Optional `.ai/artifacts/t144_arma3_map_callgraph.md` | Deep call graph if report too long |

**Commit in TBD-Reforger only** — artifacts + verify log. Prefix `T-144.1:` · tag `T-144.1`.

---

## After ship

Cursor doc sync merges recommendations into T-090 hub and unblocks map implementation slices.
