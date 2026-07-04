# T-144.1 — Phase 0 search log (Arma 3 source discovery)

**Tree:** `/home/Samuel/Projects/TBD_Arma_3_Remaster/Arma_3_SourceCode_Old` (read-only, 638 MB, 36 top-level dirs, `Arma3_2012.sln` era)
**Method:** ≥6 independent `rg` strategies over the **entire** tree (not confined to `lib/UI/`), hit counts recorded, every candidate validated by (a) who instantiates it, (b) mission-editor-2D vs other surface, (c) compile guard in the retail config chain. Date: 2026-07-04.

---

## S1 — Display/editor class names (behavior-neutral, tree-wide)

| Query | Top hits (file: count) |
|---|---|
| `DisplayArcadeMap` | `lib/UI/uiMapExt.cpp: 55` · `lib/UI/uiMap.hpp: 7` · `lib/UI/optionsUI.cpp: 6` · `lib/UI/displayUI.cpp: 3` · `lib/UI/uiArcade.cpp: 1` |
| `CStaticMapArcadeViewer\|StaticMapArcade` | `lib/UI/uiMapExt.cpp: 37` · `lib/UI/uiMap.hpp: 6` |
| `class Display\w*Map\|DisplayMainMap` | `lib/UI/uiMap.cpp: 45` · `lib/UI/uiMap.hpp: 14` · `lib/UI/uiMapExt.cpp: 2` · `lib/UI/displayUI.hpp: 4` · `lib/main.cpp: 1` · `lib/gameStateExt.cpp: 1` |

**Read:** editor display + editor widget implementations concentrate in `uiMapExt.cpp` (file header line 1: `// Implementation of mission editor`); shared widget family declared in `uiMap.hpp`.

## S2 — Draw/render machinery

| Query | Top hits |
|---|---|
| `CStaticMap\b` | `lib/UI/uiMap.cpp: 99` · `lib/UI/dispMissionEditorVBS.cpp: 26` · `lib/gameStateExt.cpp: 20` · `lib/UI/dispMissionEditor.cpp: 17` · `lib/UI/uiMapExt.cpp: 12` · `lib/HLA/VBSVisuals.hpp: 5` (VBS) |
| `DrawField` | `lib/UI/uiMap.cpp: 3` · `lib/UI/uiMap.hpp: 1` — **only** in the widget core |
| `LandGrid\|pictureMap` | `lib/world.cpp: 31` · `lib/AI/pathPlanner.cpp: 38` · `lib/AI/operMap.cpp: 30` · `lib/vehicleAI.cpp: 15` (AI grids — different consumers of the same landscape) |

**Read:** the terrain-pixels path (`DrawField`) exists exactly once, in `CStaticMap` (`uiMap.cpp`). AI files use land grids for pathing, not UI.

## S3 — Data feed (satellite / chart / landscape / roads / forests)

| Query | Top hits |
|---|---|
| `satellite\|MapChart\|UserChart` (lib) | `lib/landscape.cpp: 50` · `lib/landscape.hpp: 25` · `lib/UI/uiMap.cpp: 15` · shader sources (d3d9/d3d11 `PS.h`) · `lib/gameStateExt.cpp: 10` · `lib/vbsCmds.cpp: 9` |
| `class Landscape` | `lib/landscape.{hpp,cpp}`, `lib/world.hpp`, `lib/landSave.cpp`, `lib/landClutter.cpp` |
| `forest` in `lib/UI` | `lib/UI/uiMap.cpp: 81` · `lib/UI/uiMapExport.cpp: 57` · `lib/UI/uiMap.hpp: 17` |

**Read:** satellite lives in `Landscape` (terrain materials); the map widget consumes it. `uiMapExport.cpp` is a second, GDI-based consumer (offline export tool).

## S4 — Config side

| Query | Result |
|---|---|
| `RscDisplayArcadeMap` | only `lib/UI/optionsUI.cpp` (display class name lookup). **No Rsc config classes in this tree** — `cfg/` holds project files (`cfg2012.vcxproj`, `cfgBuldozer.hpp`, `rscChatListCurator.hpp`); the actual RscDisplayArcadeMap / map-control param classes ship in game data (pbo), not in the engine repo |
| `CfgWorlds` in lib | `lib/world.cpp: 22`, `lib/arcadeTemplate.cpp`, `lib/geography.cpp`, `lib/UI/uiMapExt.cpp: 2`, `lib/landscape.cpp: 2` — engine reads `CfgWorlds >> worldName >> Names/Grid/…` at runtime |

## S5 — Tools / legacy parallel paths

| Query | Result |
|---|---|
| `mapViewer` | `lib/gameStateExt.cpp` only — lines 25344–25379: spawns external `c:\bis\mapViewer\mapViewer.exe -stdin` and feeds it via stdin pipe. **Legacy internal debug tool**, not the shipped map |
| `ExportWMF` | impl `lib/UI/uiMapExport.cpp:1178` (fwd-declared `uiMap.cpp:3580`); callers: `CStaticMap::ProcessCheats` (`uiMap.cpp:3588` — `CheatExportMap` → `C:\<world>.emf`) and `lib/vbsCmds.cpp:8505` (VBS script command). **Diagnostic/offline export**, not runtime draw |
| `VisitorExchange/`, `CfgEdit/`, `buldozer/` | terrain-tooling side (Visitor object IDs surface in `MapObject::_id` as `VisitorObjId`, `lib/mapObject.hpp:23`) — build-time provenance, no runtime map draw |

## S6 — Compile guards (retail vs VBS vs dead)

| File | Guards found | Verdict |
|---|---|---|
| `lib/UI/dispMissionEditor.cpp` | whole file `#if _ENABLE_EDITOR2 && !_VBS2` (lines 3, 12029) | Editor2 ("real-time editor") — **compiled in retail** (`retailConfig.h:39` `_ENABLE_EDITOR2 1`, same in `normalConfig.h`) but a separate hidden surface; **`afxConfig.h:35` sets 0** |
| `lib/UI/dispMissionEditorVBS.cpp` | VBS variant | reject (VBS) |
| `lib/UI/uiMapExt.cpp` | 12 guard hits — `#if _ENABLE_EDITOR` sections + VBS islands | retail path live (`retailConfig.h:8` `_ENABLE_EDITOR 1`) |
| `lib/UI/uiMap.cpp` | 41 guard hits — `_VBS3` / `_VBS3_UDEFCHART` islands (user chart), `_ENABLE_CHEATS` | core retail; user-chart branch VBS3-only (`CStaticMap::UserMapDrawing` returns `false` unless `_VBS3_UDEFCHART`, `uiMap.cpp:3755`) |
| `lib/UI/uiCurator.cpp` | `#if _ARMA3_CURATOR` (line 6) | early Zeus — separate surface |
| Config chain | `lib/wpch.hpp:18-42` selects `retailConfig.h` (`_SUPER_RELEASE`) / `normalConfig.h` (default) / `VBS2Config.h` / `afxConfig.h` (`_USE_AFX`) … | retail flags: editor **and** editor2 both on |

## Entry-point cross-validation (2 independent paths)

1. **UI creation path:** `displayUI.cpp:17765` `CDPCreateEditor()` (guarded `#if _ENABLE_EDITOR`) → `parent->CreateChild(new DisplayArcadeMap(parent, multiplayer, setDirectory))` (`displayUI.cpp:17768`); called from main-menu/options handlers `optionsUI.cpp:11103/11174/11211`, `displayUI.cpp:17845/18118/36803`.
2. **Class-hierarchy path:** `uiMap.hpp:3202` `class DisplayArcadeMap : public DisplayMapEditor, public MissionEditCursorContainer` owns `CStaticMapArcade *_map` (`uiMap.hpp:3206`); `uiMap.hpp:3051` `CStaticMapArcade : CStaticMapArcadeViewer`; `uiMap.hpp:2983` `CStaticMapArcadeViewer : CStaticMap`; `uiMap.hpp:440` `CStaticMap : CStatic` with all terrain draw (`uiMap.cpp:1275` `DrawBackground`, `:2005` `DrawField`, …).

Editor2 cross-check: `world.cpp:17041` `CreateMissionEditorRealTime(NULL)` replaces the in-game map **only inside `#if _VBS2 && !_VBS2_LITE`** (`world.cpp:17021`) — VBS behavior, not retail A3.

## Dead ends logged (with reasons)

| Candidate | Why searched | Reject reason |
|---|---|---|
| `lib/UI/dispMissionEditor.cpp` (`DisplayMissionEditor : DisplayMap`, line 467) | "mission editor" name; spec hypothesis | Editor2/RTE — separate hidden editor (A2 lineage); not the shipped 2D arcade editor UI; secondary surface |
| `mapViewer.exe` stdin pipe (`gameStateExt.cpp:25344`) | spec hypothesis "legacy tool pipe" | external BI debug viewer, dev-only path |
| `lib/UI/uiMapExport.cpp` | forest/road hit density | offline GDI EMF/bitmap export (cheat/diag + VBS cmd), not runtime raster |
| `lib/HLA/ShapeLayers.*`, `VBSVisuals.*` | `CStaticMap` refs | VBS/HLA only |
| VBS3 user chart (`_VBS3_UDEFCHART` in `DrawField`, `uiMap.cpp:2052`) | "user chart" hypothesis | compiled out in retail |
| `cfg/` directory | config classes hypothesis | project files only; Rsc classes live in game data |
| Briefing/GPS/UAV/diary surfaces | scope exclusion check | separate displays over the same `CStaticMap` family — see report §0 rejects table |
